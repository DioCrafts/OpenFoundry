use std::collections::{HashMap, HashSet};

use base64::Engine as _;
use pyo3::{prelude::*, types::PyDict};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use query_engine::context::QueryContext;
use wasmtime::{Config, Engine, Instance, Module, Store, Val};

use crate::models::pipeline::PipelineNode;

pub mod dag_executor;

#[derive(Debug, Clone)]
pub struct ExecutionRequest {
    pub start_from_node: Option<String>,
    pub max_attempts: u32,
    pub distributed_worker_count: usize,
}

impl Default for ExecutionRequest {
    fn default() -> Self {
        Self {
            start_from_node: None,
            max_attempts: 1,
            distributed_worker_count: 1,
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
    pub worker_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage_index: Option<usize>,
}

/// Execute a pipeline by running nodes in topological order.
pub async fn execute_pipeline(
    query_ctx: &QueryContext,
    nodes: &[PipelineNode],
    request: &ExecutionRequest,
) -> Result<Vec<NodeResult>, String> {
    if request.distributed_worker_count > 1 {
		return dag_executor::execute_pipeline(query_ctx, nodes, request).await;
	}

    let order = execution_order(nodes, request.start_from_node.as_deref())?;
    let mut results = Vec::new();
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
            let mut result = execute_node(query_ctx, node).await;
            result.attempts = attempt;
            let is_completed = result.status == "completed";
            final_result = Some(result);

            if is_completed || attempt == max_attempts {
                break;
            }
        }

        let result = final_result.expect("pipeline execution should always produce a result");
        results.push(result);

        // Stop on first failure
        if results.last().map(|r| r.status.as_str()) == Some("failed") {
            break;
        }
    }

    Ok(results)
}

pub(crate) async fn execute_node(query_ctx: &QueryContext, node: &PipelineNode) -> NodeResult {
    match node.transform_type.as_str() {
        "sql" => {
            let sql = node.config.get("sql").and_then(|v| v.as_str()).unwrap_or("");
            if sql.is_empty() {
                return NodeResult {
                    node_id: node.id.clone(),
                    label: node.label.clone(),
                    transform_type: node.transform_type.clone(),
                    status: "failed".into(),
                    rows_affected: None,
                    attempts: 1,
                    output: None,
                    error: Some("SQL transform has no 'sql' config".into()),
                    worker_id: None,
                    stage_index: None,
                };
            }
            match query_ctx.execute_sql(sql).await {
                Ok(batches) => {
                    let rows: u64 = batches.iter().map(|b| b.num_rows() as u64).sum();
                    NodeResult {
                        node_id: node.id.clone(),
                        label: node.label.clone(),
                        transform_type: node.transform_type.clone(),
                        status: "completed".into(),
                        rows_affected: Some(rows),
                        attempts: 1,
                        output: Some(json!({ "rows": rows })),
                        error: None,
                        worker_id: None,
                        stage_index: None,
                    }
                }
                Err(e) => NodeResult {
                    node_id: node.id.clone(),
                    label: node.label.clone(),
                    transform_type: node.transform_type.clone(),
                    status: "failed".into(),
                    rows_affected: None,
                    attempts: 1,
                    output: None,
                    error: Some(e.to_string()),
                    worker_id: None,
                    stage_index: None,
                },
            }
        }
        "python" => match run_python_transform(node) {
            Ok((rows_affected, output)) => NodeResult {
                node_id: node.id.clone(),
                label: node.label.clone(),
                transform_type: node.transform_type.clone(),
                status: "completed".into(),
                rows_affected,
                attempts: 1,
                output: Some(output),
                error: None,
                worker_id: None,
                stage_index: None,
            },
            Err(error) => NodeResult {
                node_id: node.id.clone(),
                label: node.label.clone(),
                transform_type: node.transform_type.clone(),
                status: "failed".into(),
                rows_affected: None,
                attempts: 1,
                output: None,
                error: Some(error),
                worker_id: None,
                stage_index: None,
            },
        },
        "wasm" => match run_wasm_transform(node) {
            Ok((rows_affected, output)) => NodeResult {
                node_id: node.id.clone(),
                label: node.label.clone(),
                transform_type: node.transform_type.clone(),
                status: "completed".into(),
                rows_affected,
                attempts: 1,
                output: Some(output),
                error: None,
                worker_id: None,
                stage_index: None,
            },
            Err(error) => NodeResult {
                node_id: node.id.clone(),
                label: node.label.clone(),
                transform_type: node.transform_type.clone(),
                status: "failed".into(),
                rows_affected: None,
                attempts: 1,
                output: None,
                error: Some(error),
                worker_id: None,
                stage_index: None,
            },
        },
        "passthrough" => NodeResult {
            node_id: node.id.clone(),
            label: node.label.clone(),
            transform_type: node.transform_type.clone(),
            status: "completed".into(),
            rows_affected: None,
            attempts: 1,
            output: Some(json!({ "message": "passthrough complete" })),
            error: None,
            worker_id: None,
            stage_index: None,
        },
        other => NodeResult {
            node_id: node.id.clone(),
            label: node.label.clone(),
            transform_type: node.transform_type.clone(),
            status: "failed".into(),
            rows_affected: None,
            attempts: 1,
            output: None,
            error: Some(format!("unsupported transform type: {other}")),
            worker_id: None,
            stage_index: None,
        },
    }
}

