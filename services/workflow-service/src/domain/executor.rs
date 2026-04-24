use std::str::FromStr;

use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::Serialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::branching,
    models::{
        approval::WorkflowApproval,
        execution::WorkflowRun,
        workflow::{WorkflowDefinition, WorkflowStep},
    },
};

pub async fn execute_workflow_run(
    state: &AppState,
    workflow: &WorkflowDefinition,
    trigger_type: &str,
    started_by: Option<Uuid>,
    context: Value,
) -> Result<WorkflowRun, String> {
    let steps = workflow.parsed_steps()?;
    let Some(first_step) = steps.first() else {
        return Err("workflow must define at least one step".to_string());
    };

    let run = sqlx::query_as::<_, WorkflowRun>(
        r#"INSERT INTO workflow_runs (id, workflow_id, trigger_type, status, started_by, current_step_id, context)
           VALUES ($1, $2, $3, 'running', $4, $5, $6)
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(workflow.id)
    .bind(trigger_type)
    .bind(started_by)
    .bind(&first_step.id)
    .bind(&context)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    mark_workflow_triggered(state, workflow).await?;
    continue_run(state, workflow, run).await
}

pub async fn continue_after_approval(
    state: &AppState,
    workflow: &WorkflowDefinition,
    mut run: WorkflowRun,
    decision: &str,
    step: &WorkflowStep,
) -> Result<WorkflowRun, String> {
    let next = approval_next_step(step, decision, &run.context);

    if let Some(next_step_id) = next {
        run = sqlx::query_as::<_, WorkflowRun>(
            r#"UPDATE workflow_runs
               SET status = 'running', current_step_id = $2, context = $3, error_message = NULL
               WHERE id = $1
               RETURNING *"#,
        )
        .bind(run.id)
        .bind(next_step_id)
        .bind(&run.context)
        .fetch_one(&state.db)
        .await
        .map_err(|error| error.to_string())?;

        continue_run(state, workflow, run).await
    } else {
        complete_run(state, run.id, &run.context).await
    }
}

pub async fn run_due_cron_workflows(state: &AppState) -> Result<usize, String> {
    let workflows = sqlx::query_as::<_, WorkflowDefinition>(
        r#"SELECT * FROM workflows
           WHERE status = 'active'
             AND trigger_type = 'cron'
             AND next_run_at IS NOT NULL
             AND next_run_at <= NOW()
           ORDER BY next_run_at ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let mut triggered = 0usize;
    for workflow in workflows {
        let context = json!({
            "trigger": {
                "type": "cron",
                "scheduled_at": Utc::now(),
            }
        });

        match execute_workflow_run(state, &workflow, "cron", None, context).await {
            Ok(_) => triggered += 1,
            Err(error) => {
                tracing::warn!(workflow_id = %workflow.id, "cron workflow trigger failed: {error}")
            }
        }
    }

    Ok(triggered)
}

pub fn compute_next_run_at(workflow: &WorkflowDefinition) -> Option<DateTime<Utc>> {
    if workflow.trigger_type != "cron" {
        return None;
    }

    let expression = workflow.trigger_config.get("cron")?.as_str()?;
    let schedule = Schedule::from_str(expression).ok()?;
    schedule.upcoming(Utc).next()
}

pub async fn send_workflow_notification(
    state: &AppState,
    user_id: Option<Uuid>,
    title: impl Into<String>,
    body: impl Into<String>,
    severity: &str,
    metadata: Value,
) {
    let title = title.into();
    let body = body.into();
    let endpoint = format!(
        "{}/internal/notifications",
        state.notification_service_url.trim_end_matches('/')
    );

    let payload = InternalNotificationRequest {
        user_id,
        title,
        body,
        severity: severity.to_string(),
        category: "workflow".to_string(),
        channels: vec!["in_app".to_string()],
        metadata,
    };

    if let Err(error) = state.http_client.post(endpoint).json(&payload).send().await {
        tracing::warn!("workflow notification dispatch failed: {error}");
    }
}

async fn continue_run(
    state: &AppState,
    workflow: &WorkflowDefinition,
    mut run: WorkflowRun,
) -> Result<WorkflowRun, String> {
    let steps = workflow.parsed_steps()?;

    loop {
        let Some(step_id) = run.current_step_id.clone() else {
            return complete_run(state, run.id, &run.context).await;
        };

        let Some(step) = steps.iter().find(|candidate| candidate.id == step_id) else {
            return fail_run(
                state,
                run.id,
                &run.context,
                format!("step '{step_id}' not found"),
            )
            .await;
        };

        match step.step_type.as_str() {
            "action" => {
                let mut context = run.context.clone();
                apply_action(step, &mut context);
                if let Some(next_step_id) = branching::resolve_next_step(step, &context) {
                    run = update_running_step(state, run.id, Some(next_step_id), &context).await?;
                } else {
                    return complete_run(state, run.id, &context).await;
                }
            }
            "notification" => {
                let title = step
                    .config
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(&step.name)
                    .to_string();
                let message = step
                    .config
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Workflow notification")
                    .to_string();
                let recipient = resolve_step_user(step, &run.context).or(run.started_by);

                send_workflow_notification(
                    state,
                    recipient,
                    title,
                    message,
                    step.config
                        .get("severity")
                        .and_then(Value::as_str)
                        .unwrap_or("info"),
                    json!({
                        "workflow_id": workflow.id,
                        "workflow_run_id": run.id,
                        "step_id": step.id,
                    }),
                )
                .await;

                if let Some(next_step_id) = branching::resolve_next_step(step, &run.context) {
                    run = update_running_step(state, run.id, Some(next_step_id), &run.context)
                        .await?;
                } else {
                    return complete_run(state, run.id, &run.context).await;
                }
            }
            "approval" => {
                let assigned_to = resolve_step_user(step, &run.context).or(run.started_by);
                let title = step
                    .config
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or("Approval required")
                    .to_string();
                let instructions = step
                    .config
                    .get("instructions")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();

                let existing = sqlx::query_as::<_, WorkflowApproval>(
                    r#"SELECT * FROM workflow_approvals
                       WHERE workflow_run_id = $1 AND step_id = $2 AND status = 'pending'
                       ORDER BY requested_at DESC
                       LIMIT 1"#,
                )
                .bind(run.id)
                .bind(&step.id)
                .fetch_optional(&state.db)
                .await
                .map_err(|error| error.to_string())?;

                if existing.is_none() {
                    let _ = sqlx::query(
                        r#"INSERT INTO workflow_approvals (
                               id, workflow_id, workflow_run_id, step_id, title, instructions, assigned_to, payload
                           )
                           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
                    )
                    .bind(Uuid::now_v7())
                    .bind(workflow.id)
                    .bind(run.id)
                    .bind(&step.id)
                    .bind(&title)
                    .bind(&instructions)
                    .bind(assigned_to)
                    .bind(&run.context)
                    .execute(&state.db)
                    .await
                    .map_err(|error| error.to_string())?;

                    send_workflow_notification(
                        state,
                        assigned_to,
                        title,
                        if instructions.is_empty() {
                            format!("Workflow '{}' is waiting for approval.", workflow.name)
                        } else {
                            instructions.clone()
                        },
                        "warning",
                        json!({
                            "workflow_id": workflow.id,
                            "workflow_run_id": run.id,
                            "step_id": step.id,
                            "type": "approval",
                        }),
                    )
                    .await;
                }

                let run = sqlx::query_as::<_, WorkflowRun>(
                    r#"UPDATE workflow_runs
                       SET status = 'waiting_approval', current_step_id = $2, context = $3
                       WHERE id = $1
                       RETURNING *"#,
                )
                .bind(run.id)
                .bind(&step.id)
                .bind(&run.context)
                .fetch_one(&state.db)
                .await
                .map_err(|error| error.to_string())?;

                return Ok(run);
            }
            other => {
                return fail_run(
                    state,
                    run.id,
                    &run.context,
                    format!("unsupported step type '{other}'"),
                )
                .await;
            }
        }
    }
}

async fn update_running_step(
    state: &AppState,
    run_id: Uuid,
    next_step_id: Option<String>,
    context: &Value,
) -> Result<WorkflowRun, String> {
    sqlx::query_as::<_, WorkflowRun>(
        r#"UPDATE workflow_runs
           SET status = 'running', current_step_id = $2, context = $3, error_message = NULL
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(run_id)
    .bind(next_step_id)
    .bind(context)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())
}

async fn complete_run(
    state: &AppState,
    run_id: Uuid,
    context: &Value,
) -> Result<WorkflowRun, String> {
    sqlx::query_as::<_, WorkflowRun>(
        r#"UPDATE workflow_runs
           SET status = 'completed', current_step_id = NULL, context = $2, finished_at = NOW(), error_message = NULL
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(run_id)
    .bind(context)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())
}

async fn fail_run(
    state: &AppState,
    run_id: Uuid,
    context: &Value,
    error_message: String,
) -> Result<WorkflowRun, String> {
    sqlx::query_as::<_, WorkflowRun>(
        r#"UPDATE workflow_runs
           SET status = 'failed', context = $2, error_message = $3, finished_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(run_id)
    .bind(context)
    .bind(error_message)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())
}

async fn mark_workflow_triggered(
    state: &AppState,
    workflow: &WorkflowDefinition,
) -> Result<(), String> {
    let next_run_at = compute_next_run_at(workflow);
    sqlx::query(
        r#"UPDATE workflows
           SET last_triggered_at = NOW(), next_run_at = $2, updated_at = NOW()
           WHERE id = $1"#,
    )
    .bind(workflow.id)
    .bind(next_run_at)
    .execute(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(())
}

fn apply_action(step: &WorkflowStep, context: &mut Value) {
    if let Some(set_values) = step.config.get("set") {
        merge_objects(context, set_values);
    }
}

fn merge_objects(target: &mut Value, patch: &Value) {
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
            (Some(existing @ Value::Object(_)), Value::Object(_)) => merge_objects(existing, value),
            _ => {
                target_obj.insert(key.clone(), value.clone());
            }
        }
    }
}

