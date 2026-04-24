use auth_middleware::{Claims, JwtConfig, jwt::encode_token};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::domain::kernel::KernelExecutionResult;

#[derive(Debug, Serialize)]
struct ExecuteQueryRequest<'a> {
    sql: &'a str,
    limit: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
struct QueryColumn {
    name: String,
    data_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExecuteQueryResponse {
    columns: Vec<QueryColumn>,
    rows: Vec<Vec<String>>,
    total_rows: usize,
    execution_time_ms: u64,
}

pub async fn execute(
    http_client: &Client,
    query_service_url: &str,
    jwt_config: &JwtConfig,
    claims: &Claims,
    source: &str,
) -> Result<KernelExecutionResult, String> {
    let token = encode_token(jwt_config, claims)
        .map_err(|error| format!("failed to sign query-service token: {error}"))?;

    let response = http_client
        .post(format!(
            "{}/api/v1/queries/execute",
            query_service_url.trim_end_matches('/'),
        ))
        .bearer_auth(token)
        .json(&ExecuteQueryRequest {
            sql: source,
            limit: Some(1000),
        })
        .send()
        .await
        .map_err(|error| format!("query-service request failed: {error}"))?;

    let status = response.status();
    if !status.is_success() {
        let error_payload: serde_json::Value = response
            .json()
            .await
            .unwrap_or_else(|_| serde_json::json!({ "error": status.to_string() }));

        let error = error_payload
            .get("error")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("query execution failed");

        return Err(error.to_string());
    }

    let payload: ExecuteQueryResponse = response
        .json()
        .await
        .map_err(|error| format!("invalid query-service response: {error}"))?;

    Ok(KernelExecutionResult {
        output_type: "table".into(),
        content: serde_json::to_value(payload)
            .map_err(|error| format!("failed to serialize SQL result: {error}"))?,
    })
}
