use std::collections::HashMap;

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    domain::{devops, registry, validator},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_enrollment_branch_rows,
        load_fleet_row, load_fleet_rows, load_installs, load_listing_row, load_promotion_gate_row,
        load_promotion_gate_rows, load_versions, not_found,
    },
    models::{
        ListResponse,
        devops::{
            CreateEnrollmentBranchRequest, CreateProductFleetRequest, CreatePromotionGateRequest,
            EnrollmentBranchRecord, FleetSyncResult, ProductFleetRecord, ProductFleetRow,
            PromotionGateRecord, SyncFleetRequest, UpdatePromotionGateRequest,
        },
        install::{InstallActivation, InstallRecord},
        listing::ListingDefinition,
    },
};

pub async fn list_fleets(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<ProductFleetRecord>> {
    let rows = load_fleet_rows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let mut fleets = Vec::with_capacity(rows.len());

    for row in rows {
        fleets.push(hydrate_fleet(&state, row).await?);
    }

    Ok(Json(ListResponse { items: fleets }))
}

pub async fn create_fleet(
    State(state): State<AppState>,
    Json(request): Json<CreateProductFleetRequest>,
) -> ServiceResult<ProductFleetRecord> {
    validator::validate_product_fleet(&request).map_err(bad_request)?;
    let Some(_) = load_listing_row(&state.db, request.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("listing not found"));
    };

    let workspace_targets = devops::normalize_workspace_targets(&request.workspace_targets);
    if workspace_targets.is_empty() {
        return Err(bad_request("at least one workspace target is required"));
    }
    let deployment_cells = devops::normalize_deployment_cells(
        &request.deployment_cells,
        &workspace_targets,
        &request.environment,
    );
    let residency_policy =
        devops::normalize_residency_policy(&request.residency_policy, &deployment_cells);

    let id = uuid::Uuid::now_v7();
    let workspace_targets_json = serde_json::to_value(&workspace_targets)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let maintenance_window = serde_json::to_value(&request.maintenance_window)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let deployment_cells_json = serde_json::to_value(&deployment_cells)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let residency_policy_json = serde_json::to_value(&residency_policy)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        "INSERT INTO marketplace_product_fleets
         (id, listing_id, name, environment, workspace_targets, release_channel, auto_upgrade_enabled, maintenance_window, branch_strategy, rollout_strategy, deployment_cells, residency_policy, status, last_synced_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5::jsonb, $6, $7, $8::jsonb, $9, $10, $11::jsonb, $12::jsonb, 'active', NULL, NOW(), NOW())",
    )
    .bind(id)
    .bind(request.listing_id)
    .bind(request.name.trim())
    .bind(request.environment.trim())
    .bind(workspace_targets_json)
    .bind(devops::normalize_release_channel(&request.release_channel))
    .bind(request.auto_upgrade_enabled)
    .bind(maintenance_window)
    .bind(request.branch_strategy.trim())
    .bind(request.rollout_strategy.trim())
    .bind(deployment_cells_json)
    .bind(residency_policy_json)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_fleet_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created fleet could not be reloaded"))?;

    Ok(Json(hydrate_fleet(&state, row).await?))
}

