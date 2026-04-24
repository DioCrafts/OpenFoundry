use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Utc;
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
    AppState,
    domain::llm::provider,
    models::prompt_template::{
        CreatePromptTemplateRequest, ListPromptTemplatesResponse, PromptTemplate,
        PromptTemplateRow, PromptVersion, RenderPromptRequest, RenderPromptResponse,
        UpdatePromptTemplateRequest,
    },
};

use super::{ServiceResult, bad_request, db_error, not_found};

async fn load_prompt_row(
    db: &sqlx::PgPool,
    prompt_id: Uuid,
) -> Result<Option<PromptTemplateRow>, sqlx::Error> {
    query_as::<_, PromptTemplateRow>(
        r#"
		SELECT
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions,
			created_at,
			updated_at
		FROM ai_prompt_templates
		WHERE id = $1
		"#,
    )
    .bind(prompt_id)
    .fetch_optional(db)
    .await
}

pub async fn list_prompts(
    State(state): State<AppState>,
) -> ServiceResult<ListPromptTemplatesResponse> {
    let rows = query_as::<_, PromptTemplateRow>(
        r#"
		SELECT
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions,
			created_at,
			updated_at
		FROM ai_prompt_templates
		ORDER BY updated_at DESC, created_at DESC
		"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(ListPromptTemplatesResponse {
        data: rows.into_iter().map(Into::into).collect(),
    }))
}

pub async fn create_prompt(
    State(state): State<AppState>,
    Json(body): Json<CreatePromptTemplateRequest>,
) -> ServiceResult<PromptTemplate> {
    if body.name.trim().is_empty() || body.content.trim().is_empty() {
        return Err(bad_request("prompt name and content are required"));
    }

    let version = PromptVersion {
        version_number: 1,
        content: body.content.trim().to_string(),
        input_variables: body.input_variables,
        notes: body.notes.unwrap_or_else(|| "Initial version".to_string()),
        created_at: Utc::now(),
        created_by: None,
    };

    let row = query_as::<_, PromptTemplateRow>(
        r#"
		INSERT INTO ai_prompt_templates (
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions
		)
		VALUES ($1, $2, $3, $4, 'active', $5, 1, $6)
		RETURNING
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions,
			created_at,
			updated_at
		"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(body.description)
    .bind(body.category)
    .bind(SqlJson(body.tags))
    .bind(SqlJson(vec![version]))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn update_prompt(
    State(state): State<AppState>,
    Path(prompt_id): Path<Uuid>,
    Json(body): Json<UpdatePromptTemplateRequest>,
) -> ServiceResult<PromptTemplate> {
    let Some(current) = load_prompt_row(&state.db, prompt_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("prompt template not found"));
    };

    let current_prompt: PromptTemplate = current.clone().into();
    let mut versions = current.versions.0;

    if body.content.is_some() || body.input_variables.is_some() || body.notes.is_some() {
        let base_version = current_prompt.current_version;
        let next_version_number = versions
            .last()
            .map(|version| version.version_number + 1)
            .unwrap_or(1);

        versions.push(PromptVersion {
            version_number: next_version_number,
            content: body
                .content
                .unwrap_or(base_version.content)
                .trim()
                .to_string(),
            input_variables: body.input_variables.unwrap_or(base_version.input_variables),
            notes: body
                .notes
                .unwrap_or_else(|| format!("Version {next_version_number}")),
            created_at: Utc::now(),
            created_by: None,
        });
    }

    let latest_version_number = versions
        .last()
        .map(|version| version.version_number)
        .unwrap_or(current.latest_version_number);

    let row = query_as::<_, PromptTemplateRow>(
        r#"
		UPDATE ai_prompt_templates
		SET name = $2,
			description = $3,
			category = $4,
			status = $5,
			tags = $6,
			latest_version_number = $7,
			versions = $8,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			category,
			status,
			tags,
			latest_version_number,
			versions,
			created_at,
			updated_at
		"#,
    )
    .bind(prompt_id)
    .bind(body.name.unwrap_or(current_prompt.name))
    .bind(body.description.unwrap_or(current_prompt.description))
    .bind(body.category.unwrap_or(current_prompt.category))
    .bind(body.status.unwrap_or(current_prompt.status))
    .bind(SqlJson(body.tags.unwrap_or(current_prompt.tags)))
    .bind(latest_version_number)
    .bind(SqlJson(versions))
    .fetch_one(&state.db)
    .await
    .map_err(|cause| db_error(&cause))?;

    Ok(Json(row.into()))
}

pub async fn render_prompt(
    State(state): State<AppState>,
    Path(prompt_id): Path<Uuid>,
    Json(body): Json<RenderPromptRequest>,
) -> ServiceResult<RenderPromptResponse> {
    let Some(row) = load_prompt_row(&state.db, prompt_id)
        .await
        .map_err(|cause| db_error(&cause))?
    else {
        return Err(not_found("prompt template not found"));
    };

    let prompt: PromptTemplate = row.into();
    let (rendered_content, missing_variables) = provider::interpolate_template(
        &prompt.current_version.content,
        &body.variables,
        body.strict,
    );

    if body.strict && !missing_variables.is_empty() {
        return Err(bad_request("missing prompt variables"));
    }

    Ok(Json(RenderPromptResponse {
        prompt_id,
        version_number: prompt.current_version.version_number,
        rendered_content,
        missing_variables,
    }))
}
