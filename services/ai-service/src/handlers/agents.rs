use axum::{
	extract::{Path, State},
	Json,
};
use chrono::Utc;
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
	domain::{agents, rag},
	models::{
		agent::{
			AgentDefinition, AgentExecutionResponse, AgentRow, CreateAgentRequest,
			ExecuteAgentRequest, ListAgentsResponse, UpdateAgentRequest,
		},
		knowledge_base::KnowledgeDocumentRow,
		tool::{ToolDefinition, ToolRow},
	},
	AppState,
};

use super::{bad_request, db_error, not_found, ServiceResult};

async fn load_agent_row(db: &sqlx::PgPool, agent_id: Uuid) -> Result<Option<AgentRow>, sqlx::Error> {
	query_as::<_, AgentRow>(
		r#"
		SELECT
			id,
			name,
			description,
			status,
			system_prompt,
			objective,
			tool_ids,
			planning_strategy,
			max_iterations,
			memory,
			last_execution_at,
			created_at,
			updated_at
		FROM ai_agents
		WHERE id = $1
		"#,
	)
	.bind(agent_id)
	.fetch_optional(db)
	.await
}

async fn load_tools(db: &sqlx::PgPool, tool_ids: &[Uuid]) -> Result<Vec<ToolDefinition>, sqlx::Error> {
	let mut tools = Vec::new();
	for tool_id in tool_ids {
		if let Some(row) = query_as::<_, ToolRow>(
			r#"
			SELECT
				id,
				name,
				description,
				category,
				execution_mode,
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
		.bind(*tool_id)
		.fetch_optional(db)
		.await?
		{
			tools.push(row.into());
		}
	}

	Ok(tools)
}

async fn load_documents(
	db: &sqlx::PgPool,
	knowledge_base_id: Uuid,
) -> Result<Vec<crate::models::knowledge_base::KnowledgeDocument>, sqlx::Error> {
	let rows = query_as::<_, KnowledgeDocumentRow>(
		r#"
		SELECT
			id,
			knowledge_base_id,
			title,
			content,
			source_uri,
			metadata,
			status,
			chunk_count,
			chunks,
			created_at,
			updated_at
		FROM ai_knowledge_documents
		WHERE knowledge_base_id = $1
		ORDER BY updated_at DESC
		"#,
	)
	.bind(knowledge_base_id)
	.fetch_all(db)
	.await?;

	Ok(rows.into_iter().map(Into::into).collect())
}

pub async fn list_agents(State(state): State<AppState>) -> ServiceResult<ListAgentsResponse> {
	let rows = query_as::<_, AgentRow>(
		r#"
		SELECT
			id,
			name,
			description,
			status,
			system_prompt,
			objective,
			tool_ids,
			planning_strategy,
			max_iterations,
			memory,
			last_execution_at,
			created_at,
			updated_at
		FROM ai_agents
		ORDER BY updated_at DESC, created_at DESC
		"#,
	)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(ListAgentsResponse {
		data: rows.into_iter().map(Into::into).collect(),
	}))
}

