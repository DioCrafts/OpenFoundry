use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::AppState;
use crate::domain::{
    environment,
    kernel::{KernelExecutionContext, KernelWorkspaceFileContext},
};
use crate::models::cell::{Cell, CellOutput, ExecuteCellRequest};
use crate::models::session::Session;
use auth_middleware::layer::AuthUser;

async fn update_session_status(db: &sqlx::PgPool, session_id: Uuid, status: &str) {
    let _ = sqlx::query("UPDATE sessions SET status = $2, last_activity = NOW() WHERE id = $1")
        .bind(session_id)
        .bind(status)
        .execute(db)
        .await;
}

async fn load_session(db: &sqlx::PgPool, session_id: Uuid) -> Result<Option<Session>, sqlx::Error> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = $1")
        .bind(session_id)
        .fetch_optional(db)
        .await
}

async fn load_kernel_context(
    state: &AppState,
    notebook_id: Uuid,
) -> Result<KernelExecutionContext, String> {
    let workspace_dir = environment::notebook_workspace_root(&state.data_dir, notebook_id)
        .to_string_lossy()
        .to_string();
    let workspace_files = environment::list_workspace_files(&state.data_dir, notebook_id).await?;

    Ok(KernelExecutionContext {
        notebook_id,
        workspace_dir: Some(workspace_dir),
        workspace_files: workspace_files
            .into_iter()
            .map(|file| KernelWorkspaceFileContext {
                path: file.path,
                content: file.content,
            })
            .collect(),
    })
}

pub async fn execute_cell(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((notebook_id, cell_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<ExecuteCellRequest>,
) -> impl IntoResponse {
    let cell = match sqlx::query_as::<_, Cell>("SELECT * FROM cells WHERE id = $1")
        .bind(cell_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(c)) => c,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };

    if cell.cell_type == "markdown" {
        return Json(serde_json::json!(CellOutput {
            output_type: "text".into(),
            content: serde_json::json!(cell.source),
            execution_count: 0,
        }))
        .into_response();
    }

    let session = match body.session_id {
        Some(session_id) => match load_session(&state.db, session_id).await {
            Ok(Some(session)) => {
                if session.status == "dead" {
                    return (StatusCode::CONFLICT, "session is stopped").into_response();
                }
                if session.kernel != cell.kernel {
                    return (
                        StatusCode::BAD_REQUEST,
                        "session kernel does not match cell kernel",
                    )
                        .into_response();
                }
                if let Err(error) = state
                    .kernel_manager
                    .ensure_session(session.id, &session.kernel)
                    .await
                {
                    return (StatusCode::INTERNAL_SERVER_ERROR, error).into_response();
                }
                Some(session)
            }
            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        },
        None => None,
    };

    if let Some(session) = &session {
        update_session_status(&state.db, session.id, "busy").await;
    }

    let context = match load_kernel_context(&state, notebook_id).await {
        Ok(context) => context,
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    };

    let result = state
        .kernel_manager
        .execute(
            &cell.kernel,
            &cell.source,
            body.session_id,
            &claims,
            &context,
        )
        .await;

    let (output, exec_count) = match result {
        Ok(result) => {
            let count = cell.execution_count.unwrap_or(0) + 1;
            let cell_output = CellOutput {
                output_type: result.output_type,
                content: result.content,
                execution_count: count,
            };
            (cell_output, count)
        }
        Err(err) => {
            let count = cell.execution_count.unwrap_or(0) + 1;
            let cell_output = CellOutput {
                output_type: "error".into(),
                content: serde_json::json!({ "error": err }),
                execution_count: count,
            };
            (cell_output, count)
        }
    };

    // Persist output
    let output_json = serde_json::to_value(&output).ok();
    let _ = sqlx::query(
        "UPDATE cells SET last_output = $2, execution_count = $3, updated_at = NOW() WHERE id = $1",
    )
    .bind(cell_id)
    .bind(&output_json)
    .bind(exec_count)
    .execute(&state.db)
    .await;

    if let Some(session) = &session {
        update_session_status(&state.db, session.id, "idle").await;
    }

    Json(serde_json::json!(output)).into_response()
}

pub async fn execute_all_cells(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(notebook_id): Path<Uuid>,
    Json(body): Json<ExecuteCellRequest>,
) -> impl IntoResponse {
    let shared_session = match body.session_id {
        Some(session_id) => match load_session(&state.db, session_id).await {
            Ok(session) => session,
            Err(error) => {
                return (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response();
            }
        },
        None => None,
    };

    if let Some(session) = &shared_session {
        update_session_status(&state.db, session.id, "busy").await;
    }

    let cells = sqlx::query_as::<_, Cell>(
        "SELECT * FROM cells WHERE notebook_id = $1 AND cell_type = 'code' ORDER BY position ASC",
    )
    .bind(notebook_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let context = match load_kernel_context(&state, notebook_id).await {
        Ok(context) => context,
        Err(error) => return (StatusCode::INTERNAL_SERVER_ERROR, error).into_response(),
    };

    let mut results = Vec::new();
    for cell in &cells {
        let session_id = shared_session
            .as_ref()
            .filter(|session| session.kernel == cell.kernel && session.status != "dead")
            .map(|session| session.id);

        let result = state
            .kernel_manager
            .execute(&cell.kernel, &cell.source, session_id, &claims, &context)
            .await;
        let count = cell.execution_count.unwrap_or(0) + 1;

        let output = match result {
            Ok(result) => CellOutput {
                output_type: result.output_type,
                content: result.content,
                execution_count: count,
            },
            Err(err) => CellOutput {
                output_type: "error".into(),
                content: serde_json::json!({ "error": err }),
                execution_count: count,
            },
        };

        let output_json = serde_json::to_value(&output).ok();
        let _ = sqlx::query(
            "UPDATE cells SET last_output = $2, execution_count = $3, updated_at = NOW() WHERE id = $1",
        )
        .bind(cell.id)
        .bind(&output_json)
        .bind(count)
        .execute(&state.db)
        .await;

        results.push(serde_json::json!({ "cell_id": cell.id, "output": output }));
    }

    if let Some(session) = &shared_session {
        update_session_status(&state.db, session.id, "idle").await;
    }

    Json(serde_json::json!({ "results": results })).into_response()
}
