use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use auth_middleware::claims::Claims;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{AppState, domain::executor, models::pipeline::Pipeline};

#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct LineageEdge {
    pub id: Uuid,
    pub source_dataset_id: Uuid,
    pub target_dataset_id: Uuid,
    pub pipeline_id: Option<Uuid>,
    pub node_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
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

#[derive(Debug, Clone, Serialize)]
pub struct LineageGraph {
    pub nodes: Vec<LineageNode>,
    pub edges: Vec<LineageGraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageNode {
    pub id: Uuid,
    pub kind: String,
    pub label: String,
    pub marking: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageGraphEdge {
    pub id: Uuid,
    pub source: Uuid,
    pub source_kind: String,
    pub target: Uuid,
    pub target_kind: String,
    pub relation_kind: String,
    pub pipeline_id: Option<Uuid>,
    pub workflow_id: Option<Uuid>,
    pub node_id: Option<String>,
    pub step_id: Option<String>,
    pub effective_marking: String,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineagePathHop {
    pub source_id: Uuid,
    pub source_kind: String,
    pub target_id: Uuid,
    pub target_kind: String,
    pub relation_kind: String,
    pub effective_marking: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageImpactItem {
    pub id: Uuid,
    pub kind: String,
    pub label: String,
    pub distance: usize,
    pub marking: String,
    pub effective_marking: String,
    pub requires_acknowledgement: bool,
    pub metadata: Value,
    pub path: Vec<LineagePathHop>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageBuildCandidate {
    pub id: Uuid,
    pub kind: String,
    pub label: String,
    pub status: Option<String>,
    pub distance: usize,
    pub triggerable: bool,
    pub marking: String,
    pub effective_marking: String,
    pub requires_acknowledgement: bool,
    pub blocked_reason: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageImpactAnalysis {
    pub root: LineageNode,
    pub propagated_marking: String,
    pub upstream: Vec<LineageImpactItem>,
    pub downstream: Vec<LineageImpactItem>,
    pub build_candidates: Vec<LineageBuildCandidate>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct LineageBuildRequest {
    #[serde(default)]
    pub include_workflows: bool,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub acknowledge_sensitive_lineage: bool,
    #[serde(default)]
    pub max_depth: Option<usize>,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageBuildTriggerResult {
    pub id: Uuid,
    pub kind: String,
    pub label: String,
    pub run_id: Option<Uuid>,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LineageBuildResult {
    pub root: LineageNode,
    pub dry_run: bool,
    pub acknowledged_sensitive_lineage: bool,
    pub propagated_marking: String,
    pub candidates: Vec<LineageBuildCandidate>,
    pub triggered: Vec<LineageBuildTriggerResult>,
    pub skipped: Vec<LineageBuildTriggerResult>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowLineageSyncRequest {
    pub workflow: WorkflowLineageNodeInput,
    #[serde(default)]
    pub relations: Vec<WorkflowLineageRelationInput>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowLineageNodeInput {
    pub id: Uuid,
    pub label: String,
    pub status: String,
    pub trigger_type: String,
    #[serde(default)]
    pub marking: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorkflowLineageRelationInput {
    pub source_id: Uuid,
    pub source_kind: String,
    pub target_id: Uuid,
    pub target_kind: String,
    pub relation_kind: String,
    #[serde(default)]
    pub step_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    #[serde(default)]
    pub marking: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InternalWorkflowLineageRunRequest {
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LineageNodeRecord {
    entity_id: Uuid,
    entity_kind: String,
    label: String,
    marking: String,
    metadata: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct LineageRelationRecord {
    id: Uuid,
    source_id: Uuid,
    source_kind: String,
    target_id: Uuid,
    target_kind: String,
    relation_kind: String,
    pipeline_id: Option<Uuid>,
    workflow_id: Option<Uuid>,
    node_id: Option<String>,
    step_id: Option<String>,
    effective_marking: String,
    metadata: Value,
}

#[derive(Debug, Clone, Deserialize)]
struct DatasetMetadata {
    id: Uuid,
    name: String,
    format: String,
    marking: String,
    tags: Vec<String>,
    current_version: i32,
    active_branch: String,
    owner_id: Uuid,
    updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct NodeKey {
    id: Uuid,
    kind: NodeKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum NodeKind {
    Dataset,
    Pipeline,
    Workflow,
}

impl NodeKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Dataset => "dataset",
            Self::Pipeline => "pipeline",
            Self::Workflow => "workflow",
        }
    }

    fn parse(value: &str) -> Option<Self> {
        match value {
            "dataset" => Some(Self::Dataset),
            "pipeline" => Some(Self::Pipeline),
            "workflow" => Some(Self::Workflow),
            _ => None,
        }
    }
}

struct RelationWriteInput<'a> {
    source_id: Uuid,
    source_kind: NodeKind,
    target_id: Uuid,
    target_kind: NodeKind,
    relation_kind: &'a str,
    producer_key: String,
    pipeline_id: Option<Uuid>,
    workflow_id: Option<Uuid>,
    node_id: Option<&'a str>,
    step_id: Option<&'a str>,
    explicit_marking: Option<String>,
    metadata: Value,
}

/// Get lineage graph rooted at a specific dataset (upstream + downstream).
pub async fn get_lineage_graph(db: &PgPool, dataset_id: Uuid) -> Result<LineageGraph, sqlx::Error> {
    let nodes = load_lineage_nodes(db).await?;
    let relations = load_lineage_relations(db).await?;
    let root = NodeKey {
        id: dataset_id,
        kind: NodeKind::Dataset,
    };
    let reachable = collect_connected_nodes(root, &relations);
    Ok(build_graph(&nodes, &relations, Some(&reachable)))
}

/// Get the full lineage graph for all datasets, pipelines, and workflows.
pub async fn get_full_lineage_graph(db: &PgPool) -> Result<LineageGraph, sqlx::Error> {
    let nodes = load_lineage_nodes(db).await?;
    let relations = load_lineage_relations(db).await?;
    Ok(build_graph(&nodes, &relations, None))
}

pub async fn get_lineage_impact_analysis(
    db: &PgPool,
    dataset_id: Uuid,
) -> Result<Option<LineageImpactAnalysis>, sqlx::Error> {
    let nodes = load_lineage_nodes(db).await?;
    let relations = load_lineage_relations(db).await?;
    let root = NodeKey {
        id: dataset_id,
        kind: NodeKind::Dataset,
    };
    let Some(root_node) = build_node_view(root, &nodes) else {
        return Ok(None);
    };

    let upstream = bfs_paths(root, &relations, Direction::Incoming);
    let downstream = bfs_paths(root, &relations, Direction::Outgoing);

    let upstream_items = build_impact_items(root, &upstream, &nodes, &relations);
    let downstream_items = build_impact_items(root, &downstream, &nodes, &relations);
    let build_candidates = downstream_items
        .iter()
        .filter(|item| {
            matches!(
                NodeKind::parse(&item.kind),
                Some(NodeKind::Pipeline | NodeKind::Workflow)
            )
        })
        .map(|item| build_candidate(item, &nodes))
        .collect();

    Ok(Some(LineageImpactAnalysis {
        propagated_marking: root_node.marking.clone(),
        root: root_node,
        upstream: upstream_items,
        downstream: downstream_items,
        build_candidates,
    }))
}

pub async fn trigger_lineage_builds(
    state: &AppState,
    dataset_id: Uuid,
    requested_by: Uuid,
    request: LineageBuildRequest,
) -> Result<LineageBuildResult, String> {
    let impact = get_lineage_impact_analysis(&state.db, dataset_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "dataset has no lineage graph yet".to_string())?;

    let max_depth = request.max_depth.unwrap_or(8).max(1);
    let mut candidates: Vec<LineageBuildCandidate> = impact
        .build_candidates
        .iter()
        .filter(|candidate| candidate.distance <= max_depth)
        .filter(|candidate| request.include_workflows || candidate.kind != "workflow")
        .cloned()
        .collect();
    candidates.sort_by_key(|candidate| {
        (
            candidate.distance,
            candidate.kind.clone(),
            candidate.label.clone(),
        )
    });

    let mut triggered = Vec::new();
    let mut skipped = Vec::new();

    if !request.dry_run {
        for candidate in &mut candidates {
            candidate.blocked_reason = None;
            if candidate.requires_acknowledgement && !request.acknowledge_sensitive_lineage {
                let message = format!(
                    "acknowledge sensitive lineage before triggering {} build with {} marking",
                    candidate.kind, candidate.effective_marking
                );
                candidate.blocked_reason = Some(message.clone());
                skipped.push(LineageBuildTriggerResult {
                    id: candidate.id,
                    kind: candidate.kind.clone(),
                    label: candidate.label.clone(),
                    run_id: None,
                    status: "blocked".to_string(),
                    message: Some(message),
                });
                continue;
            }

            if !candidate.triggerable {
                candidate.blocked_reason = Some("candidate is not active".to_string());
                skipped.push(LineageBuildTriggerResult {
                    id: candidate.id,
                    kind: candidate.kind.clone(),
                    label: candidate.label.clone(),
                    run_id: None,
                    status: "skipped".to_string(),
                    message: Some("candidate is not active".to_string()),
                });
                continue;
            }

            let mut build_context = request.context.clone();
            ensure_object(&mut build_context);
            if let Some(map) = build_context.as_object_mut() {
                map.insert(
                    "lineage_build".to_string(),
                    json!({
                        "root_dataset_id": dataset_id,
                        "root_marking": impact.propagated_marking,
                        "candidate_id": candidate.id,
                        "candidate_kind": candidate.kind.clone(),
                        "candidate_marking": candidate.marking.clone(),
                        "effective_marking": candidate.effective_marking.clone(),
                        "requires_acknowledgement": candidate.requires_acknowledgement,
                        "acknowledged_sensitive_lineage": request.acknowledge_sensitive_lineage,
                        "requested_by": requested_by,
                        "requested_at": Utc::now(),
                    }),
                );
            }

            match candidate.kind.as_str() {
                "pipeline" => {
                    let pipeline = load_pipeline(state, candidate.id)
                        .await
                        .map_err(|error| error.to_string())?
                        .ok_or_else(|| format!("pipeline {} not found", candidate.id))?;

                    match executor::start_pipeline_run(
                        state,
                        &pipeline,
                        Some(requested_by),
                        "lineage_build",
                        None,
                        None,
                        1,
                        state.distributed_pipeline_workers.max(1),
                        true,
                        build_context,
                    )
                    .await
                    {
                        Ok(run) => triggered.push(LineageBuildTriggerResult {
                            id: candidate.id,
                            kind: candidate.kind.clone(),
                            label: candidate.label.clone(),
                            run_id: Some(run.id),
                            status: run.status,
                            message: None,
                        }),
                        Err(error) => skipped.push(LineageBuildTriggerResult {
                            id: candidate.id,
                            kind: candidate.kind.clone(),
                            label: candidate.label.clone(),
                            run_id: None,
                            status: "failed".to_string(),
                            message: Some(error),
                        }),
                    }
                }
                "workflow" => {
                    let endpoint = format!(
                        "{}/internal/workflows/{}/runs/lineage",
                        state.workflow_service_url.trim_end_matches('/'),
                        candidate.id
                    );
                    let response = state
                        .http_client
                        .post(endpoint)
                        .json(&InternalWorkflowLineageRunRequest {
                            context: build_context,
                        })
                        .send()
                        .await
                        .map_err(|error| error.to_string());

                    match response {
                        Ok(response) => {
                            let status = response.status();
                            if !status.is_success() {
                                let body = response.text().await.unwrap_or_default();
                                skipped.push(LineageBuildTriggerResult {
                                    id: candidate.id,
                                    kind: candidate.kind.clone(),
                                    label: candidate.label.clone(),
                                    run_id: None,
                                    status: "failed".to_string(),
                                    message: Some(body),
                                });
                                continue;
                            }

                            let body = response.json::<Value>().await.unwrap_or_else(|_| json!({}));
                            let run_id = body
                                .get("id")
                                .and_then(Value::as_str)
                                .and_then(|raw| Uuid::parse_str(raw).ok());
                            let run_status = body
                                .get("status")
                                .and_then(Value::as_str)
                                .unwrap_or("completed")
                                .to_string();
                            triggered.push(LineageBuildTriggerResult {
                                id: candidate.id,
                                kind: candidate.kind.clone(),
                                label: candidate.label.clone(),
                                run_id,
                                status: run_status,
                                message: None,
                            });
                        }
                        Err(error) => skipped.push(LineageBuildTriggerResult {
                            id: candidate.id,
                            kind: candidate.kind.clone(),
                            label: candidate.label.clone(),
                            run_id: None,
                            status: "failed".to_string(),
                            message: Some(error),
                        }),
                    }
                }
                _ => {}
            }
        }
    }

    Ok(LineageBuildResult {
        root: impact.root,
        dry_run: request.dry_run,
        acknowledged_sensitive_lineage: request.acknowledge_sensitive_lineage,
        propagated_marking: impact.propagated_marking,
        candidates,
        triggered,
        skipped,
    })
}

pub async fn sync_workflow_lineage(
    state: &AppState,
    workflow_id: Uuid,
    body: WorkflowLineageSyncRequest,
) -> Result<(), String> {
    if body.workflow.id != workflow_id {
        return Err("workflow lineage payload does not match route workflow_id".to_string());
    }

    delete_workflow_relations(&state.db, workflow_id)
        .await
        .map_err(|error| error.to_string())?;

    let explicit_workflow_marking = normalize_marking(body.workflow.marking.as_deref());
    let existing_workflow = get_node_record(&state.db, workflow_id, NodeKind::Workflow)
        .await
        .map_err(|error| error.to_string())?;

    for relation in &body.relations {
        ensure_external_nodes(state, relation)
            .await
            .map_err(|error| error.to_string())?;
    }

    let mut workflow_marking = max_markings([
        explicit_workflow_marking.as_deref(),
        existing_workflow
            .as_ref()
            .map(|record| record.marking.as_str()),
    ]);

    for relation in &body.relations {
        if relation.target_id == workflow_id && relation.target_kind == "workflow" {
            if let Some(kind) = NodeKind::parse(&relation.source_kind) {
                let source = get_node_record(&state.db, relation.source_id, kind)
                    .await
                    .map_err(|error| error.to_string())?;
                workflow_marking = max_markings([
                    Some(workflow_marking.as_str()),
                    source.as_ref().map(|record| record.marking.as_str()),
                    normalize_marking(relation.marking.as_deref()).as_deref(),
                ]);
            }
        }
    }

    upsert_node(
        &state.db,
        workflow_id,
        NodeKind::Workflow,
        &body.workflow.label,
        &workflow_marking,
        merge_metadata(
            existing_workflow.as_ref().map(|record| &record.metadata),
            json!({
                "status": body.workflow.status,
                "trigger_type": body.workflow.trigger_type,
                "source": "workflow_service",
                "lineage_synced_at": Utc::now(),
            }),
            Some(&body.workflow.metadata),
        ),
    )
    .await
    .map_err(|error| error.to_string())?;

    for relation in &body.relations {
        let source_kind = NodeKind::parse(&relation.source_kind)
            .ok_or_else(|| format!("unsupported source kind '{}'", relation.source_kind))?;
        let target_kind = NodeKind::parse(&relation.target_kind)
            .ok_or_else(|| format!("unsupported target kind '{}'", relation.target_kind))?;

        if relation.source_id == workflow_id && source_kind == NodeKind::Workflow {
            let target = get_node_record(&state.db, relation.target_id, target_kind)
                .await
                .map_err(|error| error.to_string())?;
            let target_label = target
                .as_ref()
                .map(|record| record.label.clone())
                .unwrap_or_else(|| synthetic_label(target_kind, relation.target_id));
            let target_marking = max_markings([
                Some(workflow_marking.as_str()),
                target.as_ref().map(|record| record.marking.as_str()),
                normalize_marking(relation.marking.as_deref()).as_deref(),
            ]);
            upsert_node(
                &state.db,
                relation.target_id,
                target_kind,
                &target_label,
                &target_marking,
                merge_metadata(
                    target.as_ref().map(|record| &record.metadata),
                    json!({
                        "propagated_from_workflow_id": workflow_id,
                        "propagated_at": Utc::now(),
                    }),
                    None,
                ),
            )
            .await
            .map_err(|error| error.to_string())?;
        }

        persist_relation(
            &state.db,
            RelationWriteInput {
                source_id: relation.source_id,
                source_kind,
                target_id: relation.target_id,
                target_kind,
                relation_kind: &relation.relation_kind,
                producer_key: format!(
                    "workflow:{}:{}:{}",
                    workflow_id,
                    relation
                        .step_id
                        .clone()
                        .unwrap_or_else(|| "trigger".to_string()),
                    relation.relation_kind
                ),
                pipeline_id: None,
                workflow_id: Some(workflow_id),
                node_id: None,
                step_id: relation.step_id.as_deref(),
                explicit_marking: relation.marking.clone(),
                metadata: relation.metadata.clone(),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}

pub async fn delete_workflow_lineage(db: &PgPool, workflow_id: Uuid) -> Result<(), sqlx::Error> {
    delete_workflow_relations(db, workflow_id).await?;
    sqlx::query(
        r#"DELETE FROM lineage_nodes
           WHERE entity_id = $1 AND entity_kind = 'workflow'"#,
    )
    .bind(workflow_id)
    .execute(db)
    .await?;
    Ok(())
}

pub async fn ensure_dataset_snapshot(
    state: &AppState,
    dataset_id: Uuid,
) -> Result<Option<LineageNode>, String> {
    let response = state
        .http_client
        .get(format!(
            "{}/internal/datasets/{}/metadata",
            state.dataset_service_url.trim_end_matches('/'),
            dataset_id
        ))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }

    let response = response
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let dataset = response
        .json::<DatasetMetadata>()
        .await
        .map_err(|error| error.to_string())?;

    let existing = get_node_record(&state.db, dataset.id, NodeKind::Dataset)
        .await
        .map_err(|error| error.to_string())?;
    let base_marking = normalize_marking(Some(dataset.marking.as_str()))
        .unwrap_or_else(|| marking_from_dataset_tags(&dataset.tags));
    let effective_marking = max_markings([
        Some(base_marking.as_str()),
        existing.as_ref().map(|record| record.marking.as_str()),
    ]);

    let record = upsert_node(
        &state.db,
        dataset.id,
        NodeKind::Dataset,
        &dataset.name,
        &effective_marking,
        merge_metadata(
            existing.as_ref().map(|record| &record.metadata),
            json!({
                "format": dataset.format,
                "tags": dataset.tags,
                "current_version": dataset.current_version,
                "active_branch": dataset.active_branch,
                "owner_id": dataset.owner_id,
                "dataset_marking": dataset.marking,
                "base_marking": base_marking,
                "metadata_refreshed_at": dataset.updated_at,
            }),
            None,
        ),
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(Some(node_from_record(&record)))
}

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

pub fn can_access_marking(claims: &Claims, marking: &str) -> bool {
    claims.has_role("admin") || clearance_rank(claims) >= marking_rank(marking)
}

pub fn filter_graph_for_claims(graph: LineageGraph, claims: &Claims) -> LineageGraph {
    if claims.has_role("admin") {
        return graph;
    }

    let allowed: HashSet<Uuid> = graph
        .nodes
        .iter()
        .filter(|node| can_access_marking(claims, &node.marking))
        .map(|node| node.id)
        .collect();

    let nodes = graph
        .nodes
        .into_iter()
        .filter(|node| allowed.contains(&node.id))
        .collect();
    let edges = graph
        .edges
        .into_iter()
        .filter(|edge| {
            allowed.contains(&edge.source)
                && allowed.contains(&edge.target)
                && can_access_marking(claims, &edge.effective_marking)
        })
        .collect();

    LineageGraph { nodes, edges }
}

pub fn filter_impact_for_claims(
    impact: LineageImpactAnalysis,
    claims: &Claims,
) -> Result<LineageImpactAnalysis, String> {
    if !can_access_marking(claims, &impact.root.marking) {
        return Err("forbidden: insufficient classification clearance".to_string());
    }

    if claims.has_role("admin") {
        return Ok(impact);
    }

    let upstream = impact
        .upstream
        .into_iter()
        .filter(|item| can_access_marking(claims, &item.effective_marking))
        .collect();
    let downstream: Vec<_> = impact
        .downstream
        .into_iter()
        .filter(|item| can_access_marking(claims, &item.effective_marking))
        .collect();
    let allowed_ids: BTreeSet<Uuid> = downstream.iter().map(|item| item.id).collect();
    let build_candidates = impact
        .build_candidates
        .into_iter()
        .filter(|candidate| {
            allowed_ids.contains(&candidate.id)
                && can_access_marking(claims, &candidate.effective_marking)
        })
        .collect();

    Ok(LineageImpactAnalysis {
        root: impact.root,
        propagated_marking: impact.propagated_marking,
        upstream,
        downstream,
        build_candidates,
    })
}

async fn ensure_external_nodes(
    state: &AppState,
    relation: &WorkflowLineageRelationInput,
) -> Result<(), String> {
    for (id, kind) in [
        (relation.source_id, relation.source_kind.as_str()),
        (relation.target_id, relation.target_kind.as_str()),
    ] {
        match NodeKind::parse(kind) {
            Some(NodeKind::Dataset) => {
                let _ = ensure_dataset_snapshot(state, id).await?;
            }
            Some(NodeKind::Pipeline) => {
                let _ = ensure_pipeline_snapshot(&state.db, id)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            Some(NodeKind::Workflow) if id != relation.source_id || kind != "workflow" => {
                ensure_placeholder_workflow(&state.db, id)
                    .await
                    .map_err(|error| error.to_string())?;
            }
            _ => {}
        }
    }

    Ok(())
}

pub async fn ensure_pipeline_snapshot(
    db: &PgPool,
    pipeline_id: Uuid,
) -> Result<Option<LineageNode>, sqlx::Error> {
    let existing = get_node_record(db, pipeline_id, NodeKind::Pipeline).await?;
    let pipeline = load_pipeline_by_id(db, pipeline_id).await?;
    let Some(pipeline) = pipeline else {
        if let Some(existing) = existing {
            return Ok(Some(node_from_record(&existing)));
        }
        return Ok(None);
    };

    let record = upsert_node(
        db,
        pipeline.id,
        NodeKind::Pipeline,
        &pipeline.name,
        existing
            .as_ref()
            .map(|record| record.marking.as_str())
            .unwrap_or("public"),
        merge_metadata(
            existing.as_ref().map(|record| &record.metadata),
            json!({
                "status": pipeline.status,
                "description": pipeline.description,
                "owner_id": pipeline.owner_id,
                "next_run_at": pipeline.next_run_at,
                "schedule_config": pipeline.schedule_config,
                "retry_policy": pipeline.retry_policy,
            }),
            None,
        ),
    )
    .await?;

    Ok(Some(node_from_record(&record)))
}

pub async fn propagate_pipeline_runtime_lineage(
    state: &AppState,
    pipeline: &Pipeline,
    node_id: &str,
    node_label: &str,
    transform_type: &str,
    input_dataset_ids: &[Uuid],
    output_dataset_id: Uuid,
    explicit_marking: Option<String>,
) -> Result<(), String> {
    let mut source_nodes = Vec::new();
    for source_dataset_id in input_dataset_ids {
        if let Some(source_node) = ensure_dataset_snapshot(state, *source_dataset_id).await? {
            source_nodes.push(source_node);
        }
    }

    let Some(target_dataset) = ensure_dataset_snapshot(state, output_dataset_id).await? else {
        return Ok(());
    };

    let pipeline_marking = max_markings(
        source_nodes
            .iter()
            .map(|node| Some(node.marking.as_str()))
            .chain([
                Some(target_dataset.marking.as_str()),
                explicit_marking.as_deref(),
            ]),
    );

    upsert_node(
        &state.db,
        pipeline.id,
        NodeKind::Pipeline,
        &pipeline.name,
        &pipeline_marking,
        json!({
            "status": pipeline.status,
            "description": pipeline.description,
            "owner_id": pipeline.owner_id,
            "next_run_at": pipeline.next_run_at,
            "last_lineage_node_id": node_id,
            "last_lineage_transform_type": transform_type,
        }),
    )
    .await
    .map_err(|error| error.to_string())?;

    upsert_node(
        &state.db,
        output_dataset_id,
        NodeKind::Dataset,
        &target_dataset.label,
        &max_markings([
            Some(target_dataset.marking.as_str()),
            Some(pipeline_marking.as_str()),
        ]),
        merge_metadata(
            Some(&target_dataset.metadata),
            json!({
                "propagated_from_pipeline_id": pipeline.id,
                "propagated_at": Utc::now(),
            }),
            None,
        ),
    )
    .await
    .map_err(|error| error.to_string())?;

    for source_node in &source_nodes {
        persist_relation(
            &state.db,
            RelationWriteInput {
                source_id: source_node.id,
                source_kind: NodeKind::Dataset,
                target_id: pipeline.id,
                target_kind: NodeKind::Pipeline,
                relation_kind: "consumes",
                producer_key: format!(
                    "pipeline:{}:node:{}:input:{}",
                    pipeline.id, node_id, source_node.id
                ),
                pipeline_id: Some(pipeline.id),
                workflow_id: None,
                node_id: Some(node_id),
                step_id: None,
                explicit_marking: explicit_marking.clone(),
                metadata: json!({
                    "node_label": node_label,
                    "transform_type": transform_type,
                }),
            },
        )
        .await
        .map_err(|error| error.to_string())?;
    }

    persist_relation(
        &state.db,
        RelationWriteInput {
            source_id: pipeline.id,
            source_kind: NodeKind::Pipeline,
            target_id: output_dataset_id,
            target_kind: NodeKind::Dataset,
            relation_kind: "produces",
            producer_key: format!(
                "pipeline:{}:node:{}:output:{}",
                pipeline.id, node_id, output_dataset_id
            ),
            pipeline_id: Some(pipeline.id),
            workflow_id: None,
            node_id: Some(node_id),
            step_id: None,
            explicit_marking,
            metadata: json!({
                "node_label": node_label,
                "transform_type": transform_type,
            }),
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}

async fn persist_relation(db: &PgPool, input: RelationWriteInput<'_>) -> Result<(), sqlx::Error> {
    let source = get_node_record(db, input.source_id, input.source_kind).await?;
    let target = get_node_record(db, input.target_id, input.target_kind).await?;
    let effective_marking = max_markings([
        source.as_ref().map(|record| record.marking.as_str()),
        target.as_ref().map(|record| record.marking.as_str()),
        input.explicit_marking.as_deref(),
    ]);

    sqlx::query(
        r#"INSERT INTO lineage_relations (
               id, source_id, source_kind, target_id, target_kind, relation_kind,
               producer_key, pipeline_id, workflow_id, node_id, step_id,
               effective_marking, metadata
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
           ON CONFLICT (source_id, source_kind, target_id, target_kind, relation_kind, producer_key)
           DO UPDATE SET
               pipeline_id = EXCLUDED.pipeline_id,
               workflow_id = EXCLUDED.workflow_id,
               node_id = EXCLUDED.node_id,
               step_id = EXCLUDED.step_id,
               effective_marking = EXCLUDED.effective_marking,
               metadata = EXCLUDED.metadata,
               updated_at = NOW()"#,
    )
    .bind(Uuid::now_v7())
    .bind(input.source_id)
    .bind(input.source_kind.as_str())
    .bind(input.target_id)
    .bind(input.target_kind.as_str())
    .bind(input.relation_kind)
    .bind(&input.producer_key)
    .bind(input.pipeline_id)
    .bind(input.workflow_id)
    .bind(input.node_id)
    .bind(input.step_id)
    .bind(effective_marking)
    .bind(&input.metadata)
    .execute(db)
    .await?;

    Ok(())
}

async fn upsert_node(
    db: &PgPool,
    entity_id: Uuid,
    kind: NodeKind,
    label: &str,
    marking: &str,
    metadata: Value,
) -> Result<LineageNodeRecord, sqlx::Error> {
    sqlx::query_as::<_, LineageNodeRecord>(
        r#"INSERT INTO lineage_nodes (entity_id, entity_kind, label, marking, metadata)
           VALUES ($1, $2, $3, $4, $5)
           ON CONFLICT (entity_id, entity_kind)
           DO UPDATE SET
               label = EXCLUDED.label,
               marking = EXCLUDED.marking,
               metadata = EXCLUDED.metadata,
               updated_at = NOW()
           RETURNING *"#,
    )
    .bind(entity_id)
    .bind(kind.as_str())
    .bind(label)
    .bind(marking)
    .bind(metadata)
    .fetch_one(db)
    .await
}

async fn get_node_record(
    db: &PgPool,
    entity_id: Uuid,
    kind: NodeKind,
) -> Result<Option<LineageNodeRecord>, sqlx::Error> {
    sqlx::query_as::<_, LineageNodeRecord>(
        r#"SELECT entity_id, entity_kind, label, marking, metadata
           FROM lineage_nodes
           WHERE entity_id = $1 AND entity_kind = $2"#,
    )
    .bind(entity_id)
    .bind(kind.as_str())
    .fetch_optional(db)
    .await
}

async fn ensure_placeholder_workflow(db: &PgPool, workflow_id: Uuid) -> Result<(), sqlx::Error> {
    let existing = get_node_record(db, workflow_id, NodeKind::Workflow).await?;
    if existing.is_some() {
        return Ok(());
    }

    let _ = upsert_node(
        db,
        workflow_id,
        NodeKind::Workflow,
        &synthetic_label(NodeKind::Workflow, workflow_id),
        "public",
        json!({ "placeholder": true }),
    )
    .await?;
    Ok(())
}

async fn load_lineage_nodes(
    db: &PgPool,
) -> Result<HashMap<NodeKey, LineageNodeRecord>, sqlx::Error> {
    let rows = sqlx::query_as::<_, LineageNodeRecord>(
        r#"SELECT entity_id, entity_kind, label, marking, metadata
           FROM lineage_nodes"#,
    )
    .fetch_all(db)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|row| {
            NodeKind::parse(&row.entity_kind).map(|kind| {
                (
                    NodeKey {
                        id: row.entity_id,
                        kind,
                    },
                    row,
                )
            })
        })
        .collect())
}

async fn load_lineage_relations(db: &PgPool) -> Result<Vec<LineageRelationRecord>, sqlx::Error> {
    let mut relations = sqlx::query_as::<_, LineageRelationRecord>(
        r#"SELECT
               id, source_id, source_kind, target_id, target_kind, relation_kind,
               pipeline_id, workflow_id, node_id, step_id, effective_marking, metadata
           FROM lineage_relations"#,
    )
    .fetch_all(db)
    .await?;

    let legacy = sqlx::query_as::<_, LineageEdge>("SELECT * FROM lineage_edges")
        .fetch_all(db)
        .await?;
    relations.extend(legacy.into_iter().map(|edge| LineageRelationRecord {
        id: edge.id,
        source_id: edge.source_dataset_id,
        source_kind: "dataset".to_string(),
        target_id: edge.target_dataset_id,
        target_kind: "dataset".to_string(),
        relation_kind: "derives".to_string(),
        pipeline_id: edge.pipeline_id,
        workflow_id: None,
        node_id: edge.node_id,
        step_id: None,
        effective_marking: "public".to_string(),
        metadata: json!({ "legacy": true }),
    }));

    Ok(relations)
}

fn build_graph(
    nodes: &HashMap<NodeKey, LineageNodeRecord>,
    relations: &[LineageRelationRecord],
    allowed_nodes: Option<&HashSet<NodeKey>>,
) -> LineageGraph {
    let mut graph_nodes = BTreeMap::new();
    let mut graph_edges = Vec::new();

    for relation in relations {
        let Some(source_kind) = NodeKind::parse(&relation.source_kind) else {
            continue;
        };
        let Some(target_kind) = NodeKind::parse(&relation.target_kind) else {
            continue;
        };
        let source_key = NodeKey {
            id: relation.source_id,
            kind: source_kind,
        };
        let target_key = NodeKey {
            id: relation.target_id,
            kind: target_kind,
        };

        if let Some(allowed) = allowed_nodes {
            if !allowed.contains(&source_key) || !allowed.contains(&target_key) {
                continue;
            }
        }

        if let Some(node) = build_node_view(source_key, nodes) {
            graph_nodes
                .entry((node.kind.clone(), node.id))
                .or_insert(node);
        }
        if let Some(node) = build_node_view(target_key, nodes) {
            graph_nodes
                .entry((node.kind.clone(), node.id))
                .or_insert(node);
        }

        graph_edges.push(LineageGraphEdge {
            id: relation.id,
            source: relation.source_id,
            source_kind: relation.source_kind.clone(),
            target: relation.target_id,
            target_kind: relation.target_kind.clone(),
            relation_kind: relation.relation_kind.clone(),
            pipeline_id: relation.pipeline_id,
            workflow_id: relation.workflow_id,
            node_id: relation.node_id.clone(),
            step_id: relation.step_id.clone(),
            effective_marking: relation.effective_marking.clone(),
            metadata: relation.metadata.clone(),
        });
    }

    LineageGraph {
        nodes: graph_nodes.into_values().collect(),
        edges: graph_edges,
    }
}

fn build_node_view(
    key: NodeKey,
    nodes: &HashMap<NodeKey, LineageNodeRecord>,
) -> Option<LineageNode> {
    if let Some(record) = nodes.get(&key) {
        return Some(node_from_record(record));
    }

    Some(LineageNode {
        id: key.id,
        kind: key.kind.as_str().to_string(),
        label: synthetic_label(key.kind, key.id),
        marking: "public".to_string(),
        metadata: json!({ "synthetic": true }),
    })
}

fn node_from_record(record: &LineageNodeRecord) -> LineageNode {
    LineageNode {
        id: record.entity_id,
        kind: record.entity_kind.clone(),
        label: record.label.clone(),
        marking: record.marking.clone(),
        metadata: record.metadata.clone(),
    }
}

fn collect_connected_nodes(root: NodeKey, relations: &[LineageRelationRecord]) -> HashSet<NodeKey> {
    let mut visited = HashSet::from([root]);
    let mut queue = VecDeque::from([root]);

    while let Some(current) = queue.pop_front() {
        for relation in relations {
            let Some(source_kind) = NodeKind::parse(&relation.source_kind) else {
                continue;
            };
            let Some(target_kind) = NodeKind::parse(&relation.target_kind) else {
                continue;
            };
            let source = NodeKey {
                id: relation.source_id,
                kind: source_kind,
            };
            let target = NodeKey {
                id: relation.target_id,
                kind: target_kind,
            };
            let neighbor = if source == current {
                Some(target)
            } else if target == current {
                Some(source)
            } else {
                None
            };

            if let Some(neighbor) = neighbor {
                if visited.insert(neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }
    }

    visited
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Incoming,
    Outgoing,
}

fn bfs_paths(
    root: NodeKey,
    relations: &[LineageRelationRecord],
    direction: Direction,
) -> HashMap<NodeKey, Vec<Uuid>> {
    let mut queue = VecDeque::from([root]);
    let mut paths: HashMap<NodeKey, Vec<Uuid>> = HashMap::new();
    paths.insert(root, Vec::new());

    while let Some(current) = queue.pop_front() {
        let current_path = paths.get(&current).cloned().unwrap_or_default();
        for relation in relations {
            let Some(source_kind) = NodeKind::parse(&relation.source_kind) else {
                continue;
            };
            let Some(target_kind) = NodeKind::parse(&relation.target_kind) else {
                continue;
            };
            let source = NodeKey {
                id: relation.source_id,
                kind: source_kind,
            };
            let target = NodeKey {
                id: relation.target_id,
                kind: target_kind,
            };

            let next = match direction {
                Direction::Outgoing if source == current => Some(target),
                Direction::Incoming if target == current => Some(source),
                _ => None,
            };

            if let Some(next) = next {
                paths.entry(next).or_insert_with(|| {
                    let mut next_path = current_path.clone();
                    next_path.push(relation.id);
                    queue.push_back(next);
                    next_path
                });
            }
        }
    }

    paths
}

fn build_impact_items(
    root: NodeKey,
    paths: &HashMap<NodeKey, Vec<Uuid>>,
    nodes: &HashMap<NodeKey, LineageNodeRecord>,
    relations: &[LineageRelationRecord],
) -> Vec<LineageImpactItem> {
    let relation_index: HashMap<Uuid, &LineageRelationRecord> = relations
        .iter()
        .map(|relation| (relation.id, relation))
        .collect();

    let mut items = Vec::new();
    for (node_key, relation_ids) in paths {
        if *node_key == root {
            continue;
        }

        let Some(node) = build_node_view(*node_key, nodes) else {
            continue;
        };
        let effective_marking =
            effective_path_marking(&node.marking, relation_ids, &relation_index);
        let requires_acknowledgement = requires_marking_acknowledgement(&effective_marking);
        let metadata = merge_metadata(
            Some(&node.metadata),
            json!({
                "node_marking": node.marking,
                "effective_marking": effective_marking,
            }),
            None,
        );
        let path = relation_ids
            .iter()
            .filter_map(|relation_id| relation_index.get(relation_id))
            .map(|relation| LineagePathHop {
                source_id: relation.source_id,
                source_kind: relation.source_kind.clone(),
                target_id: relation.target_id,
                target_kind: relation.target_kind.clone(),
                relation_kind: relation.relation_kind.clone(),
                effective_marking: relation.effective_marking.clone(),
            })
            .collect();

        items.push(LineageImpactItem {
            id: node.id,
            kind: node.kind,
            label: node.label,
            distance: relation_ids.len(),
            marking: node.marking,
            effective_marking,
            requires_acknowledgement,
            metadata,
            path,
        });
    }

    items.sort_by_key(|item| (item.distance, item.kind.clone(), item.label.clone()));
    items
}

fn build_candidate(
    item: &LineageImpactItem,
    nodes: &HashMap<NodeKey, LineageNodeRecord>,
) -> LineageBuildCandidate {
    let node_key = NodeKey {
        id: item.id,
        kind: NodeKind::parse(&item.kind).unwrap_or(NodeKind::Dataset),
    };
    let status = nodes
        .get(&node_key)
        .and_then(|record| record.metadata.get("status"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let triggerable = matches!(status.as_deref(), Some("active"));

    LineageBuildCandidate {
        id: item.id,
        kind: item.kind.clone(),
        label: item.label.clone(),
        status,
        distance: item.distance,
        triggerable,
        marking: item.marking.clone(),
        effective_marking: item.effective_marking.clone(),
        requires_acknowledgement: item.requires_acknowledgement,
        blocked_reason: None,
        metadata: item.metadata.clone(),
    }
}

fn effective_path_marking(
    node_marking: &str,
    relation_ids: &[Uuid],
    relation_index: &HashMap<Uuid, &LineageRelationRecord>,
) -> String {
    max_markings(
        std::iter::once(Some(node_marking)).chain(relation_ids.iter().filter_map(|relation_id| {
            relation_index
                .get(relation_id)
                .map(|relation| Some(relation.effective_marking.as_str()))
        })),
    )
}

fn merge_metadata(base: Option<&Value>, overlay: Value, extra: Option<&Value>) -> Value {
    let mut result = base.cloned().unwrap_or_else(|| json!({}));
    merge_json(&mut result, &overlay);
    if let Some(extra) = extra {
        merge_json(&mut result, extra);
    }
    result
}

fn merge_json(target: &mut Value, patch: &Value) {
    let Value::Object(target_obj) = target else {
        *target = patch.clone();
        return;
    };
    let Value::Object(patch_obj) = patch else {
        *target = patch.clone();
        return;
    };

    for (key, value) in patch_obj {
        match (target_obj.get_mut(key), value) {
            (Some(existing @ Value::Object(_)), Value::Object(_)) => merge_json(existing, value),
            _ => {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}

fn synthetic_label(kind: NodeKind, id: Uuid) -> String {
    format!("{} {}", kind.as_str(), &id.to_string()[..8])
}

fn marking_from_dataset_tags(tags: &[String]) -> String {
    for prefix in ["marking:", "classification:"] {
        if let Some(marking) = tags
            .iter()
            .find_map(|tag| normalize_marking(tag.strip_prefix(prefix)))
        {
            return marking;
        }
    }

    if tags.iter().any(|tag| tag.eq_ignore_ascii_case("pii")) {
        "pii".to_string()
    } else if tags
        .iter()
        .any(|tag| tag.eq_ignore_ascii_case("confidential"))
    {
        "confidential".to_string()
    } else {
        "public".to_string()
    }
}

fn normalize_marking(marking: Option<&str>) -> Option<String> {
    match marking.unwrap_or("public") {
        "public" => Some("public".to_string()),
        "confidential" => Some("confidential".to_string()),
        "pii" => Some("pii".to_string()),
        _ => None,
    }
}

fn max_markings<'a, I>(markings: I) -> String
where
    I: IntoIterator<Item = Option<&'a str>>,
{
    let mut best = "public";
    let mut best_rank = 0;
    for marking in markings {
        let candidate = marking.unwrap_or("public");
        let rank = marking_rank(candidate);
        if rank > best_rank {
            best = candidate;
            best_rank = rank;
        }
    }
    best.to_string()
}

fn requires_marking_acknowledgement(marking: &str) -> bool {
    marking_rank(marking) > 0
}

fn marking_rank(marking: &str) -> u8 {
    match marking {
        "pii" => 2,
        "confidential" => 1,
        _ => 0,
    }
}

fn clearance_rank(claims: &Claims) -> u8 {
    claims
        .attribute("classification_clearance")
        .and_then(Value::as_str)
        .map(marking_rank)
        .unwrap_or(0)
}

async fn delete_workflow_relations(db: &PgPool, workflow_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"DELETE FROM lineage_relations
           WHERE workflow_id = $1
              OR (source_id = $1 AND source_kind = 'workflow')
              OR (target_id = $1 AND target_kind = 'workflow')"#,
    )
    .bind(workflow_id)
    .execute(db)
    .await?;
    Ok(())
}

async fn load_pipeline(
    state: &AppState,
    pipeline_id: Uuid,
) -> Result<Option<Pipeline>, sqlx::Error> {
    load_pipeline_by_id(&state.db, pipeline_id).await
}

async fn load_pipeline_by_id(
    db: &PgPool,
    pipeline_id: Uuid,
) -> Result<Option<Pipeline>, sqlx::Error> {
    sqlx::query_as::<_, Pipeline>("SELECT * FROM pipelines WHERE id = $1")
        .bind(pipeline_id)
        .fetch_optional(db)
        .await
}

#[cfg(test)]
mod tests {
    use super::{
        Claims, Direction, LineageNodeRecord, LineageRelationRecord, NodeKey, NodeKind, bfs_paths,
        build_candidate, build_impact_items, can_access_marking, max_markings,
    };
    use chrono::Utc;
    use serde_json::json;
    use std::collections::HashMap;
    use uuid::Uuid;

    fn claims_with_clearance(clearance: &str) -> Claims {
        Claims {
            sub: Uuid::now_v7(),
            iat: Utc::now().timestamp(),
            exp: Utc::now().timestamp() + 3600,
            iss: None,
            aud: None,
            jti: Uuid::now_v7(),
            email: "test@example.com".to_string(),
            name: "Test".to_string(),
            roles: vec!["operator".to_string()],
            permissions: vec![],
            org_id: None,
            attributes: json!({ "classification_clearance": clearance }),
            auth_methods: vec![],
            token_use: Some("access".to_string()),
            api_key_id: None,
            session_kind: None,
            session_scope: None,
        }
    }

    #[test]
    fn max_marking_prefers_stricter_value() {
        assert_eq!(
            max_markings([Some("public"), Some("confidential"), Some("pii")]),
            "pii"
        );
    }

    #[test]
    fn impact_bfs_follows_downstream_relations() {
        let source_id = Uuid::now_v7();
        let pipeline_id = Uuid::now_v7();
        let target_id = Uuid::now_v7();
        let relations = vec![
            LineageRelationRecord {
                id: Uuid::now_v7(),
                source_id,
                source_kind: "dataset".to_string(),
                target_id: pipeline_id,
                target_kind: "pipeline".to_string(),
                relation_kind: "consumes".to_string(),
                pipeline_id: Some(pipeline_id),
                workflow_id: None,
                node_id: Some("n1".to_string()),
                step_id: None,
                effective_marking: "confidential".to_string(),
                metadata: json!({}),
            },
            LineageRelationRecord {
                id: Uuid::now_v7(),
                source_id: pipeline_id,
                source_kind: "pipeline".to_string(),
                target_id,
                target_kind: "dataset".to_string(),
                relation_kind: "produces".to_string(),
                pipeline_id: Some(pipeline_id),
                workflow_id: None,
                node_id: Some("n1".to_string()),
                step_id: None,
                effective_marking: "confidential".to_string(),
                metadata: json!({}),
            },
        ];

        let root = NodeKey {
            id: source_id,
            kind: NodeKind::Dataset,
        };
        let paths = bfs_paths(root, &relations, Direction::Outgoing);

        assert!(paths.contains_key(&NodeKey {
            id: pipeline_id,
            kind: NodeKind::Pipeline,
        }));
        assert!(paths.contains_key(&NodeKey {
            id: target_id,
            kind: NodeKind::Dataset,
        }));
    }

    #[test]
    fn marking_access_honors_clearance() {
        let claims = claims_with_clearance("confidential");
        assert!(can_access_marking(&claims, "public"));
        assert!(can_access_marking(&claims, "confidential"));
        assert!(!can_access_marking(&claims, "pii"));
    }

    #[test]
    fn build_impact_items_preserves_marking_and_distance() {
        let source_id = Uuid::now_v7();
        let target_id = Uuid::now_v7();
        let root = NodeKey {
            id: source_id,
            kind: NodeKind::Dataset,
        };
        let target_key = NodeKey {
            id: target_id,
            kind: NodeKind::Workflow,
        };
        let relations = vec![LineageRelationRecord {
            id: Uuid::now_v7(),
            source_id,
            source_kind: "dataset".to_string(),
            target_id,
            target_kind: "workflow".to_string(),
            relation_kind: "consumes".to_string(),
            pipeline_id: None,
            workflow_id: Some(target_id),
            node_id: None,
            step_id: Some("step".to_string()),
            effective_marking: "pii".to_string(),
            metadata: json!({}),
        }];
        let mut nodes = HashMap::new();
        nodes.insert(
            target_key,
            LineageNodeRecord {
                entity_id: target_id,
                entity_kind: "workflow".to_string(),
                label: "Review workflow".to_string(),
                marking: "pii".to_string(),
                metadata: json!({ "status": "active" }),
            },
        );
        let mut paths = HashMap::new();
        paths.insert(root, Vec::new());
        paths.insert(target_key, vec![relations[0].id]);

        let items = build_impact_items(root, &paths, &nodes, &relations);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].distance, 1);
        assert_eq!(items[0].marking, "pii");
        assert_eq!(items[0].effective_marking, "pii");
        assert!(items[0].requires_acknowledgement);
    }

    #[test]
    fn impact_items_elevate_effective_marking_from_path() {
        let source_id = Uuid::now_v7();
        let target_id = Uuid::now_v7();
        let root = NodeKey {
            id: source_id,
            kind: NodeKind::Dataset,
        };
        let target_key = NodeKey {
            id: target_id,
            kind: NodeKind::Pipeline,
        };
        let relations = vec![LineageRelationRecord {
            id: Uuid::now_v7(),
            source_id,
            source_kind: "dataset".to_string(),
            target_id,
            target_kind: "pipeline".to_string(),
            relation_kind: "consumes".to_string(),
            pipeline_id: Some(target_id),
            workflow_id: None,
            node_id: Some("node-a".to_string()),
            step_id: None,
            effective_marking: "confidential".to_string(),
            metadata: json!({}),
        }];
        let mut nodes = HashMap::new();
        nodes.insert(
            target_key,
            LineageNodeRecord {
                entity_id: target_id,
                entity_kind: "pipeline".to_string(),
                label: "Public pipeline".to_string(),
                marking: "public".to_string(),
                metadata: json!({ "status": "active" }),
            },
        );
        let mut paths = HashMap::new();
        paths.insert(root, Vec::new());
        paths.insert(target_key, vec![relations[0].id]);

        let items = build_impact_items(root, &paths, &nodes, &relations);

        assert_eq!(items[0].marking, "public");
        assert_eq!(items[0].effective_marking, "confidential");
        assert!(items[0].requires_acknowledgement);
    }

    #[test]
    fn build_candidate_requires_acknowledgement_when_effective_marking_is_sensitive() {
        let target_id = Uuid::now_v7();
        let item = super::LineageImpactItem {
            id: target_id,
            kind: "pipeline".to_string(),
            label: "Risk scorer".to_string(),
            distance: 2,
            marking: "public".to_string(),
            effective_marking: "pii".to_string(),
            requires_acknowledgement: true,
            metadata: json!({ "status": "active" }),
            path: Vec::new(),
        };
        let mut nodes = HashMap::new();
        nodes.insert(
            NodeKey {
                id: target_id,
                kind: NodeKind::Pipeline,
            },
            LineageNodeRecord {
                entity_id: target_id,
                entity_kind: "pipeline".to_string(),
                label: "Risk scorer".to_string(),
                marking: "public".to_string(),
                metadata: json!({ "status": "active" }),
            },
        );

        let candidate = build_candidate(&item, &nodes);

        assert!(candidate.triggerable);
        assert!(candidate.requires_acknowledgement);
        assert_eq!(candidate.effective_marking, "pii");
        assert!(candidate.blocked_reason.is_none());
    }
}
