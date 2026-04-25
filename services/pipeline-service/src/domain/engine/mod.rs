use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{AppState, models::pipeline::PipelineNode};

pub mod dag_executor;
mod runtime;

#[derive(Clone)]
pub struct ExecutionEnvironment {
    pub state: AppState,
    pub actor_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub start_from_node: Option<String>,
    pub max_attempts: u32,
    pub distributed_worker_count: usize,
    pub skip_unchanged: bool,
    pub prior_node_results: HashMap<String, NodeResult>,
}

impl Default for ExecutionRequest {
    fn default() -> Self {
        Self {
            start_from_node: None,
            max_attempts: 1,
            distributed_worker_count: 1,
            skip_unchanged: true,
            prior_node_results: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeResult {
    pub node_id: String,
    pub label: String,
    pub transform_type: String,
    pub status: String,
    pub rows_affected: Option<u64>,
    pub attempts: u32,
    pub output: Option<Value>,
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_index: Option<usize>,
}

/// Execute a pipeline by running nodes in topological order.
pub async fn execute_pipeline(
    env: &ExecutionEnvironment,
    nodes: &[PipelineNode],
    request: &ExecutionRequest,
) -> Result<Vec<NodeResult>, String> {
    if request.distributed_worker_count > 1 {
        return dag_executor::execute_pipeline(env, nodes, request).await;
    }

    let order = execution_order(nodes, request.start_from_node.as_deref())?;
    let mut results = Vec::new();
    let mut dependency_fingerprints = HashMap::new();
    let node_lookup: HashMap<&str, &PipelineNode> =
        nodes.iter().map(|node| (node.id.as_str(), node)).collect();
    let max_attempts = request.max_attempts.max(1);

    for node_id in order {
        let node = node_lookup
            .get(node_id.as_str())
            .copied()
            .ok_or_else(|| format!("pipeline node '{}' not found", node_id))?;

        let mut final_result = None;
        for attempt in 1..=max_attempts {
            let mut result = execute_node(
                env,
                node,
                &dependency_fingerprints,
                request.skip_unchanged,
                request.prior_node_results.get(&node.id),
            )
            .await;
            result.attempts = attempt;
            let is_terminal = matches!(result.status.as_str(), "completed" | "skipped")
                || attempt == max_attempts;
            final_result = Some(result);
            if is_terminal {
                break;
            }
        }

        let result = final_result.expect("pipeline execution should always produce a result");
        if let Some(fingerprint) = runtime::fingerprint_from_metadata(result.metadata.as_ref()) {
            dependency_fingerprints.insert(node.id.clone(), fingerprint);
        }
        let failed = result.status == "failed";
        results.push(result);
        if failed {
            break;
        }
    }

    Ok(results)
}

pub(crate) async fn execute_node(
    env: &ExecutionEnvironment,
    node: &PipelineNode,
    dependency_fingerprints: &HashMap<String, String>,
    skip_unchanged: bool,
    prior_node_result: Option<&NodeResult>,
) -> NodeResult {
    let inputs = match runtime::load_node_inputs(&env.state, env.actor_id, node).await {
        Ok(inputs) => inputs,
        Err(error) => {
            return failed_result(node, Some(error));
        }
    };
    let fingerprint = runtime::node_fingerprint(node, &inputs, dependency_fingerprints);
    if skip_unchanged {
        if let Some(previous) = prior_node_result {
            if runtime::fingerprint_from_metadata(previous.metadata.as_ref()).as_deref()
                == Some(fingerprint.as_str())
            {
                return NodeResult {
                    node_id: node.id.clone(),
                    label: node.label.clone(),
                    transform_type: node.transform_type.clone(),
                    status: "skipped".into(),
                    rows_affected: previous.rows_affected,
                    attempts: 1,
                    output: previous.output.clone().or_else(|| {
                        Some(json!({
                            "message": "node skipped because inputs did not change",
                        }))
                    }),
                    error: None,
                    metadata: Some(runtime::build_metadata(
                        fingerprint,
                        true,
                        &inputs,
                        node.output_dataset_id,
                        runtime::output_dataset_version_from_metadata(previous.metadata.as_ref()),
                    )),
                    worker_id: None,
                    stage_index: None,
                };
            }
        }
    }

    match node.transform_type.as_str() {
        "sql" => {
            match runtime::execute_sql_transform(&env.state, env.actor_id, node, &inputs).await {
                Ok(result) => success_result(
                    node,
                    result.rows_affected,
                    result.output,
                    runtime::build_metadata(
                        fingerprint,
                        false,
                        &inputs,
                        node.output_dataset_id,
                        result.output_dataset_version,
                    ),
                ),
                Err(error) => failed_result(node, Some(error)),
            }
        }
        "python" => {
            match runtime::execute_python_transform(&env.state, env.actor_id, node, &inputs).await {
                Ok(result) => success_result(
                    node,
                    result.rows_affected,
                    result.output,
                    runtime::build_metadata(
                        fingerprint,
                        false,
                        &inputs,
                        node.output_dataset_id,
                        result.output_dataset_version,
                    ),
                ),
                Err(error) => failed_result(node, Some(error)),
            }
        }
        "wasm" => match runtime::execute_wasm_transform(node) {
            Ok((rows_affected, output)) => success_result(
                node,
                rows_affected,
                output,
                runtime::build_metadata(fingerprint, false, &inputs, node.output_dataset_id, None),
            ),
            Err(error) => failed_result(node, Some(error)),
        },
        "passthrough" => {
            match runtime::execute_passthrough_transform(&env.state, env.actor_id, node, &inputs)
                .await
            {
                Ok((rows_affected, output, output_dataset_version)) => success_result(
                    node,
                    rows_affected,
                    output,
                    runtime::build_metadata(
                        fingerprint,
                        false,
                        &inputs,
                        node.output_dataset_id,
                        output_dataset_version,
                    ),
                    ),
                Err(error) => failed_result(node, Some(error)),
            }
        }
        "spark" | "pyspark" => {
            match runtime::execute_remote_compute_transform(
                &env.state,
                env.actor_id,
                node,
                &inputs,
                "spark",
            )
            .await
            {
                Ok(result) => success_result(
                    node,
                    result.rows_affected,
                    result.output,
                    runtime::build_metadata(
                        fingerprint,
                        false,
                        &inputs,
                        node.output_dataset_id,
                        result.output_dataset_version,
                    ),
                ),
                Err(error) => failed_result(node, Some(error)),
            }
        }
        "external" | "remote" => {
            match runtime::execute_remote_compute_transform(
                &env.state,
                env.actor_id,
                node,
                &inputs,
                "external",
            )
            .await
            {
                Ok(result) => success_result(
                    node,
                    result.rows_affected,
                    result.output,
                    runtime::build_metadata(
                        fingerprint,
                        false,
                        &inputs,
                        node.output_dataset_id,
                        result.output_dataset_version,
                    ),
                ),
                Err(error) => failed_result(node, Some(error)),
            }
        }
        other => failed_result(node, Some(format!("unsupported transform type: {other}"))),
    }
}

fn success_result(
    node: &PipelineNode,
    rows_affected: Option<u64>,
    output: Value,
    metadata: Value,
) -> NodeResult {
    NodeResult {
        node_id: node.id.clone(),
        label: node.label.clone(),
        transform_type: node.transform_type.clone(),
        status: "completed".into(),
        rows_affected,
        attempts: 1,
        output: Some(output),
        error: None,
        metadata: Some(metadata),
        worker_id: None,
        stage_index: None,
    }
}

fn failed_result(node: &PipelineNode, error: Option<String>) -> NodeResult {
    NodeResult {
        node_id: node.id.clone(),
        label: node.label.clone(),
        transform_type: node.transform_type.clone(),
        status: "failed".into(),
        rows_affected: None,
        attempts: 1,
        output: None,
        error,
        metadata: None,
        worker_id: None,
        stage_index: None,
    }
}

fn execution_order(
    nodes: &[PipelineNode],
    start_from_node: Option<&str>,
) -> Result<Vec<String>, String> {
    let order = topological_sort(nodes)?;
    let Some(start_from_node) = start_from_node else {
        return Ok(order);
    };

    if !nodes.iter().any(|node| node.id == start_from_node) {
        return Err(format!("start node '{start_from_node}' not found"));
    }

    let reachable = reachable_nodes(nodes, start_from_node);
    Ok(order
        .into_iter()
        .filter(|node_id| reachable.contains(node_id))
        .collect())
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

/// Simple topological sort using Kahn's algorithm.
fn topological_sort(nodes: &[PipelineNode]) -> Result<Vec<String>, String> {
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    let mut adjacency: HashMap<&str, Vec<&str>> = HashMap::new();

    for node in nodes {
        in_degree.entry(&node.id).or_insert(0);
        adjacency.entry(&node.id).or_default();
        for dep in &node.depends_on {
            adjacency.entry(dep.as_str()).or_default().push(&node.id);
            *in_degree.entry(&node.id).or_insert(0) += 1;
        }
    }

    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|&(_, &d)| d == 0)
        .map(|(&k, _)| k)
        .collect();
    let mut order = Vec::new();

    while let Some(n) = queue.pop() {
        order.push(n.to_string());
        if let Some(neighbors) = adjacency.get(n) {
            for &neighbor in neighbors {
                if let Some(d) = in_degree.get_mut(neighbor) {
                    *d -= 1;
                    if *d == 0 {
                        queue.push(neighbor);
                    }
                }
            }
        }
    }

    if order.len() != nodes.len() {
        return Err("cycle detected in pipeline DAG".into());
    }

    Ok(order)
}
