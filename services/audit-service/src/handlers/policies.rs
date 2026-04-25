use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::templates,
    handlers::{
        ServiceResult, bad_request, db_error, forbidden, internal_error, load_policies,
        load_policy_row,
    },
    models::{
        ListResponse,
        data_classification::{ClassificationCatalogEntry, ClassificationLevel},
        governance_template::{ApplyGovernanceTemplateRequest, GovernanceTemplate},
        policy::{AuditPolicy, CreatePolicyRequest, UpdatePolicyRequest},
    },
};

pub async fn list_classifications() -> ServiceResult<Vec<ClassificationCatalogEntry>> {
    Ok(Json(vec![
        ClassificationCatalogEntry::new(
            ClassificationLevel::Public,
            "Low sensitivity, broad export allowed",
        ),
        ClassificationCatalogEntry::new(
            ClassificationLevel::Confidential,
            "Internal-only, export controlled",
        ),
        ClassificationCatalogEntry::new(
            ClassificationLevel::Pii,
            "Personal data requiring masking and GDPR workflows",
        ),
    ]))
}

pub async fn list_policies(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<AuditPolicy>> {
    let policies = load_policies(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: policies }))
}

pub async fn list_governance_templates() -> ServiceResult<Vec<GovernanceTemplate>> {
    Ok(Json(templates::governance_template_catalog()))
}

pub async fn apply_governance_template(
    Path(slug): Path<String>,
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(request): Json<ApplyGovernanceTemplateRequest>,
) -> ServiceResult<ListResponse<AuditPolicy>> {
    if !claims.has_role("admin") && !claims.has_permission("policies", "write") {
        return Err(forbidden(
            "missing permission policies:write",
        ));
    }
    if request.updated_by.trim().is_empty() {
        return Err(bad_request("updated_by is required"));
    }

    let template = templates::find_governance_template(&slug)
        .ok_or_else(|| crate::handlers::not_found("governance template not found"))?;
    let scope_override = request.scope.as_deref().map(str::trim).filter(|value| !value.is_empty());

    let mut applied_ids = Vec::new();
    let now = Utc::now();
    for policy in template.policies {
        let effective_scope = scope_override.unwrap_or(policy.scope.as_str()).to_string();
        let rules = serde_json::to_value(&policy.rules)
            .map_err(|cause| internal_error(cause.to_string()))?;

        let existing_id = sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM audit_policies WHERE name = $1 AND scope = $2 ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(&policy.name)
        .bind(&effective_scope)
        .fetch_optional(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

        let id = if let Some(existing_id) = existing_id {
            sqlx::query(
                "UPDATE audit_policies
                 SET description = $2, classification = $3, retention_days = $4, legal_hold = $5, purge_mode = $6, active = true, rules = $7::jsonb, updated_by = $8, updated_at = $9
                 WHERE id = $1",
            )
            .bind(existing_id)
            .bind(&policy.description)
            .bind(policy.classification.as_str())
            .bind(policy.retention_days)
            .bind(policy.legal_hold)
            .bind(&policy.purge_mode)
            .bind(rules)
            .bind(&request.updated_by)
            .bind(now)
            .execute(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;
            existing_id
        } else {
            let id = uuid::Uuid::now_v7();
            sqlx::query(
                "INSERT INTO audit_policies (id, name, description, scope, classification, retention_days, legal_hold, purge_mode, active, rules, updated_by, created_at, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9::jsonb, $10, $11, $12)",
            )
            .bind(id)
            .bind(&policy.name)
            .bind(&policy.description)
            .bind(&effective_scope)
            .bind(policy.classification.as_str())
            .bind(policy.retention_days)
            .bind(policy.legal_hold)
            .bind(&policy.purge_mode)
            .bind(rules)
            .bind(&request.updated_by)
            .bind(now)
            .bind(now)
            .execute(&state.db)
            .await
            .map_err(|cause| db_error(&cause))?;
            id
        };

        applied_ids.push(id);
    }

    let mut applied = Vec::with_capacity(applied_ids.len());
    for id in applied_ids {
        let row = load_policy_row(&state.db, id)
            .await
            .map_err(|cause| db_error(&cause))?
            .ok_or_else(|| internal_error("applied policy could not be reloaded"))?;
        applied.push(
            AuditPolicy::try_from(row).map_err(|cause| internal_error(cause.to_string()))?,
        );
    }

    Ok(Json(ListResponse { items: applied }))
}

pub async fn create_policy(
    State(state): State<AppState>,
    Json(request): Json<CreatePolicyRequest>,
) -> ServiceResult<AuditPolicy> {
    if request.name.trim().is_empty() {
        return Err(bad_request("policy name is required"));
    }
    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let rules =
        serde_json::to_value(&request.rules).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO audit_policies (id, name, description, scope, classification, retention_days, legal_hold, purge_mode, active, rules, updated_by, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11, $12, $13)",
	)
	.bind(id)
	.bind(&request.name)
	.bind(&request.description)
	.bind(&request.scope)
	.bind(request.classification.as_str())
	.bind(request.retention_days)
	.bind(request.legal_hold)
	.bind(&request.purge_mode)
	.bind(request.active)
	.bind(rules)
	.bind(&request.updated_by)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_policy_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created policy could not be reloaded"))?;
    let policy = AuditPolicy::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(policy))
}

pub async fn update_policy(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdatePolicyRequest>,
) -> ServiceResult<AuditPolicy> {
    let row = load_policy_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| crate::handlers::not_found("policy not found"))?;
    let mut policy =
        AuditPolicy::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;

    if let Some(name) = request.name {
        policy.name = name;
    }
    if let Some(description) = request.description {
        policy.description = description;
    }
    if let Some(scope) = request.scope {
        policy.scope = scope;
    }
    if let Some(classification) = request.classification {
        policy.classification = classification;
    }
    if let Some(retention_days) = request.retention_days {
        policy.retention_days = retention_days;
    }
    if let Some(legal_hold) = request.legal_hold {
        policy.legal_hold = legal_hold;
    }
    if let Some(purge_mode) = request.purge_mode {
        policy.purge_mode = purge_mode;
    }
    if let Some(active) = request.active {
        policy.active = active;
    }
    if let Some(rules) = request.rules {
        policy.rules = rules;
    }
    if let Some(updated_by) = request.updated_by {
        policy.updated_by = updated_by;
    }

    let now = Utc::now();
    let rules =
        serde_json::to_value(&policy.rules).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"UPDATE audit_policies
		 SET name = $2, description = $3, scope = $4, classification = $5, retention_days = $6, legal_hold = $7, purge_mode = $8, active = $9, rules = $10::jsonb, updated_by = $11, updated_at = $12
		 WHERE id = $1",
	)
	.bind(id)
	.bind(&policy.name)
	.bind(&policy.description)
	.bind(&policy.scope)
	.bind(policy.classification.as_str())
	.bind(policy.retention_days)
	.bind(policy.legal_hold)
	.bind(&policy.purge_mode)
	.bind(policy.active)
	.bind(rules)
	.bind(&policy.updated_by)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = load_policy_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated policy could not be reloaded"))?;
    let policy = AuditPolicy::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(policy))
}
