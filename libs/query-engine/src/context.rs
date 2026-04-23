use datafusion::prelude::*;
use datafusion::error::DataFusionError;
use std::sync::Arc;

/// OpenFoundry query context wrapping DataFusion's SessionContext.
pub struct QueryContext {
    ctx: SessionContext,
}

impl QueryContext {
    pub fn new() -> Self {
        let config = SessionConfig::new()
            .with_information_schema(true)
            .with_default_catalog_and_schema("open_foundry", "public");
        let ctx = SessionContext::new_with_config(config);
        Self { ctx }
    }

    pub fn inner(&self) -> &SessionContext {
        &self.ctx
    }

    /// Register a Parquet file as a table.
    pub async fn register_parquet(
        &self,
        table_name: &str,
        path: &str,
    ) -> Result<(), DataFusionError> {
        self.ctx
            .register_parquet(table_name, path, ParquetReadOptions::default())
            .await
    }

    /// Register a CSV file as a table.
    pub async fn register_csv(
        &self,
        table_name: &str,
        path: &str,
    ) -> Result<(), DataFusionError> {
        self.ctx
            .register_csv(table_name, path, CsvReadOptions::default())
            .await
    }

    /// Execute a SQL query and return a DataFrame.
    pub async fn sql(&self, query: &str) -> Result<DataFrame, DataFusionError> {
        self.ctx.sql(query).await
    }

    /// Execute SQL and collect all results into RecordBatches.
    pub async fn execute_sql(
        &self,
        query: &str,
    ) -> Result<Vec<arrow::array::RecordBatch>, DataFusionError> {
        let df = self.ctx.sql(query).await?;
        df.collect().await
    }

    /// Get the logical plan for a SQL query (for EXPLAIN).
    pub async fn explain_sql(
        &self,
        query: &str,
    ) -> Result<(String, String), DataFusionError> {
        let df = self.ctx.sql(query).await?;
        let logical_plan = format!("{:?}", df.logical_plan());
        let physical_plan = format!("{:?}", df.create_physical_plan().await?);
        Ok((logical_plan, physical_plan))
    }

    /// Register an in-memory table from RecordBatches.
    pub fn register_batch(
        &self,
        table_name: &str,
        batch: arrow::array::RecordBatch,
    ) -> Result<(), DataFusionError> {
        let schema = batch.schema();
        let provider = datafusion::datasource::MemTable::try_new(schema, vec![vec![batch]])?;
        self.ctx
            .register_table(table_name, Arc::new(provider))?;
        Ok(())
    }
}

impl Default for QueryContext {
    fn default() -> Self {
        Self::new()
    }
}

