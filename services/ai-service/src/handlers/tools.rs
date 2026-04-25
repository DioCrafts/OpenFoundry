use axum::{
    Json,
    extract::{Path, State},
};
use serde_json::json;
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
    AppState,
    models::tool::{
        CreateToolRequest, ListToolsResponse, ToolDefinition, ToolRow, UpdateToolRequest,
        supported_execution_modes,
    },
};

use super::{ServiceResult, bad_request, db_error, not_found};

fn validate_execution_mode(mode: &str) -> bool {
    supported_execution_modes()
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(mode))
}

async fn load_tool_row(db: &sqlx::PgPool, tool_id: Uuid) -> Result<Option<ToolRow>, sqlx::Error> {
    query_as::<_, ToolRow>(
        r#"
		SELECT
			id,
			name,
			description,
			category,
			execution_mode,
			execution_config,
			status,
			input_schema,
			output_schema,
			tags,
			created_at,
			updated_at
		FROM ai_tools
		WHERE id = $1
		"#,
    )
    .bind(tool_id)
    .fetch_optional(db)
    .await
}

pub async fn list_tools(State(state): State<AppState>) -> ServiceResult<ListToolsResponse> {
    let rows = query_as::<_, ToolRow>(
        r#"
		SELECT
			id,
			name,
			description,
			category,
			execution_mode,
			execution_config,
			status,
			input_schema,
			output_schema,
			tags,
			created_at,
			updated_at
		FROM ai_tools
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListToolsResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_tool(
    State(state): State<AppState>,
    Json(body): Json<CreateToolRequest>,
) -> ServiceResult<ToolDefinition> {
    if body.name.trim().is_empty() {
        return Err(bad_request("tool name is required"));
    }
    if !validate_execution_mode(&body.execution_mode) {
        return Err(bad_request(format!(
            "unsupported tool execution_mode '{}'",
            body.execution_mode
        )));
    }

    let row = query_as::<_, ToolRow>(
        r#"
		INSERT INTO ai_tools (
			id,
			name,
			description,
			category,
			execution_mode,
			execution_config,
			status,
			input_schema,
			output_schema,
			tags
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		RETURNING
			id,
			name,
			description,
			category,
			execution_mode,
			execution_config,
			status,
			input_schema,
			output_schema,
			tags,
			created_at,
			updated_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description)
    .bind(body.category)
    .bind(body.execution_mode)
    .bind(SqlJson(if body.execution_config.is_null() {
        json!({})
    } else {
        body.execution_config
    }))
    .bind(body.status)
    .bind(SqlJson(if body.input_schema.is_null() {
        json!({})
    } else {
        body.input_schema
    }))
    .bind(SqlJson(if body.output_schema.is_null() {
        json!({})
    } else {
        body.output_schema
    }))
    .bind(SqlJson(body.tags))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_tool(
    State(state): State<AppState>,
    Path(tool_id): Path<Uuid>,
    Json(body): Json<UpdateToolRequest>,
) -> ServiceResult<ToolDefinition> {
    let Some(current) = load_tool_row(&state.db, tool_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("tool not found"));
    };

    let tool: ToolDefinition = current.into();
    if let Some(execution_mode) = body.execution_mode.as_ref() {
        if !validate_execution_mode(execution_mode) {
            return Err(bad_request(format!(
                "unsupported tool execution_mode '{}'",
                execution_mode
            )));
        }
    }
    let row = query_as::<_, ToolRow>(
        r#"
		UPDATE ai_tools
		SET name = $2,
			description = $3,
			category = $4,
			execution_mode = $5,
			execution_config = $6,
			status = $7,
			input_schema = $8,
			output_schema = $9,
			tags = $10,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			category,
			execution_mode,
			execution_config,
			status,
			input_schema,
			output_schema,
			tags,
			created_at,
			updated_at
		"#,
    )
    .bind(tool_id)
    .bind(body.name.unwrap_or(tool.name))
    .bind(body.description.unwrap_or(tool.description))
    .bind(body.category.unwrap_or(tool.category))
    .bind(body.execution_mode.unwrap_or(tool.execution_mode))
    .bind(SqlJson(
        body.execution_config.unwrap_or(tool.execution_config),
    ))
    .bind(body.status.unwrap_or(tool.status))
    .bind(SqlJson(body.input_schema.unwrap_or(tool.input_schema)))
    .bind(SqlJson(body.output_schema.unwrap_or(tool.output_schema)))
    .bind(SqlJson(body.tags.unwrap_or(tool.tags)))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}