pub async fn sync_fleet(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<SyncFleetRequest>,
) -> ServiceResult<FleetSyncResult> {
    let row = load_fleet_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("fleet not found"))?;
    let fleet = hydrate_fleet(&state, row.clone()).await?;
    let listing = load_listing_row(&state.db, fleet.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let listing =
        ListingDefinition::try_from(listing).map_err(|cause| internal_error(cause.to_string()))?;
    let versions = load_versions(&state.db, fleet.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let Some(target_version) =
        devops::latest_version_for_channel(&versions, &fleet.release_channel)
    else {
        return Ok(Json(FleetSyncResult {
            fleet,
            target_version: None,
            upgraded_workspaces: Vec::new(),
            skipped_workspaces: Vec::new(),
            blocked_workspaces: Vec::new(),
            workspace_cell_assignments: HashMap::new(),
            blocking_gates: Vec::new(),
            blocked_reason: Some(
                "no published version is available for the fleet channel".to_string(),
            ),
            generated_at: Utc::now(),
        }));
    };

    let promotion_gates = load_promotion_gate_rows(&state.db, Some(fleet.id))
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .map(PromotionGateRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    let blocking_gates = devops::promotion_gate_blockers(&promotion_gates);
    if !blocking_gates.is_empty() {
        return Ok(Json(FleetSyncResult {
            fleet,
            target_version: Some(target_version.version),
            upgraded_workspaces: Vec::new(),
            skipped_workspaces: Vec::new(),
            blocked_workspaces: Vec::new(),
            workspace_cell_assignments: HashMap::new(),
            blocking_gates: blocking_gates.clone(),
            blocked_reason: Some(format!(
                "promotion gates blocking rollout: {}",
                blocking_gates.join(", ")
            )),
            generated_at: Utc::now(),
        }));
    }

    if !request.force && !devops::maintenance_window_is_open(&fleet.maintenance_window, Utc::now())
    {
        return Ok(Json(FleetSyncResult {
            fleet: fleet.clone(),
            target_version: Some(target_version.version.clone()),
            upgraded_workspaces: Vec::new(),
            skipped_workspaces: fleet.workspace_targets.clone(),
            blocked_workspaces: Vec::new(),
            workspace_cell_assignments: HashMap::new(),
            blocking_gates: Vec::new(),
            blocked_reason: Some(
                "current time falls outside the configured maintenance window".to_string(),
            ),
            generated_at: Utc::now(),
        }));
    }

    let installs = load_installs(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let latest_by_workspace = latest_installs_for_fleet(&installs, fleet.id);
    let dependency_plan = serde_json::to_value(&target_version.dependencies)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let maintenance_window = serde_json::to_value(&fleet.maintenance_window)
        .map_err(|cause| internal_error(cause.to_string()))?;

    let mut upgraded_workspaces = Vec::new();
    let mut skipped_workspaces = Vec::new();
    let mut blocked_workspaces = Vec::new();
    let mut workspace_cell_assignments = HashMap::new();

    for workspace in &fleet.workspace_targets {
        if latest_by_workspace
            .get(workspace)
            .map(|install| install.version.as_str() == target_version.version.as_str())
            .unwrap_or(false)
        {
            skipped_workspaces.push(workspace.clone());
            continue;
        }

        let Some(cell) = devops::assign_workspace_to_cell(
            workspace,
            &fleet.deployment_cells,
            &fleet.residency_policy,
        ) else {
            blocked_workspaces.push(workspace.clone());
            skipped_workspaces.push(workspace.clone());
            continue;
        };
        workspace_cell_assignments.insert(workspace.clone(), cell.name.clone());

        let activation = InstallActivation {
            kind: "fleet_rollout".to_string(),
            status: "applied".to_string(),
            resource_id: None,
            resource_slug: Some(listing.slug.clone()),
            public_url: None,
            notes: Some(format!(
                "Rolled out {} {} to workspace `{}` via fleet `{}` on channel `{}` through cell `{}` ({} / {}).",
                listing.name,
                target_version.version,
                workspace,
                fleet.name,
                fleet.release_channel,
                cell.name,
                cell.cloud,
                cell.region
            )),
        };
        let mut install = registry::install_preview(
            uuid::Uuid::now_v7(),
            &listing,
            &target_version,
            workspace,
            activation,
            Some(fleet.id),
            Some(fleet.name.clone()),
            Some(fleet.maintenance_window.clone()),
            fleet.auto_upgrade_enabled,
            None,
        );
        install.status = "upgraded".to_string();

        let activation_json = serde_json::to_value(&install.activation)
            .map_err(|cause| internal_error(cause.to_string()))?;

        sqlx::query(
            "INSERT INTO marketplace_installs
             (id, listing_id, listing_name, version, release_channel, workspace_name, status, dependency_plan, activation, fleet_id, maintenance_window, auto_upgrade_enabled, enrollment_branch, installed_at, ready_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, $11::jsonb, $12, $13, $14, $15)",
        )
        .bind(install.id)
        .bind(install.listing_id)
        .bind(&install.listing_name)
        .bind(&install.version)
        .bind(&install.release_channel)
        .bind(&install.workspace_name)
        .bind(&install.status)
        .bind(dependency_plan.clone())
        .bind(activation_json)
        .bind(install.fleet_id)
        .bind(maintenance_window.clone())
        .bind(install.auto_upgrade_enabled)
        .bind(install.enrollment_branch.clone())
        .bind(install.installed_at)
        .bind(install.ready_at)
        .execute(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;

        upgraded_workspaces.push(workspace.clone());
    }

    sqlx::query(
        "UPDATE marketplace_product_fleets
         SET last_synced_at = $2, updated_at = $2
         WHERE id = $1",
    )
    .bind(fleet.id)
    .bind(Utc::now())
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let refreshed = load_fleet_row(&state.db, fleet.id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("fleet could not be reloaded after sync"))?;
    let fleet = hydrate_fleet(&state, refreshed).await?;

    Ok(Json(FleetSyncResult {
        fleet,
        target_version: Some(target_version.version),
        upgraded_workspaces,
        skipped_workspaces,
        blocked_workspaces,
        workspace_cell_assignments,
        blocking_gates: Vec::new(),
        blocked_reason: None,
        generated_at: Utc::now(),
    }))
}

pub async fn list_promotion_gates(
    Path(fleet_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<PromotionGateRecord>> {
    load_fleet_row(&state.db, fleet_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("fleet not found"))?;

    let gates = load_promotion_gate_rows(&state.db, Some(fleet_id))
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .map(PromotionGateRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    Ok(Json(ListResponse { items: gates }))
}

pub async fn create_promotion_gate(
    Path(fleet_id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<CreatePromotionGateRequest>,
) -> ServiceResult<PromotionGateRecord> {
    validator::validate_promotion_gate(&request).map_err(bad_request)?;
    let fleet = load_fleet_row(&state.db, fleet_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("fleet not found"))?;

    let gate_id = uuid::Uuid::now_v7();
    sqlx::query(
        "INSERT INTO marketplace_fleet_promotion_gates
         (id, fleet_id, name, gate_kind, required, status, evidence, notes, last_evaluated_at, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8, $9, NOW(), NOW())",
    )
    .bind(gate_id)
    .bind(fleet.id)
    .bind(request.name.trim())
    .bind(request.gate_kind.trim())
    .bind(request.required)
    .bind(request.status.unwrap_or_else(|| "pending".to_string()))
    .bind(request.evidence)
    .bind(request.notes.trim())
    .bind(Some(Utc::now()))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let gate = load_promotion_gate_row(&state.db, gate_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created promotion gate could not be reloaded"))?;

    Ok(Json(
        PromotionGateRecord::try_from(gate).map_err(internal_error)?,
    ))
}

pub async fn update_promotion_gate(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdatePromotionGateRequest>,
) -> ServiceResult<PromotionGateRecord> {
    let current = load_promotion_gate_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("promotion gate not found"))?;
    if let Some(status) = request.status.as_deref() {
        validator::validate_gate_status(status).map_err(bad_request)?;
    }

    sqlx::query(
        "UPDATE marketplace_fleet_promotion_gates
         SET required = $2,
             status = $3,
             evidence = $4::jsonb,
             notes = $5,
             last_evaluated_at = $6,
             updated_at = NOW()
         WHERE id = $1",
    )
    .bind(id)
    .bind(request.required.unwrap_or(current.required))
    .bind(request.status.unwrap_or(current.status))
    .bind(request.evidence.unwrap_or(current.evidence))
    .bind(request.notes.unwrap_or(current.notes))
    .bind(Some(Utc::now()))
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let gate = load_promotion_gate_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated promotion gate could not be reloaded"))?;

    Ok(Json(
        PromotionGateRecord::try_from(gate).map_err(internal_error)?,
    ))
}

pub async fn list_enrollment_branches(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<EnrollmentBranchRecord>> {
    let rows = load_enrollment_branch_rows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let branches = rows
        .into_iter()
        .map(EnrollmentBranchRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    Ok(Json(ListResponse { items: branches }))
}

pub async fn create_enrollment_branch(
    State(state): State<AppState>,
    Json(request): Json<CreateEnrollmentBranchRequest>,
) -> ServiceResult<EnrollmentBranchRecord> {
    validator::validate_enrollment_branch(&request).map_err(bad_request)?;
    let fleet_row = load_fleet_row(&state.db, request.fleet_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("fleet not found"))?;
    let fleet = hydrate_fleet(&state, fleet_row.clone()).await?;
    let versions = load_versions(&state.db, fleet.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let source_version = devops::latest_version_for_channel(&versions, &fleet.release_channel)
        .map(|version| version.version);
    let repository_branch = request
        .repository_branch
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| devops::derive_repository_branch(&fleet.name, &request.name));
    let branch_id = uuid::Uuid::now_v7();
    let workspace_targets = serde_json::to_value(&fleet.workspace_targets)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        "INSERT INTO marketplace_enrollment_branches
         (id, fleet_id, listing_id, name, repository_branch, source_release_channel, source_version, workspace_targets, status, notes, created_at, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, 'active', $9, NOW(), NOW())",
    )
    .bind(branch_id)
    .bind(fleet.id)
    .bind(fleet.listing_id)
    .bind(request.name.trim())
    .bind(repository_branch)
    .bind(&fleet.release_channel)
    .bind(source_version)
    .bind(workspace_targets)
    .bind(request.notes.trim())
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let rows = load_enrollment_branch_rows(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let branch = rows
        .into_iter()
        .find(|entry| entry.id == branch_id)
        .ok_or_else(|| internal_error("created enrollment branch could not be reloaded"))?;

    Ok(Json(
        EnrollmentBranchRecord::try_from(branch).map_err(internal_error)?,
    ))
}

pub async fn reconcile_auto_upgrade_fleets(state: AppState) -> Result<(), String> {
    let rows = load_fleet_rows(&state.db)
        .await
        .map_err(|error| error.to_string())?;

    for row in rows.into_iter().filter(|row| row.auto_upgrade_enabled) {
        if let Err(error) = auto_sync_fleet(&state, row).await {
            tracing::warn!("auto-upgrade reconciliation skipped for fleet: {error}");
        }
    }

    Ok(())
}

async fn hydrate_fleet(
    state: &AppState,
    row: ProductFleetRow,
) -> Result<ProductFleetRecord, (axum::http::StatusCode, Json<crate::handlers::ErrorResponse>)> {
    let installs = load_installs(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let versions = load_versions(&state.db, row.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let promotion_gates = load_promotion_gate_rows(&state.db, Some(row.id))
        .await
        .map_err(|cause| db_error(&cause))?
        .into_iter()
        .map(PromotionGateRecord::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(internal_error)?;
    let latest_by_workspace = latest_installs_for_fleet(&installs, row.id);
    let workspace_targets: Vec<String> = serde_json::from_value(row.workspace_targets.clone())
        .map_err(|cause| internal_error(format!("invalid fleet workspace_targets: {cause}")))?;
    let target_version = devops::latest_version_for_channel(&versions, &row.release_channel)
        .map(|version| version.version);
    let current_version = latest_by_workspace
        .values()
        .max_by(|left, right| left.installed_at.cmp(&right.installed_at))
        .map(|install| install.version.clone());
    let pending_upgrade_count = workspace_targets
        .iter()
        .filter(|workspace| {
            latest_by_workspace
                .get(*workspace)
                .map(|install| Some(install.version.as_str()) != target_version.as_deref())
                .unwrap_or(true)
        })
        .count();

    ProductFleetRecord::from_row(
        row,
        latest_by_workspace.len(),
        current_version,
        target_version,
        pending_upgrade_count,
        devops::summarize_promotion_gates(&promotion_gates),
    )
    .map_err(internal_error)
}

fn latest_installs_for_fleet(
    installs: &[InstallRecord],
    fleet_id: uuid::Uuid,
) -> HashMap<String, InstallRecord> {
    let mut latest_by_workspace = HashMap::new();

    for install in installs
        .iter()
        .filter(|install| install.fleet_id == Some(fleet_id))
    {
        let replace = latest_by_workspace
            .get(&install.workspace_name)
            .map(|current: &InstallRecord| current.installed_at < install.installed_at)
            .unwrap_or(true);
        if replace {
            latest_by_workspace.insert(install.workspace_name.clone(), install.clone());
        }
    }

    latest_by_workspace
}

async fn auto_sync_fleet(state: &AppState, row: ProductFleetRow) -> Result<(), String> {
    let fleet = hydrate_fleet(state, row.clone())
        .await
        .map_err(|(_, body)| body.0.error)?;
    if !fleet.auto_upgrade_enabled
        || !devops::maintenance_window_is_open(&fleet.maintenance_window, Utc::now())
    {
        return Ok(());
    }

    let listing_row = load_listing_row(&state.db, fleet.listing_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "listing not found for auto-upgrade fleet".to_string())?;
    let listing = ListingDefinition::try_from(listing_row).map_err(|error| error.to_string())?;
    let versions = load_versions(&state.db, fleet.listing_id)
        .await
        .map_err(|error| error.to_string())?;
    let Some(target_version) =
        devops::latest_version_for_channel(&versions, &fleet.release_channel)
    else {
        return Ok(());
    };

    let installs = load_installs(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    let latest_by_workspace = latest_installs_for_fleet(&installs, fleet.id);
    let dependency_plan =
        serde_json::to_value(&target_version.dependencies).map_err(|error| error.to_string())?;
    let maintenance_window =
        serde_json::to_value(&fleet.maintenance_window).map_err(|error| error.to_string())?;
    let mut upgraded_any = false;

    for workspace in &fleet.workspace_targets {
        if latest_by_workspace
            .get(workspace)
            .map(|install| install.version.as_str() == target_version.version.as_str())
            .unwrap_or(false)
        {
            continue;
        }

        let activation = InstallActivation {
            kind: "fleet_rollout".to_string(),
            status: "applied".to_string(),
            resource_id: None,
            resource_slug: Some(listing.slug.clone()),
            public_url: None,
            notes: Some(format!(
                "Auto-upgraded {} {} to workspace `{}` via fleet `{}` on channel `{}`.",
                listing.name, target_version.version, workspace, fleet.name, fleet.release_channel
            )),
        };
        let mut install = registry::install_preview(
            uuid::Uuid::now_v7(),
            &listing,
            &target_version,
            workspace,
            activation,
            Some(fleet.id),
            Some(fleet.name.clone()),
            Some(fleet.maintenance_window.clone()),
            fleet.auto_upgrade_enabled,
            None,
        );
        install.status = "auto-upgraded".to_string();
        let activation_json =
            serde_json::to_value(&install.activation).map_err(|error| error.to_string())?;

        sqlx::query(
            "INSERT INTO marketplace_installs
             (id, listing_id, listing_name, version, release_channel, workspace_name, status, dependency_plan, activation, fleet_id, maintenance_window, auto_upgrade_enabled, enrollment_branch, installed_at, ready_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, $11::jsonb, $12, $13, $14, $15)",
        )
        .bind(install.id)
        .bind(install.listing_id)
        .bind(&install.listing_name)
        .bind(&install.version)
        .bind(&install.release_channel)
        .bind(&install.workspace_name)
        .bind(&install.status)
        .bind(dependency_plan.clone())
        .bind(activation_json)
        .bind(install.fleet_id)
        .bind(maintenance_window.clone())
        .bind(install.auto_upgrade_enabled)
        .bind(install.enrollment_branch.clone())
        .bind(install.installed_at)
        .bind(install.ready_at)
        .execute(&state.db)
        .await
        .map_err(|error| error.to_string())?;

        upgraded_any = true;
    }

    if upgraded_any {
        sqlx::query(
            "UPDATE marketplace_product_fleets
             SET last_synced_at = $2, updated_at = $2
             WHERE id = $1",
        )
        .bind(fleet.id)
        .bind(Utc::now())
        .execute(&state.db)
        .await
        .map_err(|error| error.to_string())?;
    }

    Ok(())
}
