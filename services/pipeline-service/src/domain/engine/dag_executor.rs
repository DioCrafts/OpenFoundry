use std::collections::{HashMap, HashSet};

use futures::{StreamExt, TryStreamExt, stream};
use serde_json::json;

use crate::models::pipeline::PipelineNode;

use super::{ExecutionEnvironment, ExecutionRequest, NodeResult, execute_node};

pub async fn execute_pipeline(
    env: &ExecutionEnvironment,
    nodes: &[PipelineNode],
    request: &ExecutionRequest,
) -> Result<Vec<NodeResult>, String> {
    let stages = execution_stages(nodes, request.start_from_node.as_deref())?;
    let node_lookup: HashMap<&str, &PipelineNode> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let max_attempts = request.max_attempts.max(1);
    let worker_budget = request.distributed_worker_count.max(1);
    let mut results = Vec::new();
    let mut completed_fingerprints = HashMap::new();

    for (stage_index, stage) in stages.into_iter().enumerate() {
        let node_lookup = &node_lookup;
        let stage_fingerprints = completed_fingerprints.clone();
        let mut stage_results = stream::iter(stage.into_iter().enumerate())
            .map(move |(position, node_id)| {
                let stage_fingerprints = stage_fingerprints.clone();
                async move {
                    let node = node_lookup
                        .get(node_id.as_str())
                        .copied()
                        .ok_or_else(|| format!("pipeline node '{}' not found", node_id))?;
                    let mut result = execute_node_with_retries(
                        env,
                        node,
                        &stage_fingerprints,
                        request.skip_unchanged,
                        request.prior_node_results.get(&node.id),
                        max_attempts,
                    )
                    .await;
                    result.stage_index = Some(stage_index);
                    result.worker_id = Some(format!(
                        "pipeline-worker-{}",
                        (position % worker_budget) + 1
                    ));
                    annotate_output(&mut result, stage_index, position % worker_budget);
                    Ok::<_, String>((position, result))
                }
            })
            .buffer_unordered(worker_budget)
            .try_collect::<Vec<_>>()
            .await?;

        stage_results.sort_by_key(|(position, _)| *position);
        for (_, result) in stage_results {
            let failed = result.status == "failed";
            if let Some(fingerprint) =
                super::runtime::fingerprint_from_metadata(result.metadata.as_ref())
            {
                completed_fingerprints.insert(result.node_id.clone(), fingerprint);
            }
            results.push(result);
            if failed {
                return Ok(results);
            }
        }
    }

    Ok(results)
}

async fn execute_node_with_retries(
    env: &ExecutionEnvironment,
    node: &PipelineNode,
    dependency_fingerprints: &HashMap<String, String>,
    skip_unchanged: bool,
    prior_node_result: Option<&NodeResult>,
    max_attempts: u32,
) -> NodeResult {
    let mut final_result = None;
    for attempt in 1..=max_attempts {
        let mut result = execute_node(
            env,
            node,
            dependency_fingerprints,
            skip_unchanged,
            prior_node_result,
        )
        .await;
        result.attempts = attempt;
        let is_completed = matches!(result.status.as_str(), "completed" | "skipped");
        final_result = Some(result);

        if is_completed || attempt == max_attempts {
            break;
        }
    }

    final_result.expect("pipeline execution should always produce a result")
}

fn annotate_output(result: &mut NodeResult, stage_index: usize, worker_slot: usize) {
    let execution = json!({
        "worker_id": format!("pipeline-worker-{}", worker_slot + 1),
        "stage_index": stage_index,
    });

    match result.output.take() {
        Some(serde_json::Value::Object(mut object)) => {
            object.insert("execution".to_string(), execution);
            result.output = Some(serde_json::Value::Object(object));
        }
        Some(other) => {
            result.output = Some(json!({
                "result": other,
                "execution": execution,
            }));
        }
        None => {
            result.output = Some(json!({ "execution": execution }));
        }
    }
}

fn execution_stages(
    nodes: &[PipelineNode],
    start_from_node: Option<&str>,
) -> Result<Vec<Vec<String>>, String> {
    let reachable = match start_from_node {
        Some(start_from_node) => {
            if !nodes.iter().any(|node| node.id == start_from_node) {
                return Err(format!("start node '{}' not found", start_from_node));
            }
            reachable_nodes(nodes, start_from_node)
        }
        None => nodes.iter().map(|node| node.id.clone()).collect(),
    };

    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in nodes.iter().filter(|node| reachable.contains(&node.id)) {
        in_degree.entry(node.id.as_str()).or_insert(0);
        adjacency.entry(node.id.as_str()).or_default();
        for dependency in &node.depends_on {
            if !reachable.contains(dependency) {
                continue;
            }
            adjacency
                .entry(dependency.as_str())
                .or_default()
                .push(node.id.as_str());
            *in_degree.entry(node.id.as_str()).or_insert(0) += 1;
        }
    }

    let mut frontier: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &degree)| degree == 0)
        .map(|(&node_id, _)| node_id)
        .collect();
    frontier.sort_unstable();

    let mut stages = Vec::new();
    let mut visited = 0usize;
    while !frontier.is_empty() {
        let mut next_frontier = Vec::new();
        let mut stage = Vec::new();
        for node_id in frontier.drain(..) {
            visited += 1;
            stage.push(node_id.to_string());
            if let Some(neighbors) = adjacency.get(node_id) {
                for &neighbor in neighbors {
                    if let Some(degree) = in_degree.get_mut(neighbor) {
                        *degree -= 1;
                        if *degree == 0 {
                            next_frontier.push(neighbor);
                        }
                    }
                }
            }
        }
        next_frontier.sort_unstable();
        stages.push(stage);
        frontier = next_frontier;
    }

    if visited != reachable.len() {
        return Err("cycle detected in pipeline DAG".into());
    }

    Ok(stages)
}

fn reachable_nodes(nodes: &[PipelineNode], start_from_node: &str) -> HashSet<String> {
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();
    for node in nodes {
        adjacency.entry(node.id.as_str()).or_default();
        for dependency in &node.depends_on {
            adjacency
                .entry(dependency.as_str())
                .or_default()
                .push(node.id.as_str());
        }
    }

    let mut reachable = HashSet::new();
    let mut stack = vec![start_from_node.to_string()];
    while let Some(node_id) = stack.pop() {
        if !reachable.insert(node_id.clone()) {
            continue;
        }
        if let Some(neighbors) = adjacency.get(node_id.as_str()) {
            for neighbor in neighbors {
                stack.push((*neighbor).to_string());
            }
        }
    }

    reachable
}
