use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{AppState, models::workflow::WorkflowDefinition};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLineageSyncRequest {
    pub workflow: WorkflowLineageNode,
    pub relations: Vec<WorkflowLineageRelation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLineageNode {
    pub id: Uuid,
    pub label: String,
    pub status: String,
    pub trigger_type: String,
    pub marking: Option<String>,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowLineageRelation {
    pub source_id: Uuid,
    pub source_kind: String,
    pub target_id: Uuid,
    pub target_kind: String,
    pub relation_kind: String,
    pub step_id: Option<String>,
    pub metadata: Value,
    pub marking: Option<String>,
}

const INCOMING_DATASET_KEYS: &[&str] = &[
    "dataset_id",
    "dataset_ids",
    "input_dataset_id",
    "input_dataset_ids",
    "source_dataset_id",
    "source_dataset_ids",
];
const OUTGOING_DATASET_KEYS: &[&str] = &[
    "output_dataset_id",
    "output_dataset_ids",
    "target_dataset_id",
    "target_dataset_ids",
];
const PIPELINE_KEYS: &[&str] = &[
    "pipeline_id",
    "pipeline_ids",
    "target_pipeline_id",
    "target_pipeline_ids",
];
const WORKFLOW_KEYS: &[&str] = &[
    "workflow_id",
    "workflow_ids",
    "target_workflow_id",
    "target_workflow_ids",
];

pub fn build_workflow_lineage_snapshot(
    workflow: &WorkflowDefinition,
) -> Result<WorkflowLineageSyncRequest, String> {
    let steps = workflow.parsed_steps()?;
    let workflow_marking = extract_marking(&workflow.trigger_config);

    let mut relations = Vec::new();
    relations.extend(extract_lineage_relations(
        workflow.id,
        None,
        &workflow.trigger_config,
        "triggers",
    ));

    for step in &steps {
        relations.extend(extract_lineage_relations(
            workflow.id,
            Some(step.id.as_str()),
            &step.config,
            step.step_type.as_str(),
        ));
    }

    Ok(WorkflowLineageSyncRequest {
        workflow: WorkflowLineageNode {
            id: workflow.id,
            label: workflow.name.clone(),
            status: workflow.status.clone(),
            trigger_type: workflow.trigger_type.clone(),
            marking: workflow_marking,
            metadata: json!({
                "description": workflow.description,
                "status": workflow.status,
                "trigger_type": workflow.trigger_type,
                "next_run_at": workflow.next_run_at,
                "last_triggered_at": workflow.last_triggered_at,
            }),
        },
        relations,
    })
}

pub async fn sync_workflow_lineage(
    state: &AppState,
    workflow: &WorkflowDefinition,
) -> Result<(), String> {
    let payload = build_workflow_lineage_snapshot(workflow)?;
    let endpoint = format!(
        "{}/internal/lineage/workflows/{}/sync",
        state.pipeline_service_url.trim_end_matches('/'),
        workflow.id
    );

    state
        .http_client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|error| {
            format!("failed to reach pipeline-service for workflow lineage sync: {error}")
        })?
        .error_for_status()
        .map_err(|error| format!("pipeline-service rejected workflow lineage sync: {error}"))?;

    Ok(())
}

pub async fn delete_workflow_lineage(state: &AppState, workflow_id: Uuid) -> Result<(), String> {
    let endpoint = format!(
        "{}/internal/lineage/workflows/{}",
        state.pipeline_service_url.trim_end_matches('/'),
        workflow_id
    );

    state
        .http_client
        .delete(endpoint)
        .send()
        .await
        .map_err(|error| {
            format!("failed to reach pipeline-service for workflow lineage delete: {error}")
        })?
        .error_for_status()
        .map_err(|error| format!("pipeline-service rejected workflow lineage delete: {error}"))?;

    Ok(())
}

fn extract_lineage_relations(
    workflow_id: Uuid,
    step_id: Option<&str>,
    config: &Value,
    relation_hint: &str,
) -> Vec<WorkflowLineageRelation> {
    let mut relations = Vec::new();
    let explicit_marking = extract_marking(config);
    let lineage = config.get("lineage").cloned().unwrap_or_else(|| json!({}));

    for dataset_id in extract_uuid_values(
        lineage
            .get("inputs")
            .and_then(|value| value.get("datasets")),
        INCOMING_DATASET_KEYS,
        config,
    ) {
        relations.push(WorkflowLineageRelation {
            source_id: dataset_id,
            source_kind: "dataset".to_string(),
            target_id: workflow_id,
            target_kind: "workflow".to_string(),
            relation_kind: normalize_relation_kind(relation_hint, "consumes"),
            step_id: step_id.map(str::to_string),
            metadata: json!({ "scope": step_scope(step_id) }),
            marking: explicit_marking.clone(),
        });
    }

    for dataset_id in extract_uuid_values(
        lineage
            .get("outputs")
            .and_then(|value| value.get("datasets")),
        OUTGOING_DATASET_KEYS,
        config,
    ) {
        relations.push(WorkflowLineageRelation {
            source_id: workflow_id,
            source_kind: "workflow".to_string(),
            target_id: dataset_id,
            target_kind: "dataset".to_string(),
            relation_kind: "produces".to_string(),
            step_id: step_id.map(str::to_string),
            metadata: json!({ "scope": step_scope(step_id) }),
            marking: explicit_marking.clone(),
        });
    }

    for pipeline_id in extract_uuid_values(
        lineage
            .get("outputs")
            .and_then(|value| value.get("pipelines")),
        PIPELINE_KEYS,
        config,
    ) {
        relations.push(WorkflowLineageRelation {
            source_id: workflow_id,
            source_kind: "workflow".to_string(),
            target_id: pipeline_id,
            target_kind: "pipeline".to_string(),
            relation_kind: "triggers".to_string(),
            step_id: step_id.map(str::to_string),
            metadata: json!({ "scope": step_scope(step_id) }),
            marking: explicit_marking.clone(),
        });
    }

    for workflow_target_id in extract_uuid_values(
        lineage
            .get("outputs")
            .and_then(|value| value.get("workflows")),
        WORKFLOW_KEYS,
        config,
    ) {
        if workflow_target_id == workflow_id {
            continue;
        }

        relations.push(WorkflowLineageRelation {
            source_id: workflow_id,
            source_kind: "workflow".to_string(),
            target_id: workflow_target_id,
            target_kind: "workflow".to_string(),
            relation_kind: "triggers".to_string(),
            step_id: step_id.map(str::to_string),
            metadata: json!({ "scope": step_scope(step_id) }),
            marking: explicit_marking.clone(),
        });
    }

    for pipeline_id in extract_uuid_values(
        lineage
            .get("inputs")
            .and_then(|value| value.get("pipelines")),
        PIPELINE_KEYS,
        config,
    ) {
        relations.push(WorkflowLineageRelation {
            source_id: pipeline_id,
            source_kind: "pipeline".to_string(),
            target_id: workflow_id,
            target_kind: "workflow".to_string(),
            relation_kind: "triggers".to_string(),
            step_id: step_id.map(str::to_string),
            metadata: json!({ "scope": step_scope(step_id) }),
            marking: explicit_marking.clone(),
        });
    }

    relations
}

fn extract_uuid_values(
    explicit: Option<&Value>,
    fallback_keys: &[&str],
    config: &Value,
) -> Vec<Uuid> {
    let mut ids = BTreeSet::new();

    if let Some(value) = explicit {
        collect_uuids(value, &mut ids);
    }

    for key in fallback_keys {
        if let Some(value) = config.get(*key) {
            collect_uuids(value, &mut ids);
        }
    }

    ids.into_iter().collect()
}

fn collect_uuids(value: &Value, target: &mut BTreeSet<Uuid>) {
    match value {
        Value::String(raw) => {
            if let Ok(uuid) = Uuid::parse_str(raw) {
                target.insert(uuid);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_uuids(item, target);
            }
        }
        _ => {}
    }
}

fn extract_marking(config: &Value) -> Option<String> {
    config
        .get("lineage")
        .and_then(|value| value.get("marking"))
        .and_then(Value::as_str)
        .or_else(|| config.get("marking").and_then(Value::as_str))
        .map(str::to_string)
}

fn normalize_relation_kind(relation_hint: &str, fallback: &str) -> String {
    match relation_hint {
        "action" | "manual" | "event" | "webhook" | "cron" => fallback.to_string(),
        "approval" => "gates".to_string(),
        "notification" => "notifies".to_string(),
        _ => fallback.to_string(),
    }
}

fn step_scope(step_id: Option<&str>) -> &'static str {
    if step_id.is_some() { "step" } else { "trigger" }
}

