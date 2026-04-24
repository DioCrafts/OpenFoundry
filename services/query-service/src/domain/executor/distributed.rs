use std::{sync::Arc, time::Instant};

use futures::future::try_join_all;
use query_engine::context::QueryContext;

use crate::domain::executor::datafusion::{
    QueryExecutionMetadata, QueryExecutionWorker, QueryResult, execute_query, execute_query_slice,
};

pub async fn execute_distributed_query(
    query_ctx: Arc<QueryContext>,
    sql: &str,
    limit: usize,
    worker_count: usize,
) -> Result<QueryResult, String> {
    let effective_workers = worker_count.max(1).min(limit.max(1));
    if effective_workers <= 1 {
        return execute_query(query_ctx.as_ref(), sql, limit).await;
    }

    let start = Instant::now();
    let chunk_size = limit.div_ceil(effective_workers.max(1));
    let tasks = (0..effective_workers).map(|index| {
        let query_ctx = query_ctx.clone();
        let sql = sql.to_string();
        async move {
            let offset = index * chunk_size;
            let slice_limit = limit.saturating_sub(offset).min(chunk_size);
            let result = execute_query_slice(query_ctx.as_ref(), &sql, offset, slice_limit).await?;
            let row_count = result.total_rows;
            Ok::<_, String>((
                index,
                result,
                QueryExecutionWorker {
                    worker_id: format!("query-worker-{}", index + 1),
                    offset,
                    row_count,
                    limit: slice_limit,
                },
            ))
        }
    });

    let mut partials = try_join_all(tasks).await?;
    partials.sort_by_key(|(index, _, _)| *index);

    let mut columns = Vec::new();
    let mut rows = Vec::new();
    let mut workers = Vec::new();
    for (_, partial, worker) in partials {
        if columns.is_empty() {
            columns = partial.columns;
        }
        rows.extend(partial.rows);
        workers.push(worker);
    }

    Ok(QueryResult {
        columns,
        rows,
        total_rows: workers.iter().map(|worker| worker.row_count).sum(),
        execution_time_ms: start.elapsed().as_millis(),
        execution: Some(QueryExecutionMetadata {
            mode: "distributed".to_string(),
            worker_count: workers.len(),
            workers,
        }),
    })
}
