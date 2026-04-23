use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, Utc};
use cron::Schedule;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{
    domain::{
        engine::{self, ExecutionRequest, NodeResult},
        lineage,
    },
    models::{
        pipeline::{Pipeline, PipelineNode, PipelineRetryPolicy, PipelineScheduleConfig},
        run::PipelineRun,
    },
    AppState,
};

pub async fn start_pipeline_run(
    state: &AppState,
    pipeline: &Pipeline,
    started_by: Option<Uuid>,
    trigger_type: &str,
    from_node_id: Option<String>,
    retry_of_run_id: Option<Uuid>,
    attempt_number: i32,
    distributed_worker_count: usize,
    context: Value,
) -> Result<PipelineRun, String> {
    let nodes = pipeline.parsed_nodes()?;
    if nodes.is_empty() {
        return Err("pipeline must define at least one node".into());
    }

    let retry_policy = effective_retry_policy(&pipeline.parsed_retry_policy());

    let run = sqlx::query_as::<_, PipelineRun>(
        r#"INSERT INTO pipeline_runs (
               id, pipeline_id, status, trigger_type, started_by, attempt_number, started_from_node_id, retry_of_run_id, execution_context
           )
           VALUES ($1, $2, 'running', $3, $4, $5, $6, $7, $8)
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(pipeline.id)
    .bind(trigger_type)
    .bind(started_by)
    .bind(attempt_number)
    .bind(&from_node_id)
    .bind(retry_of_run_id)
    .bind(&context)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let results = engine::execute_pipeline(
        &state.query_ctx,
        &nodes,
        &ExecutionRequest {
            start_from_node: from_node_id.clone(),
            max_attempts: retry_policy.max_attempts.max(1),
            distributed_worker_count: distributed_worker_count.max(1),
        },
    )
    .await?;

    let error_message = results
        .iter()
        .find(|result| result.status == "failed")
        .and_then(|result| result.error.clone());
    let status = if error_message.is_some() { "failed" } else { "completed" };
    let node_results = serde_json::to_value(&results).unwrap_or_else(|_| json!([]));

    sqlx::query(
        r#"UPDATE pipeline_runs
           SET status = $2,
               error_message = $3,
               node_results = $4,
               finished_at = NOW()
           WHERE id = $1"#,
    )
    .bind(run.id)
    .bind(status)
    .bind(&error_message)
    .bind(&node_results)
    .execute(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    if let Err(error) = record_pipeline_lineage(&state.db, pipeline.id, &nodes, &results).await {
        tracing::warn!(pipeline_id = %pipeline.id, "pipeline lineage recording failed: {error}");
    }

    if trigger_type == "scheduled" {
        let next_run_at = compute_next_run_at(pipeline);
        sqlx::query(
            r#"UPDATE pipelines
               SET next_run_at = $2,
                   updated_at = NOW()
               WHERE id = $1"#,
        )
        .bind(pipeline.id)
        .bind(next_run_at)
        .execute(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    }

    sqlx::query_as::<_, PipelineRun>("SELECT * FROM pipeline_runs WHERE id = $1")
        .bind(run.id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| error.to_string())
}

pub async fn retry_pipeline_run(
    state: &AppState,
    pipeline: &Pipeline,
    previous_run: &PipelineRun,
    explicit_from_node_id: Option<String>,
    distributed_worker_count: usize,
) -> Result<PipelineRun, String> {
    let retry_policy = pipeline.parsed_retry_policy();
    if explicit_from_node_id.is_some() && !retry_policy.allow_partial_reexecution {
        return Err("partial re-execution is disabled for this pipeline".into());
    }

    let from_node_id = explicit_from_node_id.or_else(|| {
        if retry_policy.allow_partial_reexecution {
            first_failed_node(previous_run)
        } else {
            None
        }
    });

    start_pipeline_run(
        state,
        pipeline,
        previous_run.started_by,
        "retry",
        from_node_id,
        Some(previous_run.id),
        previous_run.attempt_number + 1,
        distributed_worker_count,
        previous_run.execution_context.clone(),
    )
    .await
}

pub async fn run_due_scheduled_pipelines(state: &AppState) -> Result<usize, String> {
    let pipelines = sqlx::query_as::<_, Pipeline>(
        r#"SELECT * FROM pipelines
           WHERE status = 'active'
             AND next_run_at IS NOT NULL
             AND next_run_at <= NOW()
           ORDER BY next_run_at ASC"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let mut triggered = 0usize;
    for pipeline in pipelines {
        let context = json!({
            "trigger": {
                "type": "scheduled",
                "scheduled_at": Utc::now(),
            }
        });

        match start_pipeline_run(
            state,
            &pipeline,
            None,
            "scheduled",
            None,
            None,
            1,
            state.distributed_pipeline_workers.max(1),
            context,
        )
        .await {
            Ok(_) => triggered += 1,
            Err(error) => tracing::warn!(pipeline_id = %pipeline.id, "scheduled pipeline run failed: {error}"),
        }
    }

    Ok(triggered)
}

pub fn compute_next_run_at(pipeline: &Pipeline) -> Option<DateTime<Utc>> {
    compute_next_run_at_from_parts(&pipeline.status, &pipeline.schedule())
}

pub fn compute_next_run_at_from_parts(
    status: &str,
    schedule_config: &PipelineScheduleConfig,
) -> Option<DateTime<Utc>> {
    if status != "active" || !schedule_config.enabled {
        return None;
    }

    let expression = schedule_config.cron.as_deref()?;
    let schedule = Schedule::from_str(expression).ok()?;
    schedule.upcoming(Utc).next()
}

fn effective_retry_policy(policy: &PipelineRetryPolicy) -> PipelineRetryPolicy {
    if policy.retry_on_failure {
        PipelineRetryPolicy {
            max_attempts: policy.max_attempts.max(1),
            retry_on_failure: true,
            allow_partial_reexecution: policy.allow_partial_reexecution,
        }
    } else {
        PipelineRetryPolicy {
            max_attempts: 1,
            retry_on_failure: false,
            allow_partial_reexecution: policy.allow_partial_reexecution,
        }
    }
}

fn first_failed_node(run: &PipelineRun) -> Option<String> {
    let results = run.node_results.clone()?;
    let parsed: Vec<NodeResult> = serde_json::from_value(results).ok()?;
    parsed
        .into_iter()
        .find(|result| result.status == "failed")
        .map(|result| result.node_id)
}

async fn record_pipeline_lineage(
    db: &sqlx::PgPool,
    pipeline_id: Uuid,
    nodes: &[PipelineNode],
    results: &[NodeResult],
) -> Result<(), sqlx::Error> {
    let completed_nodes: HashSet<&str> = results
        .iter()
        .filter(|result| result.status == "completed")
        .map(|result| result.node_id.as_str())
        .collect();

    for node in nodes {
        if !completed_nodes.contains(node.id.as_str()) {
            continue;
        }

        let Some(target_dataset_id) = node.output_dataset_id else {
            continue;
        };

        for source_dataset_id in &node.input_dataset_ids {
            lineage::record_lineage(db, *source_dataset_id, target_dataset_id, Some(pipeline_id), Some(&node.id)).await?;
        }

        for mapping in node.column_mappings() {
            let Some(source_dataset_id) = mapping
                .source_dataset_id
                .or_else(|| node.input_dataset_ids.first().copied())
            else {
                continue;
            };

            lineage::record_column_lineage(
                db,
                source_dataset_id,
                &mapping.source_column,
                target_dataset_id,
                &mapping.target_column,
                Some(pipeline_id),
                Some(&node.id),
            )
            .await?;
        }

        if node.transform_type == "passthrough" {
            let Some(source_dataset_id) = node.input_dataset_ids.first().copied() else {
                continue;
            };

            for column in identity_columns(node) {
                lineage::record_column_lineage(
                    db,
                    source_dataset_id,
                    &column,
                    target_dataset_id,
                    &column,
                    Some(pipeline_id),
                    Some(&node.id),
                )
                .await?;
            }
        }
    }

    Ok(())
}

fn identity_columns(node: &PipelineNode) -> Vec<String> {
    node.config
        .get("identity_columns")
        .or_else(|| node.config.get("columns"))
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}