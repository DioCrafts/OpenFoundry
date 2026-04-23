use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct LineageEdge {
    pub id: Uuid,
    pub source_dataset_id: Uuid,
    pub target_dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub node_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageGraphEdge>,
}

#[derive(Debug, Serialize)]
pub struct LineageNode {
    pub dataset_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct LineageGraphEdge {
    pub source: Uuid,
    pub target: Uuid,
    pub pipeline_id: Option<Uuid>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct ColumnLineageEdge {
    pub id: Uuid,
    pub source_dataset_id: Uuid,
    pub source_column: String,
    pub target_dataset_id: Uuid,
    pub target_column: String,
    pub pipeline_id: Option<Uuid>,
    pub node_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Get lineage graph rooted at a specific dataset (upstream + downstream).
pub async fn get_lineage_graph(db: &PgPool, dataset_id: Uuid) -> Result<LineageGraph, sqlx::Error> {
    let edges = sqlx::query_as::<_, LineageEdge>(
        r#"SELECT * FROM lineage_edges
           WHERE source_dataset_id = $1 OR target_dataset_id = $1"#,
    )
    .bind(dataset_id)
    .fetch_all(db)
    .await?;

    Ok(build_graph(&edges))
}

/// Get the full lineage graph for all datasets.
pub async fn get_full_lineage_graph(db: &PgPool) -> Result<LineageGraph, sqlx::Error> {
    let edges = sqlx::query_as::<_, LineageEdge>("SELECT * FROM lineage_edges")
        .fetch_all(db)
        .await?;

    Ok(build_graph(&edges))
}

/// Record a lineage edge between datasets.
pub async fn record_lineage(
    db: &PgPool,
    source_dataset_id: Uuid,
    target_dataset_id: Uuid,
    pipeline_id: Option<Uuid>,
    node_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO lineage_edges (id, source_dataset_id, target_dataset_id, pipeline_id, node_id)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(source_dataset_id)
    .bind(target_dataset_id)
    .bind(pipeline_id)
    .bind(node_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn get_dataset_column_lineage(
    db: &PgPool,
    dataset_id: Uuid,
) -> Result<Vec<ColumnLineageEdge>, sqlx::Error> {
    sqlx::query_as::<_, ColumnLineageEdge>(
        r#"SELECT * FROM column_lineage_edges
           WHERE source_dataset_id = $1 OR target_dataset_id = $1
           ORDER BY created_at DESC"#,
    )
    .bind(dataset_id)
    .fetch_all(db)
    .await
}

pub async fn record_column_lineage(
    db: &PgPool,
    source_dataset_id: Uuid,
    source_column: &str,
    target_dataset_id: Uuid,
    target_column: &str,
    pipeline_id: Option<Uuid>,
    node_id: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO column_lineage_edges (
               id, source_dataset_id, source_column, target_dataset_id, target_column, pipeline_id, node_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT DO NOTHING"#,
    )
    .bind(Uuid::now_v7())
    .bind(source_dataset_id)
    .bind(source_column)
    .bind(target_dataset_id)
    .bind(target_column)
    .bind(pipeline_id)
    .bind(node_id)
    .execute(db)
    .await?;
    Ok(())
}

fn build_graph(edges: &[LineageEdge]) -> LineageGraph {
    let mut node_ids = std::collections::HashSet::new();
    let mut graph_edges = Vec::new();

    for e in edges {
        node_ids.insert(e.source_dataset_id);
        node_ids.insert(e.target_dataset_id);
        graph_edges.push(LineageGraphEdge {
            source: e.source_dataset_id,
            target: e.target_dataset_id,
            pipeline_id: e.pipeline_id,
        });
    }

    let nodes = node_ids.into_iter().map(|id| LineageNode { dataset_id: id }).collect();

    LineageGraph { nodes, edges: graph_edges }
}
