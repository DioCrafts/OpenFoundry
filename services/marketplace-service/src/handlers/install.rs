use axum::{Json, extract::State, http::HeaderMap};

use crate::{
    AppState,
    domain::{activation, dependency, registry},
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_installs, load_listing_row,
        load_versions, not_found,
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
    let version = versions
        .iter()
        .find(|entry| entry.version == request.version)
        .cloned()
        .or_else(|| registry::latest_version(&listing, &versions))
        .ok_or_else(|| bad_request("listing has no published versions"))?;
    let dependency_plan = dependency::resolve_dependencies(&version);
    let install_id = uuid::Uuid::now_v7();
    let activation = activation::activate_install(
        &state,
        &headers,
        &listing,
        &version,
        &request.workspace_name,
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
        &request.workspace_name,
        activation.clone(),
    );
    let dependency_plan = serde_json::to_value(&dependency_plan)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let activation =
        serde_json::to_value(&activation).map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO marketplace_installs (id, listing_id, listing_name, version, workspace_name, status, dependency_plan, activation, installed_at, ready_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9, $10)",
	)
	.bind(install.id)
	.bind(install.listing_id)
	.bind(&install.listing_name)
	.bind(&install.version)
	.bind(&install.workspace_name)
	.bind(&install.status)
	.bind(dependency_plan)
	.bind(activation)
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
