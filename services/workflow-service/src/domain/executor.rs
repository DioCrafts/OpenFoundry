use std::str::FromStr;

use auth_middleware::{
    claims::Claims,
    jwt::{build_access_claims, encode_token},
};
use chrono::{DateTime, Utc};
use cron::Schedule;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{branching, lineage},
    models::{
        approval::WorkflowApproval,
        execution::WorkflowRun,
        workflow::{WorkflowDefinition, WorkflowStep},
    },
};

#[derive(Debug, Clone, Deserialize)]
struct ActionStepConfig {
    action_id: Uuid,
    #[serde(default)]
    target_object_id: Option<Uuid>,
    #[serde(default)]
    target_object_id_field: Option<String>,
    #[serde(default)]
    parameters: Value,
    #[serde(default)]
    justification: Option<String>,
    #[serde(default)]
    result_key: Option<String>,
    #[serde(default)]
    execution_identity: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct ApprovalProposalConfig {
    action_id: Uuid,
    #[serde(default)]
    target_object_id: Option<Uuid>,
    #[serde(default)]
    target_object_id_field: Option<String>,
    #[serde(default)]
    parameters: Value,
    #[serde(default)]
    justification: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    reasoning: Option<Value>,
    #[serde(default)]
    what_if_name: Option<String>,
    #[serde(default)]
    what_if_description: Option<String>,
    #[serde(default = "default_true")]
    auto_apply_on_approval: bool,
    #[serde(default)]
    execution_identity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredActionProposal {
    kind: String,
    action_id: Uuid,
    target_object_id: Option<Uuid>,
    parameters: Value,
    justification: Option<String>,
    summary: String,
    reasoning: Option<Value>,
    preview: Value,
    what_if_branch: Option<Value>,
    auto_apply_on_approval: bool,
    execution_identity: String,
}

#[derive(Debug, Clone)]
struct ResolvedActionInvocation {
    action_id: Uuid,
    target_object_id: Option<Uuid>,
    parameters: Value,
    justification: Option<String>,
    execution_identity: String,
    result_key: Option<String>,
}

fn default_true() -> bool {
    true
}

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

    if let Err(error) = lineage::sync_workflow_lineage(state, workflow).await {
        tracing::warn!(workflow_id = %workflow.id, "workflow lineage sync before run failed: {error}");
    }

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
    channels: Vec<String>,
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
        channels: if channels.is_empty() {
            default_notification_channels()
        } else {
            channels
        },
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
            "submit_action" => {
                let config = match parse_action_step_config(step) {
                    Ok(config) => config,
                    Err(error) => return fail_run(state, run.id, &run.context, error).await,
                };

                let invocation =
                    match resolve_action_invocation(&config, &run.context, "service", true) {
                        Ok(invocation) => invocation,
                        Err(error) => return fail_run(state, run.id, &run.context, error).await,
                    };

                let response = match execute_ontology_action_as_service(
                    state,
                    &invocation,
                    run.started_by,
                    None,
                )
                .await
                {
                    Ok(response) => response,
                    Err(error) => {
                        let mut failed_context = run.context.clone();
                        record_submitted_action(
                            &mut failed_context,
                            &step.id,
                            &invocation,
                            "failed",
                            None,
                            Some(&error),
                        );
                        return fail_run(state, run.id, &failed_context, error).await;
                    }
                };

                let mut context = run.context.clone();
                record_submitted_action(
                    &mut context,
                    &step.id,
                    &invocation,
                    "applied",
                    Some(&response),
                    None,
                );

                if let Some(result_key) = invocation.result_key.as_deref() {
                    set_value_at_path(&mut context, result_key, response.clone());
                }

                if let Some(next_step_id) = branching::resolve_next_step(step, &context) {
                    run = update_running_step(state, run.id, Some(next_step_id), &context).await?;
                } else {
                    return complete_run(state, run.id, &context).await;
                }
            }
            "notification" => {
                let title = resolve_text_value(step.config.get("title"), &run.context, &step.name);
                let message = resolve_text_value(
                    step.config.get("message"),
                    &run.context,
                    "Workflow notification",
                );
                let severity =
                    resolve_text_value(step.config.get("severity"), &run.context, "info");
                let recipient = resolve_step_user(step, &run.context).or(run.started_by);

                send_workflow_notification(
                    state,
                    recipient,
                    title,
                    message,
                    &severity,
                    extract_channels(&step.config),
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
                let title =
                    resolve_text_value(step.config.get("title"), &run.context, "Approval required");
                let instructions =
                    resolve_text_value(step.config.get("instructions"), &run.context, "");

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

                let mut waiting_context = run.context.clone();
                if existing.is_none() {
                    let proposal = match prepare_approval_proposal(
                        state,
                        workflow,
                        step,
                        run.started_by,
                        &run.context,
                    )
                    .await
                    {
                        Ok(proposal) => proposal,
                        Err(error) => return fail_run(state, run.id, &run.context, error).await,
                    };

                    if let Some(proposal) = proposal.as_ref() {
                        record_action_proposal(
                            &mut waiting_context,
                            &step.id,
                            proposal,
                            "pending_review",
                            None,
                            None,
                        );
                    }

                    let approval_payload = build_approval_payload(&run.context, proposal.as_ref());
                    let approval_message =
                        approval_notification_body(&instructions, workflow, proposal.as_ref());
                    let metadata =
                        approval_notification_metadata(workflow, run.id, step, proposal.as_ref());

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
                    .bind(&approval_payload)
                    .execute(&state.db)
                    .await
                    .map_err(|error| error.to_string())?;

                    send_workflow_notification(
                        state,
                        assigned_to,
                        title,
                        approval_message,
                        "warning",
                        extract_channels(&step.config),
                        metadata,
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
                .bind(&waiting_context)
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

async fn prepare_approval_proposal(
    state: &AppState,
    workflow: &WorkflowDefinition,
    step: &WorkflowStep,
    started_by: Option<Uuid>,
    context: &Value,
) -> Result<Option<StoredActionProposal>, String> {
    let Some(proposal_value) = step.config.get("proposal") else {
        return Ok(None);
    };

    let config = serde_json::from_value::<ApprovalProposalConfig>(proposal_value.clone())
        .map_err(|error| format!("invalid proposal config for step '{}': {error}", step.id))?;
    let invocation = resolve_action_invocation(
        &ActionStepConfig {
            action_id: config.action_id,
            target_object_id: config.target_object_id,
            target_object_id_field: config.target_object_id_field.clone(),
            parameters: config.parameters.clone(),
            justification: config.justification.clone(),
            result_key: None,
            execution_identity: config.execution_identity.clone(),
        },
        context,
        "approver",
        false,
    )?;

    let summary = config
        .summary
        .as_deref()
        .map(|template| render_string_template(template, context))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| match invocation.target_object_id {
            Some(target_object_id) => format!(
                "Review ontology action {} for object {} in workflow '{}'",
                invocation.action_id, target_object_id, workflow.name
            ),
            None => format!(
                "Review ontology action {} proposed by workflow '{}'",
                invocation.action_id, workflow.name
            ),
        });
    let reasoning = config
        .reasoning
        .as_ref()
        .map(|value| resolve_templated_value(value, context))
        .filter(|value| !value.is_null());

    let service_token = issue_service_token(state, started_by, None)?;
    let (preview, what_if_branch) = match invocation.target_object_id {
        Some(target_object_id) => {
            let branch_name = config
                .what_if_name
                .as_deref()
                .map(|value| render_string_template(value, context))
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| {
                    format!(
                        "{} proposal {}",
                        step.name,
                        Utc::now().format("%Y-%m-%d %H:%M:%S")
                    )
                });
            let branch_description = config
                .what_if_description
                .as_deref()
                .map(|value| render_string_template(value, context))
                .unwrap_or_else(|| {
                    format!(
                        "Generated by workflow '{}' run proposal step '{}'",
                        workflow.name, step.name
                    )
                });
            let branch = post_ontology_json(
                state,
                &service_token,
                &format!("/api/v1/ontology/actions/{}/what-if", invocation.action_id),
                &json!({
                    "target_object_id": target_object_id,
                    "parameters": invocation.parameters,
                    "name": branch_name,
                    "description": branch_description,
                }),
            )
            .await?;
            (
                branch.get("preview").cloned().unwrap_or(Value::Null),
                Some(branch),
            )
        }
        None => {
            let preview = post_ontology_json(
                state,
                &service_token,
                &format!("/api/v1/ontology/actions/{}/validate", invocation.action_id),
                &json!({
                    "target_object_id": Value::Null,
                    "parameters": invocation.parameters,
                }),
            )
            .await?;
            (preview.get("preview").cloned().unwrap_or(Value::Null), None)
        }
    };

    Ok(Some(StoredActionProposal {
        kind: "ontology_action".to_string(),
        action_id: invocation.action_id,
        target_object_id: invocation.target_object_id,
        parameters: invocation.parameters,
        justification: invocation.justification,
        summary,
        reasoning,
        preview,
        what_if_branch,
        auto_apply_on_approval: config.auto_apply_on_approval,
        execution_identity: invocation.execution_identity,
    }))
}

fn build_approval_payload(context: &Value, proposal: Option<&StoredActionProposal>) -> Value {
    let mut payload = json!({
        "request_context": context,
    });

    if let Some(proposal) = proposal {
        ensure_object(&mut payload);
        payload
            .as_object_mut()
            .expect("approval payload must be object")
            .insert("proposal".to_string(), json!(proposal));
    }

    payload
}

fn approval_notification_body(
    instructions: &str,
    workflow: &WorkflowDefinition,
    proposal: Option<&StoredActionProposal>,
) -> String {
    let base = if instructions.trim().is_empty() {
        format!("Workflow '{}' is waiting for approval.", workflow.name)
    } else {
        instructions.to_string()
    };

    if let Some(proposal) = proposal {
        format!("{base}\n\nProposed change: {}", proposal.summary)
    } else {
        base
    }
}

fn approval_notification_metadata(
    workflow: &WorkflowDefinition,
    run_id: Uuid,
    step: &WorkflowStep,
    proposal: Option<&StoredActionProposal>,
) -> Value {
    json!({
        "workflow_id": workflow.id,
        "workflow_run_id": run_id,
        "step_id": step.id,
        "type": "approval",
        "proposal": proposal.map(|proposal| json!({
            "kind": proposal.kind,
            "action_id": proposal.action_id,
            "target_object_id": proposal.target_object_id,
            "summary": proposal.summary,
            "what_if_branch_id": proposal
                .what_if_branch
                .as_ref()
                .and_then(|branch| branch.get("id"))
                .cloned(),
            "auto_apply_on_approval": proposal.auto_apply_on_approval,
            "execution_identity": proposal.execution_identity,
        })),
    })
}

fn parse_action_step_config(step: &WorkflowStep) -> Result<ActionStepConfig, String> {
    serde_json::from_value(step.config.clone()).map_err(|error| {
        format!(
            "invalid submit_action config for step '{}': {error}",
            step.id
        )
    })
}

fn resolve_action_invocation(
    config: &ActionStepConfig,
    context: &Value,
    default_identity: &str,
    require_service_identity: bool,
) -> Result<ResolvedActionInvocation, String> {
    let target_object_id = resolve_action_target_object_id(
        config.target_object_id,
        config.target_object_id_field.as_deref(),
        context,
    )?;
    let parameters = resolve_templated_value(&config.parameters, context);
    let justification = config
        .justification
        .as_deref()
        .map(|template| render_string_template(template, context))
        .filter(|value| !value.trim().is_empty());
    let execution_identity =
        normalize_execution_identity(config.execution_identity.as_deref(), default_identity)?;

    if require_service_identity && execution_identity != "service" {
        return Err(
            "submit_action steps currently support only execution_identity = 'service'".to_string(),
        );
    }

    Ok(ResolvedActionInvocation {
        action_id: config.action_id,
        target_object_id,
        parameters,
        justification,
        execution_identity,
        result_key: config.result_key.clone(),
    })
}

fn resolve_action_target_object_id(
    static_target: Option<Uuid>,
    target_field: Option<&str>,
    context: &Value,
) -> Result<Option<Uuid>, String> {
    if let Some(target_object_id) = static_target {
        return Ok(Some(target_object_id));
    }

    let Some(target_field) = target_field else {
        return Ok(None);
    };

    let value = context_pointer(context, target_field).ok_or_else(|| {
        format!("target_object_id_field '{target_field}' did not resolve in workflow context")
    })?;
    let raw = value.as_str().ok_or_else(|| {
        format!("target_object_id_field '{target_field}' must resolve to a UUID string")
    })?;
    Uuid::parse_str(raw).map(Some).map_err(|error| {
        format!("target_object_id_field '{target_field}' is not a valid UUID: {error}")
    })
}

fn normalize_execution_identity(
    raw: Option<&str>,
    default_identity: &str,
) -> Result<String, String> {
    let normalized = raw.unwrap_or(default_identity).trim().to_lowercase();
    match normalized.as_str() {
        "service" | "approver" => Ok(normalized),
        other => Err(format!(
            "unsupported execution_identity '{other}', expected 'service' or 'approver'"
        )),
    }
}

async fn execute_ontology_action_as_service(
    state: &AppState,
    invocation: &ResolvedActionInvocation,
    impersonated_actor_id: Option<Uuid>,
    org_id: Option<Uuid>,
) -> Result<Value, String> {
    let service_token = issue_service_token(state, impersonated_actor_id, org_id)?;
    execute_ontology_action(state, invocation, &service_token).await
}

async fn execute_ontology_action(
    state: &AppState,
    invocation: &ResolvedActionInvocation,
    bearer_token: &str,
) -> Result<Value, String> {
    post_ontology_json(
        state,
        bearer_token,
        &format!("/api/v1/ontology/actions/{}/execute", invocation.action_id),
        &json!({
            "target_object_id": invocation.target_object_id,
            "parameters": invocation.parameters,
            "justification": invocation.justification,
        }),
    )
    .await
}

async fn post_ontology_json(
    state: &AppState,
    bearer_token: &str,
    path: &str,
    body: &Value,
) -> Result<Value, String> {
    let endpoint = format!(
        "{}{}",
        state.ontology_service_url.trim_end_matches('/'),
        path
    );
    let response = state
        .http_client
        .post(endpoint)
        .header("authorization", bearer_token)
        .json(body)
        .send()
        .await
        .map_err(|error| format!("ontology request failed: {error}"))?;

    let status = response.status();
    let raw_body = response
        .text()
        .await
        .map_err(|error| format!("failed to read ontology response body: {error}"))?;
    let payload = if raw_body.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str::<Value>(&raw_body).unwrap_or_else(|_| json!({ "raw": raw_body }))
    };

    if status.is_success() {
        Ok(payload)
    } else {
        let message = payload
            .get("error")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| payload.get("details").map(|details| details.to_string()))
            .unwrap_or_else(|| payload.to_string());
        Err(format!("ontology request returned {}: {message}", status))
    }
}

fn issue_service_token(
    state: &AppState,
    impersonated_actor_id: Option<Uuid>,
    org_id: Option<Uuid>,
) -> Result<String, String> {
    let mut attributes = Map::new();
    attributes.insert(
        "service".to_string(),
        Value::String("workflow-service".to_string()),
    );
    attributes.insert(
        "classification_clearance".to_string(),
        Value::String("pii".to_string()),
    );
    if let Some(actor_id) = impersonated_actor_id {
        attributes.insert(
            "impersonated_actor_id".to_string(),
            Value::String(actor_id.to_string()),
        );
    }

    let service_claims = build_access_claims(
        &state.jwt_config,
        Uuid::now_v7(),
        "workflow-service@internal.openfoundry",
        "workflow-service",
        vec!["admin".to_string()],
        vec!["*:*".to_string()],
        org_id,
        Value::Object(attributes),
        vec!["service".to_string()],
    );
    let token = encode_token(&state.jwt_config, &service_claims)
        .map_err(|error| format!("failed to issue workflow service token: {error}"))?;
    Ok(format!("Bearer {token}"))
}

fn issue_actor_token(state: &AppState, claims: &Claims) -> Result<String, String> {
    let actor_claims = build_access_claims(
        &state.jwt_config,
        claims.sub,
        &claims.email,
        &claims.name,
        claims.roles.clone(),
        claims.permissions.clone(),
        claims.org_id,
        claims.attributes.clone(),
        claims.auth_methods.clone(),
    );
    let token = encode_token(&state.jwt_config, &actor_claims)
        .map_err(|error| format!("failed to issue workflow actor token: {error}"))?;
    Ok(format!("Bearer {token}"))
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

pub(crate) async fn fail_run(
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

fn record_submitted_action(
    context: &mut Value,
    step_id: &str,
    invocation: &ResolvedActionInvocation,
    status: &str,
    response: Option<&Value>,
    error: Option<&str>,
) {
    let entry = json!({
        "step_id": step_id,
        "status": status,
        "action_id": invocation.action_id,
        "target_object_id": invocation.target_object_id,
        "parameters": invocation.parameters,
        "justification": invocation.justification,
        "execution_identity": invocation.execution_identity,
        "response": response,
        "error": error,
        "executed_at": Utc::now(),
    });

    set_collection_entry(context, "submitted_actions", step_id, entry.clone());
    ensure_object(context);
    context
        .as_object_mut()
        .expect("context must be object")
        .insert("last_submitted_action".to_string(), entry);
}

fn record_action_proposal(
    context: &mut Value,
    step_id: &str,
    proposal: &StoredActionProposal,
    status: &str,
    response: Option<&Value>,
    error: Option<&str>,
) {
    let entry = json!({
        "step_id": step_id,
        "status": status,
        "proposal": proposal,
        "response": response,
        "error": error,
        "updated_at": Utc::now(),
    });

    set_collection_entry(context, "action_proposals", step_id, entry.clone());
    ensure_object(context);
    context
        .as_object_mut()
        .expect("context must be object")
        .insert("last_action_proposal".to_string(), entry);
}

fn set_collection_entry(context: &mut Value, collection_key: &str, item_key: &str, value: Value) {
    ensure_object(context);
    let object = context.as_object_mut().expect("context must be object");
    let collection = object
        .entry(collection_key.to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    ensure_object(collection);
    collection
        .as_object_mut()
        .expect("collection must be object")
        .insert(item_key.to_string(), value);
}

fn set_value_at_path(context: &mut Value, path: &str, value: Value) {
    if path.trim().is_empty() {
        *context = value;
        return;
    }

    ensure_object(context);
    let mut current = context;
    let mut segments = path.split('.').peekable();
    while let Some(segment) = segments.next() {
        if segments.peek().is_none() {
            ensure_object(current);
            current
                .as_object_mut()
                .expect("context object expected")
                .insert(segment.to_string(), value);
            return;
        }

        ensure_object(current);
        let object = current.as_object_mut().expect("context object expected");
        current = object
            .entry(segment.to_string())
            .or_insert_with(|| Value::Object(Map::new()));
    }
}

fn extract_channels(config: &Value) -> Vec<String> {
    config
        .get("channels")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .filter(|value| !value.trim().is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|channels| !channels.is_empty())
        .unwrap_or_else(default_notification_channels)
}

fn default_notification_channels() -> Vec<String> {
    vec!["in_app".to_string()]
}

fn resolve_text_value(value: Option<&Value>, context: &Value, default: &str) -> String {
    value
        .map(|value| resolve_templated_value(value, context))
        .and_then(|value| value_to_display_string(&value))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn render_string_template(template: &str, context: &Value) -> String {
    let mut remaining = template;
    let mut rendered = String::new();

    while let Some(start) = remaining.find("{{") {
        let (prefix, rest) = remaining.split_at(start);
        rendered.push_str(prefix);

        let Some(end) = rest.find("}}") else {
            rendered.push_str(rest);
            return rendered;
        };

        let token = rest[2..end].trim();
        if token.is_empty() {
            rendered.push_str(rest);
            return rendered;
        }

        if let Some(value) = context_pointer(context, token) {
            rendered.push_str(&value_to_display_string(value).unwrap_or_default());
        }

        remaining = &rest[end + 2..];
    }

    rendered.push_str(remaining);
    rendered
}

fn resolve_templated_value(value: &Value, context: &Value) -> Value {
    match value {
        Value::Object(object)
            if object.len() == 1 && object.get("$context").and_then(Value::as_str).is_some() =>
        {
            let path = object
                .get("$context")
                .and_then(Value::as_str)
                .expect("checked above");
            context_pointer(context, path)
                .cloned()
                .unwrap_or(Value::Null)
        }
        Value::Object(object)
            if object.len() == 1 && object.get("$template").and_then(Value::as_str).is_some() =>
        {
            let template = object
                .get("$template")
                .and_then(Value::as_str)
                .expect("checked above");
            Value::String(render_string_template(template, context))
        }
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), resolve_templated_value(value, context)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(
            values
                .iter()
                .map(|value| resolve_templated_value(value, context))
                .collect(),
        ),
        Value::String(template) => {
            if let Some(path) = exact_context_reference(template) {
                context_pointer(context, path)
                    .cloned()
                    .unwrap_or(Value::Null)
            } else if template.contains("{{") && template.contains("}}") {
                Value::String(render_string_template(template, context))
            } else {
                Value::String(template.clone())
            }
        }
        other => other.clone(),
    }
}

fn exact_context_reference(template: &str) -> Option<&str> {
    let trimmed = template.trim();
    let without_prefix = trimmed.strip_prefix("{{")?;
    let without_suffix = without_prefix.strip_suffix("}}")?;
    if without_suffix.contains("{{") || without_suffix.contains("}}") {
        None
    } else {
        Some(without_suffix.trim())
    }
}

fn value_to_display_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(raw) => Some(raw.clone()),
        Value::Bool(raw) => Some(raw.to_string()),
        Value::Number(raw) => Some(raw.to_string()),
        other => serde_json::to_string(other).ok(),
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

pub(crate) async fn apply_approval_proposal(
    state: &AppState,
    context: &mut Value,
    approval_step_id: &str,
    approval_payload: &Value,
    claims: &Claims,
) -> Result<Value, String> {
    let Some(proposal_value) = approval_payload.get("proposal") else {
        return Ok(Value::Null);
    };
    let proposal = serde_json::from_value::<StoredActionProposal>(proposal_value.clone())
        .map_err(|error| format!("invalid stored proposal payload: {error}"))?;

    if !proposal.auto_apply_on_approval {
        record_action_proposal(
            context,
            approval_step_id,
            &proposal,
            "approved_pending_manual_apply",
            None,
            None,
        );
        return Ok(Value::Null);
    }

    let invocation = ResolvedActionInvocation {
        action_id: proposal.action_id,
        target_object_id: proposal.target_object_id,
        parameters: proposal.parameters.clone(),
        justification: proposal.justification.clone(),
        execution_identity: proposal.execution_identity.clone(),
        result_key: None,
    };

    let response = match proposal.execution_identity.as_str() {
        "approver" => {
            let actor_token = issue_actor_token(state, claims)?;
            execute_ontology_action(state, &invocation, &actor_token).await
        }
        _ => {
            execute_ontology_action_as_service(state, &invocation, Some(claims.sub), claims.org_id)
                .await
        }
    };

    match response {
        Ok(response) => {
            record_action_proposal(
                context,
                approval_step_id,
                &proposal,
                "applied",
                Some(&response),
                None,
            );
            record_submitted_action(
                context,
                approval_step_id,
                &invocation,
                "applied",
                Some(&response),
                None,
            );
            Ok(response)
        }
        Err(error) => {
            record_action_proposal(
                context,
                approval_step_id,
                &proposal,
                "failed",
                None,
                Some(&error),
            );
            record_submitted_action(
                context,
                approval_step_id,
                &invocation,
                "failed",
                None,
                Some(&error),
            );
            Err(error)
        }
    }
}

pub(crate) fn upsert_approval_review_payload(
    payload: &Value,
    decision: &str,
    decided_by: Uuid,
    comment: Option<&str>,
    decision_payload: &Value,
) -> Value {
    let mut next_payload = payload.clone();
    ensure_object(&mut next_payload);
    next_payload
        .as_object_mut()
        .expect("approval payload must be object")
        .insert(
            "decision_review".to_string(),
            json!({
                "decision": decision,
                "decided_by": decided_by,
                "comment": comment,
                "payload": decision_payload,
                "decided_at": Utc::now(),
            }),
        );
    next_payload
}

pub(crate) fn annotate_approval_proposal_execution(
    payload: &Value,
    status: &str,
    response: Option<&Value>,
    error: Option<&str>,
) -> Value {
    let mut next_payload = payload.clone();
    ensure_object(&mut next_payload);
    next_payload
        .as_object_mut()
        .expect("approval payload must be object")
        .insert(
            "proposal_execution".to_string(),
            json!({
                "status": status,
                "response": response,
                "error": error,
                "updated_at": Utc::now(),
            }),
        );
    next_payload
}

fn ensure_object(value: &mut Value) {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_context_references_inside_values() {
        let context = json!({
            "event": {
                "object_id": "01968522-2f75-7f3a-bb5d-000000000099",
                "severity": "critical",
                "score": 98,
                "labels": ["urgent", "triage"],
            }
        });

        let resolved = resolve_templated_value(
            &json!({
                "target": "{{event.object_id}}",
                "severity": { "$context": "event.severity" },
                "score": { "$context": "event.score" },
                "message": "Escalate {{event.object_id}}",
                "labels": { "$context": "event.labels" },
            }),
            &context,
        );

        assert_eq!(
            resolved,
            json!({
                "target": "01968522-2f75-7f3a-bb5d-000000000099",
                "severity": "critical",
                "score": 98,
                "message": "Escalate 01968522-2f75-7f3a-bb5d-000000000099",
                "labels": ["urgent", "triage"],
            })
        );
    }

    #[test]
    fn extracts_notification_channels_with_default() {
        assert_eq!(extract_channels(&json!({})), vec!["in_app".to_string()]);
        assert_eq!(
            extract_channels(&json!({ "channels": ["email", "slack"] })),
            vec!["email".to_string(), "slack".to_string()]
        );
    }

    #[test]
    fn approval_payload_keeps_request_and_proposal() {
        let proposal = StoredActionProposal {
            kind: "ontology_action".to_string(),
            action_id: Uuid::nil(),
            target_object_id: None,
            parameters: json!({ "status": "approved" }),
            justification: Some("Because {{event.reason}}".to_string()),
            summary: "Escalate incident".to_string(),
            reasoning: Some(json!("High severity")),
            preview: json!({ "after": { "status": "approved" } }),
            what_if_branch: None,
            auto_apply_on_approval: true,
            execution_identity: "approver".to_string(),
        };

        let payload = build_approval_payload(
            &json!({ "event": { "reason": "risk spike" } }),
            Some(&proposal),
        );

        assert_eq!(
            payload
                .get("proposal")
                .and_then(|value| value.get("summary")),
            Some(&json!("Escalate incident"))
        );
        assert!(payload.get("request_context").is_some());
    }
}