fn run_python_transform(node: &PipelineNode) -> Result<(Option<u64>, Value), String> {
    let source = node
        .config
        .get("source")
        .or_else(|| node.config.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if source.is_empty() {
        return Err("Python transform has no 'source' or 'code' config".into());
    }

    Python::with_gil(|py| {
        let locals = PyDict::new_bound(py);
        locals
            .set_item("config_json", node.config.to_string())
            .map_err(|error| error.to_string())?;
        locals
            .set_item(
                "input_dataset_ids",
                node.input_dataset_ids
                    .iter()
                    .map(uuid::Uuid::to_string)
                    .collect::<Vec<_>>(),
            )
            .map_err(|error| error.to_string())?;
        locals
            .set_item("output_dataset_id", node.output_dataset_id.map(|id| id.to_string()))
            .map_err(|error| error.to_string())?;

        py.run_bound(
            "import io, json, sys\nconfig = json.loads(config_json)\n_buf = io.StringIO()\n_real_stdout = sys.stdout\nsys.stdout = _buf",
            None,
            Some(&locals),
        )
        .map_err(|error| error.to_string())?;

        let execution = py.run_bound(source, None, Some(&locals));
        let stdout = py
            .eval_bound("_buf.getvalue()", None, Some(&locals))
            .ok()
            .and_then(|value| value.extract::<String>().ok())
            .unwrap_or_default();
        let rows_affected = py
            .eval_bound(
                "int(rows_affected) if 'rows_affected' in locals() and rows_affected is not None else None",
                None,
                Some(&locals),
            )
            .ok()
            .and_then(|value| value.extract::<Option<u64>>().ok())
            .flatten();
        let result = py
            .eval_bound(
                "str(result) if 'result' in locals() and result is not None else None",
                None,
                Some(&locals),
            )
            .ok()
            .and_then(|value| value.extract::<Option<String>>().ok())
            .flatten();

        let _ = py.run_bound("sys.stdout = _real_stdout", None, Some(&locals));

        match execution {
            Ok(_) => Ok((
                rows_affected,
                json!({
                    "stdout": stdout,
                    "result": result,
                }),
            )),
            Err(error) => Err(format!("{error}")),
        }
    })
}

fn run_wasm_transform(node: &PipelineNode) -> Result<(Option<u64>, Value), String> {
    let module_source = node
        .config
        .get("module")
        .and_then(Value::as_str)
        .unwrap_or("");
    if module_source.is_empty() {
        return Err("WASM transform has no 'module' config".into());
    }

    let mut config = Config::new();
    config.consume_fuel(true);
    let engine = Engine::new(&config).map_err(|error| error.to_string())?;

    let module = if module_source.trim_start().starts_with("(module") {
        Module::new(&engine, module_source).map_err(|error| error.to_string())?
    } else if let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(module_source) {
        Module::from_binary(&engine, &bytes).map_err(|error| error.to_string())?
    } else {
        Module::new(&engine, module_source).map_err(|error| error.to_string())?
    };

    let mut store = Store::new(&engine, ());
    store
        .set_fuel(10_000_000)
        .map_err(|error| error.to_string())?;

    let instance = Instance::new(&mut store, &module, &[]).map_err(|error| error.to_string())?;
    let function_name = node
        .config
        .get("function")
        .and_then(Value::as_str)
        .unwrap_or("run");
    let function = instance
        .get_func(&mut store, function_name)
        .ok_or_else(|| format!("WASM export '{function_name}' not found"))?;
    let function_type = function.ty(&store);
    if function_type.params().len() > 0 {
        return Err("WASM transform functions with parameters are not supported".into());
    }

    let mut results = vec![Val::I32(0); function_type.results().len()];
    function
        .call(&mut store, &[], &mut results)
        .map_err(|error| error.to_string())?;

    let output_values = results.iter().map(wasm_val_to_json).collect::<Vec<_>>();
    let rows_affected = results.first().and_then(|value| match value {
        Val::I32(inner) => Some((*inner).max(0) as u64),
        Val::I64(inner) => Some((*inner).max(0) as u64),
        _ => None,
    });

    Ok((rows_affected, json!({ "results": output_values })))
}

fn wasm_val_to_json(value: &Val) -> Value {
    match value {
        Val::I32(inner) => json!(inner),
        Val::I64(inner) => json!(inner),
        Val::F32(inner) => json!(f32::from_bits(*inner)),
        Val::F64(inner) => json!(f64::from_bits(*inner)),
        _ => json!(format!("{value:?}")),
    }
}

fn execution_order(nodes: &[PipelineNode], start_from_node: Option<&str>) -> Result<Vec<String>, String> {
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
