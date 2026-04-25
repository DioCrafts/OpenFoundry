use auth_middleware::layer::AuthUser;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    domain::notepad,
    models::notepad::{
        CreateNotepadDocumentRequest, ListNotepadDocumentsQuery, NotepadDocument, NotepadPresence,
        UpdateNotepadDocumentRequest, UpsertNotepadPresenceRequest,
    },
};

pub async fn list_documents(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListNotepadDocumentsQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search = params.search.unwrap_or_default();
    let pattern = format!("%{search}%");

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notepad_documents WHERE title ILIKE $1 OR description ILIKE $1",
    )
    .bind(&pattern)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let documents = sqlx::query_as::<_, NotepadDocument>(
        r#"SELECT * FROM notepad_documents
           WHERE title ILIKE $1 OR description ILIKE $1
           ORDER BY updated_at DESC, created_at DESC
           LIMIT $2 OFFSET $3"#,
    )
    .bind(&pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(json!({
        "data": documents,
        "total": total,
        "page": page,
        "per_page": per_page
    }))
}

pub async fn create_document(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateNotepadDocumentRequest>,
) -> impl IntoResponse {
    if body.title.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "title is required" })),
        )
            .into_response();
    }

    let id = Uuid::now_v7();
    let result = sqlx::query_as::<_, NotepadDocument>(
        r#"INSERT INTO notepad_documents (
               id, title, description, owner_id, content, template_key, widgets
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           RETURNING *"#,
    )
    .bind(id)
    .bind(body.title.trim())
    .bind(body.description.unwrap_or_default())
    .bind(claims.sub)
    .bind(body.content.unwrap_or_default())
    .bind(body.template_key.and_then(non_empty))
    .bind(body.widgets.unwrap_or_else(|| json!([])))
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(document) => (StatusCode::CREATED, Json(document)).into_response(),
        Err(error) => {
            tracing::error!("create notepad document failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn get_document(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, NotepadDocument>("SELECT * FROM notepad_documents WHERE id = $1")
        .bind(document_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(document)) => Json(document).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("get notepad document failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn update_document(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(body): Json<UpdateNotepadDocumentRequest>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, NotepadDocument>(
        r#"UPDATE notepad_documents
           SET title = COALESCE($2, title),
               description = COALESCE($3, description),
               content = COALESCE($4, content),
               template_key = COALESCE($5, template_key),
               widgets = COALESCE($6, widgets),
               last_indexed_at = COALESCE($7, last_indexed_at),
               updated_at = NOW()
           WHERE id = $1
           RETURNING *"#,
    )
    .bind(document_id)
    .bind(body.title.as_deref().and_then(non_empty))
    .bind(body.description)
    .bind(body.content)
    .bind(body.template_key.and_then(non_empty))
    .bind(body.widgets)
    .bind(body.last_indexed_at)
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(document)) => Json(document).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("update notepad document failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn delete_document(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM notepad_documents WHERE id = $1")
        .bind(document_id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("delete notepad document failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn list_presence(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> impl IntoResponse {
    cleanup_presence(&state).await;

    match sqlx::query_as::<_, NotepadPresence>(
        r#"SELECT * FROM notepad_presence
           WHERE document_id = $1
           ORDER BY last_seen_at DESC"#,
    )
    .bind(document_id)
    .fetch_all(&state.db)
    .await
    {
        Ok(presence) => Json(json!({ "data": presence })).into_response(),
        Err(error) => {
            tracing::error!("list notepad presence failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn upsert_presence(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
    Json(body): Json<UpsertNotepadPresenceRequest>,
) -> impl IntoResponse {
    if body.session_id.trim().is_empty() || body.display_name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({ "error": "session_id and display_name are required" })),
        )
            .into_response();
    }

    cleanup_presence(&state).await;

    match sqlx::query_as::<_, NotepadPresence>(
        r#"INSERT INTO notepad_presence (
               id, document_id, user_id, session_id, display_name, cursor_label, color
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7)
           ON CONFLICT (document_id, user_id, session_id)
           DO UPDATE SET
               display_name = EXCLUDED.display_name,
               cursor_label = EXCLUDED.cursor_label,
               color = EXCLUDED.color,
               last_seen_at = NOW()
           RETURNING *"#,
    )
    .bind(Uuid::now_v7())
    .bind(document_id)
    .bind(claims.sub)
    .bind(body.session_id.trim())
    .bind(body.display_name.trim())
    .bind(body.cursor_label.unwrap_or_default())
    .bind(body.color.unwrap_or_else(|| "#0f766e".to_string()))
    .fetch_one(&state.db)
    .await
    {
        Ok(presence) => Json(presence).into_response(),
        Err(error) => {
            tracing::error!("upsert notepad presence failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

pub async fn export_document(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(document_id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query_as::<_, NotepadDocument>("SELECT * FROM notepad_documents WHERE id = $1")
        .bind(document_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(document)) => Json(notepad::render_export_payload(&document)).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("export notepad document failed: {error}");
            (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
        }
    }
}

async fn cleanup_presence(state: &AppState) {
    let _ = sqlx::query(notepad::cleanup_stale_presence_sql())
        .execute(&state.db)
        .await;
}

fn non_empty(value: impl AsRef<str>) -> Option<String> {
    let trimmed = value.as_ref().trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}
