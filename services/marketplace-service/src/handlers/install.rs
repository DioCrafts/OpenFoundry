use axum::{Json, extract::State, http::HeaderMap};

use crate::{
    AppState,
    domain::{activation, dependency, devops, registry},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_fleet_row, load_installs,
        load_listing_row, load_versions, not_found,
    },
    models::{
        ListResponse,
        install::{CreateInstallRequest, InstallRecord},
    },
};

pub async fn list_installs(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<InstallRecord>> {
    let installs = load_installs(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items: installs }))
}

pub async fn create_install(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CreateInstallRequest>,
) -> ServiceResult<InstallRecord> {
    let listing_row = load_listing_row(&state.db, request.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("listing not found"))?;
    let listing = crate::models::listing::ListingDefinition::try_from(listing_row)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let versions = load_versions(&state.db, request.listing_id)
        .await
        .map_err(|cause| db_error(&cause))?;
    let fleet = match request.fleet_id {
        Some(fleet_id) => Some(
            load_fleet_row(&state.db, fleet_id)
                .await
                .map_err(|cause| db_error(&cause))?
                .ok_or_else(|| not_found("fleet not found"))?,
        ),
        None => None,
    };
    if let Some(fleet) = &fleet {
        if fleet.listing_id != request.listing_id {
            return Err(bad_request(
                "fleet listing does not match the selected package",
            ));
        }
    }

    let requested_channel = fleet
        .as_ref()
        .map(|fleet| fleet.release_channel.clone())
        .unwrap_or_else(|| devops::normalize_release_channel(&request.release_channel));
    let version = versions
        .iter()
        .find(|entry| {
            entry.version == request.version
                && devops::normalize_release_channel(&entry.release_channel) == requested_channel
        })
        .cloned()
        .or_else(|| devops::latest_version_for_channel(&versions, &requested_channel))
        .ok_or_else(|| bad_request("listing has no published versions"))?;
    let dependency_plan = dependency::resolve_dependencies(&version);
    let install_id = uuid::Uuid::now_v7();
    let fleet_name = fleet.as_ref().map(|entry| entry.name.clone());
    let maintenance_window = fleet
        .as_ref()
        .and_then(|entry| serde_json::from_value(entry.maintenance_window.clone()).ok());
    let workspace_name = if request.workspace_name.trim().is_empty() {
        fleet
            .as_ref()
            .and_then(|entry| {
                serde_json::from_value::<Vec<String>>(entry.workspace_targets.clone()).ok()
            })
            .and_then(|targets| targets.into_iter().next())
            .unwrap_or_else(|| request.workspace_name.trim().to_string())
    } else {
        request.workspace_name.trim().to_string()
    };
    if workspace_name.is_empty() {
        return Err(bad_request("workspace name is required"));
    }
    let activation = activation::activate_install(
        &state,
        &headers,
        &listing,
        &version,
        &workspace_name,
        install_id,
    )
    .await
    .map_err(|cause| internal_error(cause.to_string()))?;
    let install = registry::install_preview(
        install_id,
        &listing,
        &crate::models::package::PackageVersion {
            dependencies: dependency_plan.clone(),
            ..version.clone()
        },
        &workspace_name,
        activation.clone(),
        request.fleet_id,
        fleet_name,
        maintenance_window.clone(),
        fleet
            .as_ref()
            .map(|entry| entry.auto_upgrade_enabled)
            .unwrap_or(false),
        request
            .enrollment_branch
            .clone()
            .filter(|branch| !branch.trim().is_empty()),
    );
    let dependency_plan = serde_json::to_value(&dependency_plan)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let activation =
        serde_json::to_value(&activation).map_err(|cause| internal_error(cause.to_string()))?;
    let maintenance_window = serde_json::to_value(&maintenance_window)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO marketplace_installs (id, listing_id, listing_name, version, release_channel, workspace_name, status, dependency_plan, activation, fleet_id, maintenance_window, auto_upgrade_enabled, enrollment_branch, installed_at, ready_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, $11::jsonb, $12, $13, $14, $15)",
	)
	.bind(install.id)
	.bind(install.listing_id)
	.bind(&install.listing_name)
	.bind(&install.version)
	.bind(&install.release_channel)
	.bind(&install.workspace_name)
	.bind(&install.status)
	.bind(dependency_plan)
	.bind(activation)
	.bind(install.fleet_id)
	.bind(maintenance_window)
	.bind(install.auto_upgrade_enabled)
	.bind(install.enrollment_branch.clone())
	.bind(install.installed_at)
	.bind(install.ready_at)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    sqlx::query("UPDATE marketplace_listings SET install_count = install_count + 1, updated_at = NOW() WHERE id = $1")
		.bind(install.listing_id)
		.execute(&state.db)
		.await
		.map_err(|cause| db_error(&cause))?;

    Ok(Json(install))
}