#[cfg(test)]
mod tests {
    use super::build_workflow_lineage_snapshot;
    use crate::models::workflow::{WorkflowDefinition, WorkflowStep};
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    #[test]
    fn workflow_snapshot_extracts_dataset_and_pipeline_relations() {
        let workflow = WorkflowDefinition {
            id: Uuid::now_v7(),
            name: "Sync Orders".to_string(),
            description: "".to_string(),
            owner_id: Uuid::now_v7(),
            status: "active".to_string(),
            trigger_type: "event".to_string(),
            trigger_config: json!({
                "event_name": "dataset.updated",
                "dataset_id": Uuid::now_v7().to_string(),
                "marking": "confidential",
            }),
            steps: serde_json::to_value(vec![WorkflowStep {
                id: "fanout".to_string(),
                name: "Fanout".to_string(),
                step_type: "action".to_string(),
                description: "".to_string(),
                config: json!({
                    "pipeline_id": Uuid::now_v7().to_string(),
                    "output_dataset_id": Uuid::now_v7().to_string(),
                }),
                next_step_id: None,
                branches: Vec::new(),
            }])
            .expect("steps"),
            webhook_secret: None,
            next_run_at: None,
            last_triggered_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let snapshot = build_workflow_lineage_snapshot(&workflow).expect("snapshot");

        assert_eq!(snapshot.workflow.marking.as_deref(), Some("confidential"));
        assert!(snapshot.relations.iter().any(
            |relation| relation.source_kind == "dataset" && relation.target_kind == "workflow"
        ));
        assert!(snapshot.relations.iter().any(
            |relation| relation.source_kind == "workflow" && relation.target_kind == "pipeline"
        ));
        assert!(snapshot.relations.iter().any(
            |relation| relation.source_kind == "workflow" && relation.target_kind == "dataset"
        ));
    }
}
