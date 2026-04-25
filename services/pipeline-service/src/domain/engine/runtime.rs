use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
    str::from_utf8,
};

use auth_middleware::jwt::{build_access_claims, encode_token};
use base64::Engine as _;
use bytes::Bytes;
use datafusion::{
    arrow::{array::RecordBatch, util::display::array_value_to_string},
    prelude::NdJsonReadOptions,
};
use pyo3::{prelude::*, types::PyDict};
use query_engine::context::QueryContext;
use reqwest::{
    header::{HeaderName, HeaderValue},
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use tokio::fs;
use uuid::Uuid;
use wasmtime::{Config, Engine, Instance, Module, Store, Val};

use crate::{AppState, models::pipeline::PipelineNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetInputMetadata {
    pub dataset_id: Uuid,
    pub name: String,
    pub format: String,
    pub version: i32,
    pub row_count: i64,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeExecutionMetadata {
    pub fingerprint: String,
    pub skipped: bool,
    pub input_datasets: Vec<DatasetInputMetadata>,
    pub output_dataset_id: Option<Uuid>,
    pub output_dataset_version: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct LoadedDataset {
    pub metadata: DatasetInputMetadata,
    pub bytes: Bytes,
    pub storage_path: String,
}

#[derive(Debug)]
pub struct SqlExecutionResult {
    pub rows_affected: Option<u64>,
    pub output: Value,
    pub output_dataset_version: Option<i32>,
}

#[derive(Debug)]
pub struct PythonExecutionResult {
    pub rows_affected: Option<u64>,
    pub output: Value,
    pub output_dataset_version: Option<i32>,
}

#[derive(Debug)]
pub struct LlmExecutionResult {
    pub rows_affected: Option<u64>,
    pub output: Value,
    pub output_dataset_version: Option<i32>,
}

#[derive(Debug)]
pub struct RemoteComputeExecutionResult {
    pub rows_affected: Option<u64>,
    pub output: Value,
    pub output_dataset_version: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct RemoteDataset {
    id: Uuid,
    name: String,
    format: String,
    storage_path: String,
    size_bytes: i64,
    row_count: i64,
    current_version: i32,
}

#[derive(Debug, Serialize)]
struct PreparedInput {
    alias: String,
    metadata: DatasetInputMetadata,
    rows: Vec<Value>,
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequestPayload {
    conversation_id: Option<Uuid>,
    system_prompt: Option<String>,
    user_message: String,
    knowledge_base_id: Option<Uuid>,
    preferred_provider_id: Option<Uuid>,
    fallback_enabled: bool,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponsePayload {
    provider_id: Uuid,
    provider_name: String,
    reply: String,
    citations: Vec<Value>,
    guardrail: ChatCompletionGuardrail,
    cache: ChatCompletionCache,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionGuardrail {
    blocked: bool,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionCache {
    hit: bool,
}

#[derive(Debug, Serialize)]
struct RemoteComputeRequest {
    job_type: String,
    pipeline_node_id: String,
    pipeline_node_label: String,
    transform_type: String,
    config: Value,
    inputs: Vec<PreparedInput>,
    output_dataset_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
struct RemoteComputeResponse {
    status: Option<String>,
    rows_affected: Option<u64>,
    output: Option<Value>,
    result_rows: Option<Value>,
    run_id: Option<String>,
    worker_id: Option<String>,
}

struct PreparedQueryContext {
    ctx: QueryContext,
    paths: Vec<PathBuf>,
}

pub async fn load_node_inputs(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
) -> Result<Vec<LoadedDataset>, String> {
    let token = issue_service_token(state, actor_id)?;
    let mut inputs = Vec::new();

    for dataset_id in &node.input_dataset_ids {
        let url = format!(
            "{}/api/v1/datasets/{dataset_id}",
            state.dataset_service_url.trim_end_matches('/')
        );
        let response = state
            .http_client
            .get(url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|error| error.to_string())?;

        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            return Err(format!(
                "dataset lookup for {dataset_id} failed with HTTP {status}: {body}"
            ));
        }

        let remote =
            serde_json::from_str::<RemoteDataset>(&body).map_err(|error| error.to_string())?;
        let storage_path = format!("{}/v{}", remote.storage_path, remote.current_version);
        let bytes = state
            .storage
            .get(&storage_path)
            .await
            .map_err(|error| error.to_string())?;

        inputs.push(LoadedDataset {
            metadata: DatasetInputMetadata {
                dataset_id: remote.id,
                name: remote.name,
                format: remote.format,
                version: remote.current_version,
                row_count: remote.row_count,
                size_bytes: remote.size_bytes,
            },
            bytes,
            storage_path,
        });
    }

    Ok(inputs)
}

pub fn node_fingerprint(
    node: &PipelineNode,
    inputs: &[LoadedDataset],
    dependency_fingerprints: &HashMap<String, String>,
) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    node.id.hash(&mut hasher);
    node.label.hash(&mut hasher);
    node.transform_type.hash(&mut hasher);
    node.config.to_string().hash(&mut hasher);

    let mut input_fingerprints = inputs
        .iter()
        .map(|input| {
            (
                input.metadata.dataset_id,
                input.metadata.version,
                input.metadata.size_bytes,
                input.storage_path.as_str(),
            )
        })
        .collect::<Vec<_>>();
    input_fingerprints.sort_by_key(|(dataset_id, _, _, _)| *dataset_id);
    input_fingerprints.hash(&mut hasher);

    let mut dependencies = node
        .depends_on
        .iter()
        .map(|dependency| {
            (
                dependency.clone(),
                dependency_fingerprints
                    .get(dependency)
                    .cloned()
                    .unwrap_or_default(),
            )
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| left.0.cmp(&right.0));
    dependencies.hash(&mut hasher);

    format!("{:016x}", hasher.finish())
}

pub async fn execute_sql_transform(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<SqlExecutionResult, String> {
    let sql = node
        .config
        .get("sql")
        .or_else(|| node.config.get("query"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if sql.is_empty() {
        return Err("SQL transform has no 'sql' or 'query' config".to_string());
    }

    let prepared = prepare_query_context(node, inputs).await?;
    let batches = prepared
        .ctx
        .execute_sql(&sql)
        .await
        .map_err(|error| error.to_string());
    let result = match batches {
        Ok(batches) => {
            let rows = collect_object_rows(&batches);
            let rows_affected = rows.len() as u64;
            let output_dataset_version = match node.output_dataset_id {
                Some(dataset_id) => {
                    Some(upload_json_rows(state, actor_id, dataset_id, &node.id, &rows).await?)
                }
                None => None,
            };

            Ok(SqlExecutionResult {
                rows_affected: Some(rows_affected),
                output: json!({
                    "rows": rows_affected,
                    "columns": column_metadata(&batches),
                    "sample_rows": rows.iter().take(10).cloned().collect::<Vec<_>>(),
                }),
                output_dataset_version,
            })
        }
        Err(error) => Err(error),
    };
    cleanup_paths(prepared.paths).await;
    result
}

pub async fn execute_python_transform(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<PythonExecutionResult, String> {
    let source = node
        .config
        .get("source")
        .or_else(|| node.config.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if source.is_empty() {
        return Err("Python transform has no 'source' or 'code' config".to_string());
    }

    let prepared_inputs = prepare_python_inputs(node, inputs).await?;
    let prepared_json =
        serde_json::to_string(&prepared_inputs).map_err(|error| error.to_string())?;

    let execution = Python::with_gil(
        |py| -> Result<(Option<u64>, Value, Option<Vec<Value>>), String> {
            let locals = PyDict::new_bound(py);
            locals
                .set_item("config_json", node.config.to_string())
                .map_err(|error| error.to_string())?;
            locals
                .set_item("prepared_inputs_json", prepared_json.clone())
                .map_err(|error| error.to_string())?;
            locals
                .set_item(
                    "input_dataset_ids",
                    node.input_dataset_ids
                        .iter()
                        .map(Uuid::to_string)
                        .collect::<Vec<_>>(),
                )
                .map_err(|error| error.to_string())?;
            locals
                .set_item(
                    "output_dataset_id",
                    node.output_dataset_id.map(|id| id.to_string()),
                )
                .map_err(|error| error.to_string())?;

            py.run_bound(
            "import io, json, sys\nconfig = json.loads(config_json)\nprepared_inputs = json.loads(prepared_inputs_json)\ninput_datasets = prepared_inputs\ninput_rows = prepared_inputs[0]['rows'] if prepared_inputs else []\n_buf = io.StringIO()\n_real_stdout = sys.stdout\nsys.stdout = _buf",
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
            let result_rows_json = py
            .eval_bound(
                "json.dumps(result_rows) if 'result_rows' in locals() and result_rows is not None else None",
                None,
                Some(&locals),
            )
            .ok()
            .and_then(|value| value.extract::<Option<String>>().ok())
            .flatten();

            let _ = py.run_bound("sys.stdout = _real_stdout", None, Some(&locals));

            match execution {
                Ok(_) => {
                    let result_rows = result_rows_json
                        .map(|raw| {
                            serde_json::from_str::<Value>(&raw).map_err(|error| error.to_string())
                        })
                        .transpose()?
                        .map(normalize_result_rows)
                        .transpose()?;
                    Ok((
                        rows_affected
                            .or_else(|| result_rows.as_ref().map(|rows| rows.len() as u64)),
                        json!({
                            "stdout": stdout,
                            "result": result,
                            "sample_rows": result_rows
                                .as_ref()
                                .map(|rows| rows.iter().take(10).cloned().collect::<Vec<_>>()),
                        }),
                        result_rows,
                    ))
                }
                Err(error) => Err(format!("{error}")),
            }
        },
    )?;

    let output_dataset_version = match (node.output_dataset_id, execution.2.as_ref()) {
        (Some(dataset_id), Some(rows)) => {
            Some(upload_json_rows(state, actor_id, dataset_id, &node.id, rows).await?)
        }
        (Some(_), None) => {
            return Err(
                "Python transform with output_dataset_id must set 'result_rows' to a list of objects"
                    .to_string(),
            );
        }
        (None, _) => None,
    };

    Ok(PythonExecutionResult {
        rows_affected: execution.0,
        output: execution.1,
        output_dataset_version,
    })
}

pub async fn execute_llm_transform(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<LlmExecutionResult, String> {
    let prompt_template =
        config_string(node, &["prompt", "user_prompt", "template"]).ok_or_else(|| {
            "LLM transform has no 'prompt', 'user_prompt', or 'template' config".to_string()
        })?;
    let system_prompt = config_string(node, &["system_prompt"]).map(str::to_string);
    let input_field = config_string(node, &["input_field"]);
    let output_field = config_string(node, &["output_field"])
        .unwrap_or("llm_response")
        .to_string();
    let response_format = config_string(node, &["response_format"])
        .unwrap_or("text")
        .to_ascii_lowercase();
    let flatten_json_output = config_bool(node, &["flatten_json_output"], false);
    let preserve_input = config_bool(node, &["preserve_input"], true);
    let fallback_enabled = config_bool(node, &["fallback_enabled"], true);
    let max_rows = config_usize(node, &["max_rows"], 25).max(1);
    let max_tokens = config_i32(node, &["max_tokens"], 256).max(32);
    let temperature = config_f32(node, &["temperature"], 0.2).clamp(0.0, 2.0);
    let knowledge_base_id = config_uuid(node, &["knowledge_base_id"])?;
    let preferred_provider_id = config_uuid(node, &["preferred_provider_id"])?;

    let prepared_inputs = prepare_python_inputs(node, inputs).await?;
    let mut output_rows = Vec::new();
    let mut provider_names = Vec::new();
    let mut provider_ids = Vec::new();
    let mut blocked_count = 0usize;
    let mut cache_hits = 0usize;
    let mut citation_count = 0usize;

    if prepared_inputs.is_empty() {
        let user_message = render_llm_prompt(prompt_template, None, &json!({}), 0, input_field);
        let response = request_llm_completion(
            state,
            actor_id,
            system_prompt.clone(),
            user_message,
            knowledge_base_id,
            preferred_provider_id,
            fallback_enabled,
            temperature,
            max_tokens,
        )
        .await?;
        blocked_count += usize::from(response.guardrail.blocked);
        cache_hits += usize::from(response.cache.hit);
        citation_count += response.citations.len();
        provider_names.push(response.provider_name.clone());
        provider_ids.push(response.provider_id);
        output_rows.push(build_llm_output_row(
            None,
            &json!({}),
            0,
            &response,
            &output_field,
            &response_format,
            flatten_json_output,
            preserve_input,
        )?);
    } else {
        'outer: for prepared_input in &prepared_inputs {
            for (row_index, row) in prepared_input.rows.iter().enumerate() {
                if output_rows.len() >= max_rows {
                    break 'outer;
                }

                let user_message = render_llm_prompt(
                    prompt_template,
                    Some(prepared_input),
                    row,
                    row_index,
                    input_field,
                );
                let response = request_llm_completion(
                    state,
                    actor_id,
                    system_prompt.clone(),
                    user_message,
                    knowledge_base_id,
                    preferred_provider_id,
                    fallback_enabled,
                    temperature,
                    max_tokens,
                )
                .await?;
                blocked_count += usize::from(response.guardrail.blocked);
                cache_hits += usize::from(response.cache.hit);
                citation_count += response.citations.len();
                provider_names.push(response.provider_name.clone());
                provider_ids.push(response.provider_id);
                output_rows.push(build_llm_output_row(
                    Some(prepared_input),
                    row,
                    row_index,
                    &response,
                    &output_field,
                    &response_format,
                    flatten_json_output,
                    preserve_input,
                )?);
            }
        }
    }

    provider_names.sort();
    provider_names.dedup();
    provider_ids.sort();
    provider_ids.dedup();

    let output_dataset_version = match node.output_dataset_id {
        Some(dataset_id) => {
            Some(upload_json_rows(state, actor_id, dataset_id, &node.id, &output_rows).await?)
        }
        None => None,
    };

    Ok(LlmExecutionResult {
        rows_affected: Some(output_rows.len() as u64),
        output: json!({
            "rows": output_rows.len(),
            "output_field": output_field,
            "response_format": response_format,
            "provider_names": provider_names,
            "provider_ids": provider_ids,
            "cache_hits": cache_hits,
            "guardrail_blocked": blocked_count,
            "citations": citation_count,
            "sample_rows": output_rows.iter().take(10).cloned().collect::<Vec<_>>(),
        }),
        output_dataset_version,
    })
}

pub async fn execute_passthrough_transform(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<(Option<u64>, Value, Option<i32>), String> {
    let Some(primary_input) = inputs.first() else {
        return Ok((
            None,
            json!({ "message": "passthrough complete", "copied": false }),
            None,
        ));
    };

    let output_dataset_version = match node.output_dataset_id {
        Some(dataset_id) => Some(
            upload_dataset_bytes(
                state,
                actor_id,
                dataset_id,
                &primary_input.bytes,
                &primary_input.metadata.format,
                format!(
                    "{}.{}",
                    node.id,
                    file_extension(&primary_input.metadata.format)
                ),
            )
            .await?,
        ),
        None => None,
    };

    Ok((
        Some(primary_input.metadata.row_count.max(0) as u64),
        json!({
            "message": "passthrough complete",
            "source_dataset_id": primary_input.metadata.dataset_id,
            "source_version": primary_input.metadata.version,
        }),
        output_dataset_version,
    ))
}

pub async fn execute_remote_compute_transform(
    state: &AppState,
    actor_id: Uuid,
    node: &PipelineNode,
    inputs: &[LoadedDataset],
    default_job_type: &str,
) -> Result<RemoteComputeExecutionResult, String> {
    let prepared_inputs = prepare_python_inputs(node, inputs).await?;
    let (endpoint, request_payload) =
        build_remote_compute_request(node, prepared_inputs, default_job_type)?;

    let mut request = state.http_client.post(&endpoint).json(&request_payload);
    if node
        .config
        .get("auth_mode")
        .and_then(Value::as_str)
        .unwrap_or("none")
        == "service_jwt"
    {
        request = request.bearer_auth(issue_service_token(state, actor_id)?);
    }
    if let Some(headers) = node.config.get("headers").and_then(Value::as_object) {
        for (name, value) in headers {
            let header_value = value
                .as_str()
                .ok_or_else(|| format!("header '{name}' must be a string"))?;
            let header_name =
                HeaderName::from_bytes(name.as_bytes()).map_err(|error| error.to_string())?;
            let header_value =
                HeaderValue::from_str(header_value).map_err(|error| error.to_string())?;
            request = request.header(header_name, header_value);
        }
    }

    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "remote compute request failed with HTTP {status}: {body}"
        ));
    }

    let payload =
        serde_json::from_str::<RemoteComputeResponse>(&body).map_err(|error| error.to_string())?;
    let (rows_affected, output, result_rows) =
        prepare_remote_compute_output(payload, &endpoint, &request_payload.job_type)?;

    let output_dataset_version = match (node.output_dataset_id, result_rows.as_ref()) {
        (Some(dataset_id), Some(rows)) => {
            Some(upload_json_rows(state, actor_id, dataset_id, &node.id, rows).await?)
        }
        (Some(_), None) => {
            return Err(
                "remote compute transform with output_dataset_id must return 'result_rows'"
                    .to_string(),
            );
        }
        (None, _) => None,
    };

    Ok(RemoteComputeExecutionResult {
        rows_affected,
        output,
        output_dataset_version,
    })
}

async fn request_llm_completion(
    state: &AppState,
    actor_id: Uuid,
    system_prompt: Option<String>,
    user_message: String,
    knowledge_base_id: Option<Uuid>,
    preferred_provider_id: Option<Uuid>,
    fallback_enabled: bool,
    temperature: f32,
    max_tokens: i32,
) -> Result<ChatCompletionResponsePayload, String> {
    let token = issue_service_token(state, actor_id)?;
    let url = format!(
        "{}/api/v1/ai/chat/completions",
        state.ai_service_url.trim_end_matches('/')
    );
    let request = ChatCompletionRequestPayload {
        conversation_id: None,
        system_prompt,
        user_message,
        knowledge_base_id,
        preferred_provider_id,
        fallback_enabled,
        temperature,
        max_tokens,
    };

    let response = state
        .http_client
        .post(url)
        .bearer_auth(token)
        .json(&request)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "LLM transform request failed with HTTP {status}: {body}"
        ));
    }

    serde_json::from_str::<ChatCompletionResponsePayload>(&body).map_err(|error| error.to_string())
}

fn render_llm_prompt(
    template: &str,
    prepared_input: Option<&PreparedInput>,
    row: &Value,
    row_index: usize,
    input_field: Option<&str>,
) -> String {
    let row_json = serde_json::to_string_pretty(row).unwrap_or_else(|_| row.to_string());
    let row_count = prepared_input.map(|input| input.rows.len()).unwrap_or(1);
    let input_text = input_field
        .and_then(|field| row.get(field))
        .map(value_to_prompt_text)
        .unwrap_or_else(|| value_to_prompt_text(row));
    let replacements = vec![
        ("{{input_json}}", row_json.clone()),
        ("{{input_text}}", input_text),
        (
            "{{dataset_name}}",
            prepared_input
                .map(|input| input.metadata.name.clone())
                .unwrap_or_default(),
        ),
        (
            "{{dataset_id}}",
            prepared_input
                .map(|input| input.metadata.dataset_id.to_string())
                .unwrap_or_default(),
        ),
        (
            "{{dataset_alias}}",
            prepared_input
                .map(|input| input.alias.clone())
                .unwrap_or_default(),
        ),
        ("{{row_index}}", row_index.to_string()),
        ("{{row_count}}", row_count.to_string()),
    ];

    let mut rendered = template.to_string();
    let mut replaced = 0usize;
    for (token, value) in replacements {
        if rendered.contains(token) {
            rendered = rendered.replace(token, &value);
            replaced += 1;
        }
    }

    if rendered.contains("{{input_rows_json}}") {
        let rows_json = prepared_input
            .map(|input| serde_json::to_string(&input.rows).unwrap_or_else(|_| "[]".to_string()))
            .unwrap_or_else(|| "[]".to_string());
        rendered = rendered.replace("{{input_rows_json}}", &rows_json);
        replaced += 1;
    }

    if replaced == 0 {
        format!("{template}\n\nInput row:\n{row_json}")
    } else {
        rendered
    }
}

fn build_llm_output_row(
    prepared_input: Option<&PreparedInput>,
    row: &Value,
    row_index: usize,
    response: &ChatCompletionResponsePayload,
    output_field: &str,
    response_format: &str,
    flatten_json_output: bool,
    preserve_input: bool,
) -> Result<Value, String> {
    let mut output = if preserve_input {
        row.as_object().cloned().unwrap_or_default()
    } else {
        Map::new()
    };

    let should_parse_json = matches!(response_format, "json" | "json_object");
    if should_parse_json {
        match serde_json::from_str::<Value>(&response.reply) {
            Ok(Value::Object(object)) if flatten_json_output => {
                for (key, value) in object {
                    output.insert(key, value);
                }
            }
            Ok(value) => {
                output.insert(output_field.to_string(), value);
            }
            Err(_) => {
                output.insert(
                    output_field.to_string(),
                    Value::String(response.reply.clone()),
                );
                output.insert(format!("{output_field}_parse_error"), json!(true));
            }
        }
    } else {
        output.insert(
            output_field.to_string(),
            Value::String(response.reply.clone()),
        );
    }

    if let Some(prepared_input) = prepared_input {
        output
            .entry("_source_dataset_id".to_string())
            .or_insert_with(|| json!(prepared_input.metadata.dataset_id));
        output
            .entry("_source_dataset_name".to_string())
            .or_insert_with(|| json!(prepared_input.metadata.name.clone()));
        output
            .entry("_source_dataset_alias".to_string())
            .or_insert_with(|| json!(prepared_input.alias.clone()));
    }
    output.insert("_source_row_index".to_string(), json!(row_index));
    output.insert(
        "_llm".to_string(),
        json!({
            "provider_id": response.provider_id,
            "provider_name": response.provider_name.clone(),
            "cache_hit": response.cache.hit,
            "guardrail_blocked": response.guardrail.blocked,
            "citations": response.citations.len(),
        }),
    );

    Ok(Value::Object(output))
}

fn config_string<'a>(node: &'a PipelineNode, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        node.config
            .get(*key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    })
}

fn config_bool(node: &PipelineNode, keys: &[&str], default: bool) -> bool {
    keys.iter()
        .find_map(|key| match node.config.get(*key) {
            Some(Value::Bool(value)) => Some(*value),
            Some(Value::String(value)) => {
                let normalized = value.trim().to_ascii_lowercase();
                match normalized.as_str() {
                    "true" | "1" | "yes" | "on" => Some(true),
                    "false" | "0" | "no" | "off" => Some(false),
                    _ => None,
                }
            }
            Some(Value::Number(value)) => value.as_i64().map(|value| value != 0),
            _ => None,
        })
        .unwrap_or(default)
}

fn config_usize(node: &PipelineNode, keys: &[&str], default: usize) -> usize {
    keys.iter()
        .find_map(|key| match node.config.get(*key) {
            Some(Value::Number(value)) => value.as_u64().map(|value| value as usize),
            Some(Value::String(value)) => value.trim().parse::<usize>().ok(),
            _ => None,
        })
        .unwrap_or(default)
}

fn config_i32(node: &PipelineNode, keys: &[&str], default: i32) -> i32 {
    keys.iter()
        .find_map(|key| match node.config.get(*key) {
            Some(Value::Number(value)) => value.as_i64().map(|value| value as i32),
            Some(Value::String(value)) => value.trim().parse::<i32>().ok(),
            _ => None,
        })
        .unwrap_or(default)
}

fn config_f32(node: &PipelineNode, keys: &[&str], default: f32) -> f32 {
    keys.iter()
        .find_map(|key| match node.config.get(*key) {
            Some(Value::Number(value)) => value.as_f64().map(|value| value as f32),
            Some(Value::String(value)) => value.trim().parse::<f32>().ok(),
            _ => None,
        })
        .unwrap_or(default)
}

fn config_uuid(node: &PipelineNode, keys: &[&str]) -> Result<Option<Uuid>, String> {
    for key in keys {
        let Some(value) = node.config.get(*key) else {
            continue;
        };
        match value {
            Value::String(raw) if raw.trim().is_empty() => return Ok(None),
            Value::String(raw) => {
                return Uuid::parse_str(raw.trim())
                    .map(Some)
                    .map_err(|error| format!("invalid UUID in '{key}': {error}"));
            }
            Value::Null => return Ok(None),
            other => {
                return Err(format!(
                    "config field '{key}' must be a UUID string, got {other}"
                ));
            }
        }
    }

    Ok(None)
}

fn value_to_prompt_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_json::to_string(other).unwrap_or_else(|_| other.to_string()),
    }
}

fn build_remote_compute_request(
    node: &PipelineNode,
    prepared_inputs: Vec<PreparedInput>,
    default_job_type: &str,
) -> Result<(String, RemoteComputeRequest), String> {
    let endpoint = node
        .config
        .get("endpoint")
        .or_else(|| node.config.get("url"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{default_job_type} transform has no 'endpoint' or 'url' config"))?
        .to_string();
    let job_type = node
        .config
        .get("job_type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_job_type)
        .to_string();

    Ok((
        endpoint,
        RemoteComputeRequest {
            job_type,
            pipeline_node_id: node.id.clone(),
            pipeline_node_label: node.label.clone(),
            transform_type: node.transform_type.clone(),
            config: node.config.clone(),
            inputs: prepared_inputs,
            output_dataset_id: node.output_dataset_id,
        },
    ))
}

fn prepare_remote_compute_output(
    payload: RemoteComputeResponse,
    endpoint: &str,
    job_type: &str,
) -> Result<(Option<u64>, Value, Option<Vec<Value>>), String> {
    if let Some(remote_status) = payload.status.as_deref() {
        let normalized = remote_status.to_ascii_lowercase();
        if !matches!(normalized.as_str(), "completed" | "success" | "ok") {
            return Err(format!(
                "remote compute job reported non-success status '{remote_status}'"
            ));
        }
    }

    let result_rows = payload.result_rows.map(normalize_result_rows).transpose()?;
    let rows_affected = payload
        .rows_affected
        .or_else(|| result_rows.as_ref().map(|rows| rows.len() as u64));

    let mut output = payload.output.unwrap_or_else(|| {
        json!({
            "endpoint": endpoint,
            "job_type": job_type,
            "rows": rows_affected,
        })
    });
    if let Some(object) = output.as_object_mut() {
        if let Some(run_id) = payload.run_id {
            object
                .entry("run_id".to_string())
                .or_insert_with(|| json!(run_id));
        }
        if let Some(worker_id) = payload.worker_id {
            object
                .entry("worker_id".to_string())
                .or_insert_with(|| json!(worker_id));
        }
    }

    Ok((rows_affected, output, result_rows))
}

pub fn execute_wasm_transform(node: &PipelineNode) -> Result<(Option<u64>, Value), String> {
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

pub fn build_metadata(
    fingerprint: String,
    skipped: bool,
    inputs: &[LoadedDataset],
    output_dataset_id: Option<Uuid>,
    output_dataset_version: Option<i32>,
) -> Value {
    serde_json::to_value(NodeExecutionMetadata {
        fingerprint,
        skipped,
        input_datasets: inputs.iter().map(|input| input.metadata.clone()).collect(),
        output_dataset_id,
        output_dataset_version,
    })
    .unwrap_or_else(|_| json!({}))
}

pub fn fingerprint_from_metadata(metadata: Option<&Value>) -> Option<String> {
    metadata
        .cloned()
        .and_then(|value| serde_json::from_value::<NodeExecutionMetadata>(value).ok())
        .map(|value| value.fingerprint)
}

pub fn output_dataset_version_from_metadata(metadata: Option<&Value>) -> Option<i32> {
    metadata
        .cloned()
        .and_then(|value| serde_json::from_value::<NodeExecutionMetadata>(value).ok())
        .and_then(|value| value.output_dataset_version)
}

fn normalize_result_rows(value: Value) -> Result<Vec<Value>, String> {
    match value {
        Value::Array(rows) => Ok(rows),
        Value::Object(_) => Ok(vec![value]),
        _ => Err("result_rows must serialize to an object or array of objects".to_string()),
    }
}

async fn prepare_python_inputs(
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<Vec<PreparedInput>, String> {
    let prepared = prepare_query_context(node, inputs).await?;
    let mut result = Vec::new();
    for (index, input) in inputs.iter().enumerate() {
        let alias = preferred_alias(node, index, input);
        let rows = prepared
            .ctx
            .execute_sql(&format!("SELECT * FROM {}", quote_identifier(&alias)))
            .await
            .map_err(|error| error.to_string())
            .map(|batches| collect_object_rows(&batches))?;
        result.push(PreparedInput {
            alias,
            metadata: input.metadata.clone(),
            rows,
        });
    }
    cleanup_paths(prepared.paths).await;
    Ok(result)
}

async fn prepare_query_context(
    node: &PipelineNode,
    inputs: &[LoadedDataset],
) -> Result<PreparedQueryContext, String> {
    let ctx = QueryContext::new();
    let mut paths = Vec::new();

    for (index, input) in inputs.iter().enumerate() {
        let extension = file_extension(&input.metadata.format);
        let path = std::env::temp_dir().join(format!(
            "openfoundry-pipeline-{}-{}-{}.{}",
            node.id,
            index,
            Uuid::now_v7(),
            extension
        ));
        let bytes = if input.metadata.format == "json" {
            normalize_json_bytes(&input.bytes)?
        } else {
            input.bytes.to_vec()
        };

        fs::write(&path, bytes)
            .await
            .map_err(|error| error.to_string())?;
        let file_path = path.to_string_lossy().to_string();

        for alias in dataset_aliases(node, index, input) {
            register_dataset_alias(&ctx, &alias, &file_path, &input.metadata.format).await?;
        }
        paths.push(path);
    }

    Ok(PreparedQueryContext { ctx, paths })
}

async fn register_dataset_alias(
    ctx: &QueryContext,
    alias: &str,
    file_path: &str,
    format: &str,
) -> Result<(), String> {
    match format {
        "csv" => ctx
            .register_csv(alias, file_path)
            .await
            .map_err(|error| error.to_string()),
        "json" => ctx
            .inner()
            .register_json(alias, file_path, NdJsonReadOptions::default())
            .await
            .map_err(|error| error.to_string()),
        "parquet" => ctx
            .register_parquet(alias, file_path)
            .await
            .map_err(|error| error.to_string()),
        other => Err(format!(
            "unsupported dataset format for pipeline input: {other}"
        )),
    }
}

async fn upload_json_rows(
    state: &AppState,
    actor_id: Uuid,
    dataset_id: Uuid,
    node_id: &str,
    rows: &[Value],
) -> Result<i32, String> {
    let bytes = serde_json::to_vec(rows).map_err(|error| error.to_string())?;
    upload_dataset_bytes(
        state,
        actor_id,
        dataset_id,
        &bytes,
        "json",
        format!("{node_id}.json"),
    )
    .await
}

async fn upload_dataset_bytes(
    state: &AppState,
    actor_id: Uuid,
    dataset_id: Uuid,
    bytes: &[u8],
    format: &str,
    file_name: String,
) -> Result<i32, String> {
    let token = issue_service_token(state, actor_id)?;
    let url = format!(
        "{}/api/v1/datasets/{dataset_id}/upload",
        state.dataset_service_url.trim_end_matches('/')
    );

    let part = Part::bytes(bytes.to_vec())
        .file_name(file_name)
        .mime_str(mime_for_format(format))
        .map_err(|error| error.to_string())?;
    let form = Form::new().part("file", part);

    let response = state
        .http_client
        .post(url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("dataset upload failed with HTTP {status}: {body}"));
    }

    let payload = serde_json::from_str::<Value>(&body).map_err(|error| error.to_string())?;
    payload
        .get("version")
        .and_then(Value::as_i64)
        .map(|value| value as i32)
        .ok_or_else(|| "dataset upload response did not include version".to_string())
}

fn issue_service_token(state: &AppState, actor_id: Uuid) -> Result<String, String> {
    let claims = build_access_claims(
        &state.jwt_config,
        actor_id,
        "pipeline@openfoundry.local",
        "OpenFoundry Pipeline",
        vec!["admin".to_string()],
        vec!["*:*".to_string()],
        None,
        json!({ "source": "pipeline_runtime" }),
        vec!["service_pipeline".to_string()],
    );
    encode_token(&state.jwt_config, &claims).map_err(|error| error.to_string())
}

fn dataset_aliases(node: &PipelineNode, index: usize, input: &LoadedDataset) -> Vec<String> {
    let mut aliases = vec![
        preferred_alias(node, index, input),
        format!("input_{index}"),
        format!("dataset_{index}"),
        format!(
            "dataset_{}",
            input
                .metadata
                .dataset_id
                .as_simple()
                .to_string()
                .chars()
                .take(12)
                .collect::<String>()
        ),
    ];
    aliases.sort();
    aliases.dedup();
    aliases
        .into_iter()
        .map(|alias| sanitize_alias(&alias))
        .collect()
}

fn preferred_alias(node: &PipelineNode, index: usize, input: &LoadedDataset) -> String {
    node.config
        .get("table_aliases")
        .and_then(Value::as_array)
        .and_then(|aliases| aliases.get(index))
        .and_then(Value::as_str)
        .filter(|alias| !alias.trim().is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| sanitize_alias(&input.metadata.name))
}

fn sanitize_alias(raw: &str) -> String {
    let sanitized = raw
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();
    if sanitized.is_empty() {
        "dataset".to_string()
    } else if sanitized
        .chars()
        .next()
        .map(|ch| ch.is_ascii_digit())
        .unwrap_or(false)
    {
        format!("dataset_{sanitized}")
    } else {
        sanitized
    }
}

fn column_metadata(batches: &[RecordBatch]) -> Vec<Value> {
    batches
        .first()
        .map(|batch| {
            batch
                .schema()
                .fields()
                .iter()
                .map(|field| {
                    json!({
                        "name": field.name(),
                        "data_type": field.data_type().to_string(),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn collect_object_rows(batches: &[RecordBatch]) -> Vec<Value> {
    let mut rows = Vec::new();
    for batch in batches {
        let field_names = batch
            .schema()
            .fields()
            .iter()
            .map(|field| field.name().to_string())
            .collect::<Vec<_>>();
        for row_index in 0..batch.num_rows() {
            let mut row = serde_json::Map::new();
            for (column_index, field_name) in field_names.iter().enumerate() {
                let raw = array_value_to_string(batch.column(column_index), row_index)
                    .unwrap_or_else(|_| "null".to_string());
                row.insert(field_name.clone(), json_scalar_or_string(&raw));
            }
            rows.push(Value::Object(row));
        }
    }
    rows
}

fn json_scalar_or_string(raw: &str) -> Value {
    if raw == "null" {
        Value::Null
    } else {
        serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()))
    }
}

fn normalize_json_bytes(data: &[u8]) -> Result<Vec<u8>, String> {
    let text = from_utf8(data).map_err(|error| error.to_string())?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }

    if trimmed.starts_with('[') || trimmed.starts_with('{') {
        let parsed: Value = serde_json::from_slice(data).map_err(|error| error.to_string())?;
        let mut lines = String::new();
        match parsed {
            Value::Array(rows) => {
                for row in rows {
                    lines
                        .push_str(&serde_json::to_string(&row).map_err(|error| error.to_string())?);
                    lines.push('\n');
                }
            }
            Value::Object(_) => {
                lines.push_str(&serde_json::to_string(&parsed).map_err(|error| error.to_string())?);
                lines.push('\n');
            }
            _ => return Err("JSON datasets must contain objects or arrays of objects".to_string()),
        }
        return Ok(lines.into_bytes());
    }

    Ok(data.to_vec())
}

fn mime_for_format(format: &str) -> &'static str {
    match format {
        "csv" => "text/csv",
        "json" => "application/json",
        "parquet" => "application/octet-stream",
        _ => "application/octet-stream",
    }
}

fn file_extension(format: &str) -> &'static str {
    match format {
        "csv" => "csv",
        "json" => "json",
        "parquet" => "parquet",
        _ => "bin",
    }
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

fn quote_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

async fn cleanup_paths(paths: Vec<PathBuf>) {
    for path in paths {
        let _ = fs::remove_file(path).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_compute_request_uses_node_config_and_inputs() {
        let node = PipelineNode {
            id: "node_spark".to_string(),
            label: "Remote Spark".to_string(),
            transform_type: "spark".to_string(),
            config: json!({
                "endpoint": "http://compute.local/jobs/run",
                "job_type": "spark-batch"
            }),
            depends_on: Vec::new(),
            input_dataset_ids: Vec::new(),
            output_dataset_id: None,
        };

        let prepared_inputs = vec![PreparedInput {
            alias: "orders".to_string(),
            metadata: DatasetInputMetadata {
                dataset_id: Uuid::nil(),
                name: "orders".to_string(),
                format: "json".to_string(),
                version: 1,
                row_count: 1,
                size_bytes: 32,
            },
            rows: vec![json!({ "order_id": 1 })],
        }];

        let (endpoint, request) = build_remote_compute_request(&node, prepared_inputs, "spark")
            .expect("request should build");
        assert_eq!(endpoint, "http://compute.local/jobs/run");
        assert_eq!(request.job_type, "spark-batch");
        assert_eq!(request.pipeline_node_id, "node_spark");
        assert_eq!(request.transform_type, "spark");
        assert_eq!(request.inputs.len(), 1);
        assert_eq!(request.inputs[0].alias, "orders");
    }

    #[test]
    fn remote_compute_output_parses_rows_and_metadata() {
        let payload = RemoteComputeResponse {
            status: Some("completed".to_string()),
            rows_affected: Some(2),
            output: Some(json!({ "engine": "spark" })),
            result_rows: Some(json!([{ "value": 1 }, { "value": 2 }])),
            run_id: Some("spark-run-1".to_string()),
            worker_id: Some("executor-a".to_string()),
        };

        let (rows_affected, output, rows) =
            prepare_remote_compute_output(payload, "http://compute.local/jobs/run", "spark")
                .expect("output should parse");
        assert_eq!(rows_affected, Some(2));
        assert_eq!(output["engine"], json!("spark"));
        assert_eq!(output["run_id"], json!("spark-run-1"));
        assert_eq!(output["worker_id"], json!("executor-a"));
        assert_eq!(rows.expect("rows should exist").len(), 2);
    }

    #[test]
    fn remote_compute_request_requires_endpoint() {
        let node = PipelineNode {
            id: "node_external".to_string(),
            label: "External compute".to_string(),
            transform_type: "external".to_string(),
            config: json!({}),
            depends_on: Vec::new(),
            input_dataset_ids: Vec::new(),
            output_dataset_id: None,
        };

        let error = build_remote_compute_request(&node, Vec::new(), "external")
            .expect_err("missing endpoint should fail");

        assert!(error.contains("endpoint"));
    }

    #[test]
    fn llm_prompt_replaces_supported_placeholders() {
        let prepared_input = PreparedInput {
            alias: "orders".to_string(),
            metadata: DatasetInputMetadata {
                dataset_id: Uuid::nil(),
                name: "orders".to_string(),
                format: "json".to_string(),
                version: 1,
                row_count: 1,
                size_bytes: 32,
            },
            rows: vec![json!({ "customer_id": "c-1", "text": "translate me" })],
        };

        let prompt = render_llm_prompt(
            "Dataset {{dataset_name}} row {{row_index}} says {{input_text}}",
            Some(&prepared_input),
            &prepared_input.rows[0],
            0,
            Some("text"),
        );

        assert!(prompt.contains("orders"));
        assert!(prompt.contains("row 0"));
        assert!(prompt.contains("translate me"));
    }

    #[test]
    fn llm_output_row_merges_json_reply_when_requested() {
        let prepared_input = PreparedInput {
            alias: "reviews".to_string(),
            metadata: DatasetInputMetadata {
                dataset_id: Uuid::nil(),
                name: "reviews".to_string(),
                format: "json".to_string(),
                version: 1,
                row_count: 1,
                size_bytes: 32,
            },
            rows: vec![json!({ "review_id": 7 })],
        };
        let response = ChatCompletionResponsePayload {
            provider_id: Uuid::nil(),
            provider_name: "OpenFoundry AI".to_string(),
            reply: r#"{"sentiment":"positive","score":0.98}"#.to_string(),
            citations: Vec::new(),
            guardrail: ChatCompletionGuardrail { blocked: false },
            cache: ChatCompletionCache { hit: false },
        };

        let row = build_llm_output_row(
            Some(&prepared_input),
            &prepared_input.rows[0],
            0,
            &response,
            "llm_response",
            "json",
            true,
            true,
        )
        .expect("output row should build");

        assert_eq!(row["review_id"], json!(7));
        assert_eq!(row["sentiment"], json!("positive"));
        assert_eq!(row["_llm"]["provider_name"], json!("OpenFoundry AI"));
    }
}
