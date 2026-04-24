use arrow::array::RecordBatch;
use query_engine::context::QueryContext;
use serde::Serialize;
use serde_json::Value;
use std::time::Instant;

#[derive(Debug, Serialize)]
pub struct QueryResult {
    pub columns: Vec<ColumnMeta>,
    pub rows: Vec<Vec<Value>>,
    pub total_rows: usize,
    pub execution_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution: Option<QueryExecutionMetadata>,
}

#[derive(Debug, Serialize)]
pub struct ColumnMeta {
    pub name: String,
    pub data_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryExecutionMetadata {
    pub mode: String,
    pub worker_count: usize,
    pub workers: Vec<QueryExecutionWorker>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryExecutionWorker {
    pub worker_id: String,
    pub offset: usize,
    pub row_count: usize,
    pub limit: usize,
}

/// Execute a SQL query using DataFusion and return JSON-serializable results.
pub async fn execute_query(
    ctx: &QueryContext,
    sql: &str,
    limit: usize,
) -> Result<QueryResult, String> {
    let start = Instant::now();
    let mut result = execute_query_slice(ctx, sql, 0, limit).await?;
    result.execution_time_ms = start.elapsed().as_millis();
    result.execution = Some(QueryExecutionMetadata {
        mode: "local".to_string(),
        worker_count: 1,
        workers: vec![QueryExecutionWorker {
            worker_id: "query-local-1".to_string(),
            offset: 0,
            row_count: result.total_rows,
            limit,
        }],
    });
    Ok(result)
}

pub async fn execute_query_slice(
    ctx: &QueryContext,
    sql: &str,
    offset: usize,
    limit: usize,
) -> Result<QueryResult, String> {
    let df = ctx.sql(sql).await.map_err(|e| e.to_string())?;
    let schema = df.schema().clone();

    let columns: Vec<ColumnMeta> = schema
        .fields()
        .iter()
        .map(|f| ColumnMeta {
            name: f.name().clone(),
            data_type: f.data_type().to_string(),
        })
        .collect();

    let batches: Vec<RecordBatch> = df
        .limit(offset, Some(limit))
        .map_err(|e| e.to_string())?
        .collect()
        .await
        .map_err(|e| e.to_string())?;

    let mut rows = Vec::new();
    for batch in &batches {
        for row_idx in 0..batch.num_rows() {
            let mut row = Vec::new();
            for col_idx in 0..batch.num_columns() {
                let col = batch.column(col_idx);
                let val = arrow::util::display::array_value_to_string(col, row_idx)
                    .unwrap_or_else(|_| "null".to_string());
                row.push(Value::String(val));
            }
            rows.push(row);
        }
    }

    let total_rows = rows.len();

    Ok(QueryResult {
        columns,
        rows,
        total_rows,
        execution_time_ms: 0,
        execution: None,
    })
}

/// Get the logical and physical plan for a SQL query.
pub async fn explain_query(ctx: &QueryContext, sql: &str) -> Result<(String, String), String> {
    ctx.explain_sql(sql).await.map_err(|e| e.to_string())
}
