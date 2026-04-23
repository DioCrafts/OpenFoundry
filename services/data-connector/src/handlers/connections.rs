use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use uuid::Uuid;

use crate::AppState;
use crate::connectors;
use crate::models::connection::{Connection, CreateConnectionRequest, ListConnectionsQuery, VALID_TYPES};

/// POST /api/v1/connections
pub async fn create_connection(
    State(state): State<AppState>,
    auth_middleware::layer::AuthUser(claims): auth_middleware::layer::AuthUser,
    Json(body): Json<CreateConnectionRequest>,
) -> impl IntoResponse {
    // Validate connector type
    if !VALID_TYPES.contains(&body.connector_type.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("unsupported type: {}", body.connector_type) })),
        ).into_response();
    }

    // Validate config per connector type
    let validation = match body.connector_type.as_str() {
        "postgresql" => connectors::postgres::validate_config(&body.config),
        "csv" => connectors::csv::validate_config(&body.config),
        "json" => connectors::json::validate_config(&body.config),
        "rest_api" => connectors::rest_api::validate_config(&body.config),
        _ => Ok(()),
    };

    if let Err(msg) = validation {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({ "error": msg }))).into_response();
    }

    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, Connection>(
        r#"INSERT INTO connections (id, name, connector_type, config, owner_id)
           VALUES ($1, $2, $3, $4, $5)
           RETURNING *"#,
    )
    .bind(id)
    .bind(&body.name)
    .bind(&body.connector_type)
    .bind(&body.config)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(conn) => (StatusCode::CREATED, Json(conn)).into_response(),
        Err(e) => {
            tracing::error!("create connection failed: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({ "error": "create failed" }))).into_response()
        }
    }
}

/// GET /api/v1/connections
pub async fn list_connections(
    State(state): State<AppState>,
    Query(params): Query<ListConnectionsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;

    let connections = sqlx::query_as::<_, Connection>(
        "SELECT * FROM connections ORDER BY created_at DESC LIMIT $1 OFFSET $2",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await;

    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM connections")
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

    match connections {
        Ok(conns) => Json(serde_json::json!({
            "data": conns,
            "page": page,
            "per_page": per_page,
            "total": total,
        })).into_response(),
        Err(e) => {
            tracing::error!("list connections failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// GET /api/v1/connections/:id
pub async fn get_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let conn = sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
        .bind(connection_id)
        .fetch_optional(&state.db)
        .await;

    match conn {
        Ok(Some(c)) => Json(c).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("get connection failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

/// POST /api/v1/connections/:id/test
pub async fn test_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let conn = match sqlx::query_as::<_, Connection>("SELECT * FROM connections WHERE id = $1")
        .bind(connection_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("test connection lookup failed: {e}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    // For PostgreSQL, try to actually connect
    let (success, message) = match conn.connector_type.as_str() {
        "postgresql" => {
            let dsn = connectors::postgres::build_connection_string(&conn.config);
            match sqlx::PgPool::connect(&dsn).await {
                Ok(_pool) => (true, "connection successful".to_string()),
                Err(e) => (false, format!("connection failed: {e}")),
            }
        }
        _ => (true, "config validated".to_string()),
    };

    // Update status
    let new_status = if success { "connected" } else { "error" };
    let _ = sqlx::query("UPDATE connections SET status = $2, updated_at = NOW() WHERE id = $1")
        .bind(connection_id)
        .bind(new_status)
        .execute(&state.db)
        .await;

    Json(serde_json::json!({
        "success": success,
        "message": message,
    })).into_response()
}

/// DELETE /api/v1/connections/:id
pub async fn delete_connection(
    State(state): State<AppState>,
    Path(connection_id): Path<Uuid>,
) -> impl IntoResponse {
    let result = sqlx::query("DELETE FROM connections WHERE id = $1")
        .bind(connection_id)
        .execute(&state.db)
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => {
            tracing::error!("delete connection failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
