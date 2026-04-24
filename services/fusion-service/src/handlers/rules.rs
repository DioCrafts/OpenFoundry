use axum::{
    Json,
    extract::{Path, State},
};
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
    AppState,
    models::{
        ListResponse,
        match_rule::{CreateMatchRuleRequest, MatchRule, MatchRuleRow, UpdateMatchRuleRequest},
        merge_strategy::{
            CreateMergeStrategyRequest, MergeStrategy, MergeStrategyRow, UpdateMergeStrategyRequest,
        },
    },
};

use super::{ServiceResult, bad_request, db_error, not_found};

async fn load_rule_row(
    db: &sqlx::PgPool,
    rule_id: Uuid,
) -> Result<Option<MatchRuleRow>, sqlx::Error> {
    query_as::<_, MatchRuleRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold,
            created_at,
            updated_at
        FROM fusion_match_rules
        WHERE id = $1
        "#,
    )
    .bind(rule_id)
    .fetch_optional(db)
    .await
}

async fn load_merge_strategy_row(
    db: &sqlx::PgPool,
    strategy_id: Uuid,
) -> Result<Option<MergeStrategyRow>, sqlx::Error> {
    query_as::<_, MergeStrategyRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules,
            created_at,
            updated_at
        FROM fusion_merge_strategies
        WHERE id = $1
        "#,
    )
    .bind(strategy_id)
    .fetch_optional(db)
    .await
}

pub async fn list_rules(State(state): State<AppState>) -> ServiceResult<ListResponse<MatchRule>> {
    let rows = query_as::<_, MatchRuleRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold,
            created_at,
            updated_at
        FROM fusion_match_rules
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_rule(
    State(state): State<AppState>,
    Json(body): Json<CreateMatchRuleRequest>,
) -> ServiceResult<MatchRule> {
    if body.name.trim().is_empty() || body.conditions.is_empty() {
        return Err(bad_request(
            "rule name and at least one condition are required",
        ));
    }

    let row = query_as::<_, MatchRuleRow>(
        r#"
        INSERT INTO fusion_match_rules (
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold,
            created_at,
            updated_at
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description.unwrap_or_default())
    .bind(body.status.unwrap_or_else(|| "active".to_string()))
    .bind(body.entity_type.unwrap_or_else(|| "person".to_string()))
    .bind(SqlJson(body.blocking_strategy.unwrap_or_default()))
    .bind(SqlJson(body.conditions))
    .bind(body.review_threshold.unwrap_or(0.76))
    .bind(body.auto_merge_threshold.unwrap_or(0.9))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_rule(
    State(state): State<AppState>,
    Path(rule_id): Path<Uuid>,
    Json(body): Json<UpdateMatchRuleRequest>,
) -> ServiceResult<MatchRule> {
    let Some(current) = load_rule_row(&state.db, rule_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("match rule not found"));
    };

    let rule: MatchRule = current.into();
    let row = query_as::<_, MatchRuleRow>(
        r#"
        UPDATE fusion_match_rules
        SET name = $2,
            description = $3,
            status = $4,
            entity_type = $5,
            blocking_strategy = $6,
            conditions = $7,
            review_threshold = $8,
            auto_merge_threshold = $9,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            blocking_strategy,
            conditions,
            review_threshold,
            auto_merge_threshold,
            created_at,
            updated_at
        "#,
    )
    .bind(rule_id)
    .bind(body.name.unwrap_or(rule.name))
    .bind(body.description.unwrap_or(rule.description))
    .bind(body.status.unwrap_or(rule.status))
    .bind(body.entity_type.unwrap_or(rule.entity_type))
    .bind(SqlJson(
        body.blocking_strategy.unwrap_or(rule.blocking_strategy),
    ))
    .bind(SqlJson(body.conditions.unwrap_or(rule.conditions)))
    .bind(body.review_threshold.unwrap_or(rule.review_threshold))
    .bind(
        body.auto_merge_threshold
            .unwrap_or(rule.auto_merge_threshold),
    )
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn list_merge_strategies(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<MergeStrategy>> {
    let rows = query_as::<_, MergeStrategyRow>(
        r#"
        SELECT
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules,
            created_at,
            updated_at
        FROM fusion_merge_strategies
        ORDER BY updated_at DESC, created_at DESC
        "#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_merge_strategy(
    State(state): State<AppState>,
    Json(body): Json<CreateMergeStrategyRequest>,
) -> ServiceResult<MergeStrategy> {
    if body.name.trim().is_empty() {
        return Err(bad_request("merge strategy name is required"));
    }

    let row = query_as::<_, MergeStrategyRow>(
        r#"
        INSERT INTO fusion_merge_strategies (
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules,
            created_at,
            updated_at
        "#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description.unwrap_or_default())
    .bind(body.status.unwrap_or_else(|| "active".to_string()))
    .bind(body.entity_type.unwrap_or_else(|| "person".to_string()))
    .bind(
        body.default_strategy
            .unwrap_or_else(|| "longest_non_empty".to_string()),
    )
    .bind(SqlJson(body.rules))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_merge_strategy(
    State(state): State<AppState>,
    Path(strategy_id): Path<Uuid>,
    Json(body): Json<UpdateMergeStrategyRequest>,
) -> ServiceResult<MergeStrategy> {
    let Some(current) = load_merge_strategy_row(&state.db, strategy_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("merge strategy not found"));
    };

    let strategy: MergeStrategy = current.into();
    let row = query_as::<_, MergeStrategyRow>(
        r#"
        UPDATE fusion_merge_strategies
        SET name = $2,
            description = $3,
            status = $4,
            entity_type = $5,
            default_strategy = $6,
            rules = $7,
            updated_at = NOW()
        WHERE id = $1
        RETURNING
            id,
            name,
            description,
            status,
            entity_type,
            default_strategy,
            rules,
            created_at,
            updated_at
        "#,
    )
    .bind(strategy_id)
    .bind(body.name.unwrap_or(strategy.name))
    .bind(body.description.unwrap_or(strategy.description))
    .bind(body.status.unwrap_or(strategy.status))
    .bind(body.entity_type.unwrap_or(strategy.entity_type))
    .bind(body.default_strategy.unwrap_or(strategy.default_strategy))
    .bind(SqlJson(body.rules.unwrap_or(strategy.rules)))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}
