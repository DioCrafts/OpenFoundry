use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::models::transaction::DatasetTransaction;

pub struct TransactionRecord {
    pub view_id: Option<Uuid>,
    pub operation: String,
    pub branch_name: Option<String>,
    pub summary: String,
    pub metadata: serde_json::Value,
}

pub async fn record_committed_transaction(
    tx: &mut Transaction<'_, Postgres>,
    dataset_id: Uuid,
    record: TransactionRecord,
) -> Result<DatasetTransaction, sqlx::Error> {
    sqlx::query_as::<_, DatasetTransaction>(
        r#"INSERT INTO dataset_transactions (
               id, dataset_id, view_id, operation, branch_name, status, summary, metadata, committed_at
           )
           VALUES ($1, $2, $3, $4, $5, 'committed', $6, $7::jsonb, NOW())
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(dataset_id)
    .bind(record.view_id)
    .bind(record.operation)
    .bind(record.branch_name)
    .bind(record.summary)
    .bind(record.metadata)
    .fetch_one(&mut **tx)
    .await
}
