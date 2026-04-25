use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;

use crate::{
    AppState,
    handlers::{
        ServiceResult, bad_request, db_error, internal_error, load_peers, load_space_row,
        load_spaces, not_found,
    },
    models::{
        ListResponse,
        space::{CreateSpaceRequest, NexusSpace, UpdateSpaceRequest},
    },
};

pub async fn list_spaces(State(state): State<AppState>) -> ServiceResult<ListResponse<NexusSpace>> {
    let items = load_spaces(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse { items }))
}

pub async fn create_space(
    State(state): State<AppState>,
    Json(request): Json<CreateSpaceRequest>,
) -> ServiceResult<NexusSpace> {
    validate_space_request(
        &state.db,
        &request.slug,
        &request.display_name,
        &request.space_kind,
        request.owner_peer_id,
        &request.member_peer_ids,
        &request.status,
    )
    .await?;

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let member_peer_ids = serde_json::to_value(&request.member_peer_ids)
        .map_err(|cause| internal_error(cause.to_string()))?;
    let governance_tags = serde_json::to_value(&request.governance_tags)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
        r#"INSERT INTO nexus_spaces (id, slug, display_name, description, space_kind, owner_peer_id, region, member_peer_ids, governance_tags, status, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9::jsonb, $10, $11, $12)"#,
    )
    .bind(id)
    .bind(&request.slug)
    .bind(&request.display_name)
    .bind(&request.description)
    .bind(&request.space_kind)
    .bind(request.owner_peer_id)
    .bind(&request.region)
    .bind(member_peer_ids)
    .bind(governance_tags)
    .bind(&request.status)
    .bind(now)
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_space_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created space could not be reloaded"))?;
    let space = NexusSpace::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(space))
}

pub async fn update_space(
    Path(id): Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateSpaceRequest>,
) -> ServiceResult<NexusSpace> {
    let current = load_space_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| not_found("space not found"))?;
    let current =
        NexusSpace::try_from(current).map_err(|cause| internal_error(cause.to_string()))?;

    let owner_peer_id = request.owner_peer_id.or(current.owner_peer_id);
    let member_peer_ids = request
        .member_peer_ids
        .clone()
        .unwrap_or(current.member_peer_ids.clone());
    validate_space_request(
        &state.db,
        &current.slug,
        request
            .display_name
            .as_deref()
            .unwrap_or(current.display_name.as_str()),
        &current.space_kind,
        owner_peer_id,
        &member_peer_ids,
        request.status.as_deref().unwrap_or(current.status.as_str()),
    )
    .await?;

    let now = Utc::now();
    sqlx::query(
        r#"UPDATE nexus_spaces
           SET display_name = $2,
               description = $3,
               owner_peer_id = $4,
               region = $5,
               member_peer_ids = $6::jsonb,
               governance_tags = $7::jsonb,
               status = $8,
               updated_at = $9
           WHERE id = $1"#,
    )
    .bind(id)
    .bind(request.display_name.unwrap_or(current.display_name))
    .bind(request.description.unwrap_or(current.description))
    .bind(owner_peer_id)
    .bind(request.region.unwrap_or(current.region))
    .bind(serde_json::to_value(member_peer_ids).map_err(|cause| internal_error(cause.to_string()))?)
    .bind(
        serde_json::to_value(request.governance_tags.unwrap_or(current.governance_tags))
            .map_err(|cause| internal_error(cause.to_string()))?,
    )
    .bind(request.status.unwrap_or(current.status))
    .bind(now)
    .execute(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    let row = load_space_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated space could not be reloaded"))?;
    let space = NexusSpace::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(space))
}

async fn validate_space_request(
    db: &sqlx::PgPool,
    slug: &str,
    display_name: &str,
    space_kind: &str,
    owner_peer_id: Option<uuid::Uuid>,
    member_peer_ids: &[uuid::Uuid],
    status: &str,
) -> Result<(), (axum::http::StatusCode, Json<crate::handlers::ErrorResponse>)> {
    if slug.trim().is_empty() || display_name.trim().is_empty() {
        return Err(bad_request("space slug and display name are required"));
    }
    if !matches!(space_kind, "private" | "shared") {
        return Err(bad_request("space_kind must be private or shared"));
    }
    if !matches!(status, "draft" | "active" | "paused") {
        return Err(bad_request("space status must be draft, active or paused"));
    }

    let peers = load_peers(db).await.map_err(|cause| db_error(&cause))?;
    if let Some(owner_peer_id) = owner_peer_id {
        if !peers.iter().any(|peer| peer.id == owner_peer_id) {
            return Err(bad_request("owner_peer_id does not exist"));
        }
    }
    if !member_peer_ids
        .iter()
        .all(|peer_id| peers.iter().any(|peer| &peer.id == peer_id))
    {
        return Err(bad_request(
            "member_peer_ids contains unknown peer references",
        ));
    }

    Ok(())
}