pub async fn create_agent(
	State(state): State<AppState>,
	Json(body): Json<CreateAgentRequest>,
) -> ServiceResult<AgentDefinition> {
	if body.name.trim().is_empty() {
		return Err(bad_request("agent name is required"));
	}

	let row = query_as::<_, AgentRow>(
		r#"
		INSERT INTO ai_agents (
			id,
			name,
			description,
			status,
			system_prompt,
			objective,
			tool_ids,
			planning_strategy,
			max_iterations,
			memory,
			last_execution_at
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NULL)
		RETURNING
			id,
			name,
			description,
			status,
			system_prompt,
			objective,
			tool_ids,
			planning_strategy,
			max_iterations,
			memory,
			last_execution_at,
			created_at,
			updated_at
		"#,
	)
	.bind(Uuid::now_v7())
	.bind(body.name.trim())
	.bind(body.description)
	.bind(body.status)
	.bind(body.system_prompt)
	.bind(body.objective)
	.bind(SqlJson(body.tool_ids))
	.bind(body.planning_strategy)
	.bind(body.max_iterations)
	.bind(SqlJson(crate::models::agent::AgentMemorySnapshot::default()))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn update_agent(
	State(state): State<AppState>,
	Path(agent_id): Path<Uuid>,
	Json(body): Json<UpdateAgentRequest>,
) -> ServiceResult<AgentDefinition> {
	let Some(current) = load_agent_row(&state.db, agent_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("agent not found"));
	};

	let agent: AgentDefinition = current.into();
	let row = query_as::<_, AgentRow>(
		r#"
		UPDATE ai_agents
		SET name = $2,
			description = $3,
			status = $4,
			system_prompt = $5,
			objective = $6,
			tool_ids = $7,
			planning_strategy = $8,
			max_iterations = $9,
			memory = $10,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			status,
			system_prompt,
			objective,
			tool_ids,
			planning_strategy,
			max_iterations,
			memory,
			last_execution_at,
			created_at,
			updated_at
		"#,
	)
	.bind(agent_id)
	.bind(body.name.unwrap_or(agent.name))
	.bind(body.description.unwrap_or(agent.description))
	.bind(body.status.unwrap_or(agent.status))
	.bind(body.system_prompt.unwrap_or(agent.system_prompt))
	.bind(body.objective.unwrap_or(agent.objective))
	.bind(SqlJson(body.tool_ids.unwrap_or(agent.tool_ids)))
	.bind(body.planning_strategy.unwrap_or(agent.planning_strategy))
	.bind(body.max_iterations.unwrap_or(agent.max_iterations))
	.bind(SqlJson(body.memory.unwrap_or(agent.memory)))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn execute_agent(
	State(state): State<AppState>,
	Path(agent_id): Path<Uuid>,
	Json(body): Json<ExecuteAgentRequest>,
) -> ServiceResult<AgentExecutionResponse> {
	if body.user_message.trim().is_empty() {
		return Err(bad_request("agent execution requires a user message"));
	}

	let Some(current) = load_agent_row(&state.db, agent_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("agent not found"));
	};

	let agent: AgentDefinition = current.into();
	let tools = load_tools(&state.db, &agent.tool_ids)
		.await
		.map_err(|cause| db_error(&cause))?;

	let knowledge_hits = if let Some(knowledge_base_id) = body.knowledge_base_id {
		let documents = load_documents(&state.db, knowledge_base_id)
			.await
			.map_err(|cause| db_error(&cause))?;
		rag::retriever::search(&body.user_message, &documents, 4, 0.55)
	} else {
		Vec::new()
	};

	let objective = body.objective.unwrap_or_else(|| {
		if agent.objective.trim().is_empty() {
			body.user_message.clone()
		} else {
			agent.objective.clone()
		}
	});

	let steps = agents::planner::build_plan(&agent, &objective, &tools, &knowledge_hits);
	let traces = agents::executor::execute_plan(&steps, &tools, &body.user_message, &knowledge_hits);
	let final_response = traces
		.last()
		.map(|trace| trace.observation.clone())
		.unwrap_or_else(|| "Agent execution completed without traces.".to_string());
	let updated_memory = agents::memory::update_memory(
		&agent.memory,
		&body.user_message,
		&final_response,
		&knowledge_hits,
	);

	sqlx::query(
		"UPDATE ai_agents SET memory = $2, last_execution_at = NOW(), updated_at = NOW() WHERE id = $1",
	)
	.bind(agent_id)
	.bind(SqlJson(updated_memory))
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(AgentExecutionResponse {
		agent_id,
		steps,
		traces: traces.clone(),
		final_response,
		used_tool_names: traces
			.into_iter()
			.filter_map(|trace| trace.tool_name)
			.collect(),
		executed_at: Utc::now(),
	}))
}