fn resolve_step_user(step: &WorkflowStep, context: &Value) -> Option<Uuid> {
    step.config
        .get("assigned_to")
        .and_then(Value::as_str)
        .and_then(|raw| Uuid::parse_str(raw).ok())
        .or_else(|| {
            step.config
                .get("assigned_to_field")
                .and_then(Value::as_str)
                .and_then(|field| context_pointer(context, field))
                .and_then(Value::as_str)
                .and_then(|raw| Uuid::parse_str(raw).ok())
        })
}

fn approval_next_step(step: &WorkflowStep, decision: &str, context: &Value) -> Option<String> {
    if decision.eq_ignore_ascii_case("approved") {
        step.config
            .get("approved_next_step_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| branching::resolve_next_step(step, context))
    } else {
        step.config
            .get("rejected_next_step_id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| branching::resolve_next_step(step, context))
    }
}

fn context_pointer<'a>(context: &'a Value, field: &str) -> Option<&'a Value> {
    let mut current = context;
    for part in field.split('.') {
        current = current.get(part)?;
    }
    Some(current)
}

#[derive(Serialize)]
struct InternalNotificationRequest {
    user_id: Option<Uuid>,
    title: String,
    body: String,
    severity: String,
    category: String,
    channels: Vec<String>,
    metadata: Value,
}

pub fn insert_approval_decision(
    context: &mut Value,
    step_id: &str,
    decision: &str,
    decided_by: Uuid,
    payload: &Value,
    comment: Option<&str>,
) {
    ensure_object(context);
    let object = context.as_object_mut().expect("context must be object");
    let approvals = object
        .entry("approvals")
        .or_insert_with(|| Value::Object(Map::new()));

    ensure_object(approvals);
    approvals
        .as_object_mut()
        .expect("approvals must be object")
        .insert(
            step_id.to_string(),
            json!({
                "decision": decision,
                "decided_by": decided_by,
                "comment": comment,
                "payload": payload,
                "decided_at": Utc::now(),
            }),
        );

    object.insert(
        "last_approval_decision".to_string(),
        json!({
            "step_id": step_id,
            "decision": decision,
            "decided_by": decided_by,
            "comment": comment,
            "payload": payload,
        }),
    );
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}
