use axum::{
	extract::{Path, State},
	Json,
};
use chrono::Utc;
use sqlx::{query_as, types::Json as SqlJson};
use uuid::Uuid;

use crate::{
	domain::rag,
	models::knowledge_base::{
		CreateKnowledgeBaseRequest, CreateKnowledgeDocumentRequest, KnowledgeBase,
		KnowledgeBaseRow, KnowledgeDocument, KnowledgeDocumentRow, ListKnowledgeBasesResponse,
		ListKnowledgeDocumentsResponse, SearchKnowledgeBaseRequest, SearchKnowledgeBaseResponse,
		UpdateKnowledgeBaseRequest,
	},
	AppState,
};

use super::{bad_request, db_error, not_found, ServiceResult};

async fn load_knowledge_base_row(
	db: &sqlx::PgPool,
	knowledge_base_id: Uuid,
) -> Result<Option<KnowledgeBaseRow>, sqlx::Error> {
	query_as::<_, KnowledgeBaseRow>(
		r#"
		SELECT
			id,
			name,
			description,
			status,
			embedding_provider,
			chunking_strategy,
			tags,
			document_count,
			chunk_count,
			created_at,
			updated_at
		FROM ai_knowledge_bases
		WHERE id = $1
		"#,
	)
	.bind(knowledge_base_id)
	.fetch_optional(db)
	.await
}

pub async fn list_knowledge_bases(
	State(state): State<AppState>,
) -> ServiceResult<ListKnowledgeBasesResponse> {
	let rows = query_as::<_, KnowledgeBaseRow>(
		r#"
		SELECT
			id,
			name,
			description,
			status,
			embedding_provider,
			chunking_strategy,
			tags,
			document_count,
			chunk_count,
			created_at,
			updated_at
		FROM ai_knowledge_bases
		ORDER BY updated_at DESC, created_at DESC
		"#,
	)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(ListKnowledgeBasesResponse {
		data: rows.into_iter().map(Into::into).collect(),
	}))
}

pub async fn create_knowledge_base(
	State(state): State<AppState>,
	Json(body): Json<CreateKnowledgeBaseRequest>,
) -> ServiceResult<KnowledgeBase> {
	if body.name.trim().is_empty() {
		return Err(bad_request("knowledge base name is required"));
	}

	let row = query_as::<_, KnowledgeBaseRow>(
		r#"
		INSERT INTO ai_knowledge_bases (
			id,
			name,
			description,
			status,
			embedding_provider,
			chunking_strategy,
			tags,
			document_count,
			chunk_count
		)
		VALUES ($1, $2, $3, $4, $5, $6, $7, 0, 0)
		RETURNING
			id,
			name,
			description,
			status,
			embedding_provider,
			chunking_strategy,
			tags,
			document_count,
			chunk_count,
			created_at,
			updated_at
		"#,
	)
	.bind(Uuid::now_v7())
	.bind(body.name.trim())
	.bind(body.description)
	.bind(body.status)
	.bind(body.embedding_provider)
	.bind(body.chunking_strategy)
	.bind(SqlJson(body.tags))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn update_knowledge_base(
	State(state): State<AppState>,
	Path(knowledge_base_id): Path<Uuid>,
	Json(body): Json<UpdateKnowledgeBaseRequest>,
) -> ServiceResult<KnowledgeBase> {
	let Some(current) = load_knowledge_base_row(&state.db, knowledge_base_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("knowledge base not found"));
	};

	let knowledge_base: KnowledgeBase = current.into();
	let row = query_as::<_, KnowledgeBaseRow>(
		r#"
		UPDATE ai_knowledge_bases
		SET name = $2,
			description = $3,
			status = $4,
			embedding_provider = $5,
			chunking_strategy = $6,
			tags = $7,
			updated_at = NOW()
		WHERE id = $1
		RETURNING
			id,
			name,
			description,
			status,
			embedding_provider,
			chunking_strategy,
			tags,
			document_count,
			chunk_count,
			created_at,
			updated_at
		"#,
	)
	.bind(knowledge_base_id)
	.bind(body.name.unwrap_or(knowledge_base.name))
	.bind(body.description.unwrap_or(knowledge_base.description))
	.bind(body.status.unwrap_or(knowledge_base.status))
	.bind(body.embedding_provider.unwrap_or(knowledge_base.embedding_provider))
	.bind(body.chunking_strategy.unwrap_or(knowledge_base.chunking_strategy))
	.bind(SqlJson(body.tags.unwrap_or(knowledge_base.tags)))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn list_documents(
	State(state): State<AppState>,
	Path(knowledge_base_id): Path<Uuid>,
) -> ServiceResult<ListKnowledgeDocumentsResponse> {
	load_knowledge_base_row(&state.db, knowledge_base_id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| not_found("knowledge base not found"))?;

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
		ORDER BY updated_at DESC, created_at DESC
		"#,
	)
	.bind(knowledge_base_id)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(ListKnowledgeDocumentsResponse {
		data: rows.into_iter().map(Into::into).collect(),
	}))
}

pub async fn create_document(
	State(state): State<AppState>,
	Path(knowledge_base_id): Path<Uuid>,
	Json(body): Json<CreateKnowledgeDocumentRequest>,
) -> ServiceResult<KnowledgeDocument> {
	let Some(knowledge_base_row) = load_knowledge_base_row(&state.db, knowledge_base_id)
		.await
		.map_err(|cause| db_error(&cause))?
	else {
		return Err(not_found("knowledge base not found"));
	};

	if body.title.trim().is_empty() || body.content.trim().is_empty() {
		return Err(bad_request("document title and content are required"));
	}

	let document_id = Uuid::now_v7();
	let knowledge_base: KnowledgeBase = knowledge_base_row.into();
	let chunks = rag::indexer::index_document(document_id, &body.content, &knowledge_base.chunking_strategy);

	let row = query_as::<_, KnowledgeDocumentRow>(
		r#"
		INSERT INTO ai_knowledge_documents (
			id,
			knowledge_base_id,
			title,
			content,
			source_uri,
			metadata,
			status,
			chunk_count,
			chunks
		)
		VALUES ($1, $2, $3, $4, $5, $6, 'indexed', $7, $8)
		RETURNING
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
		"#,
	)
	.bind(document_id)
	.bind(knowledge_base_id)
	.bind(body.title.trim())
	.bind(body.content)
	.bind(body.source_uri)
	.bind(SqlJson(body.metadata))
	.bind(chunks.len() as i32)
	.bind(SqlJson(chunks.clone()))
	.fetch_one(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	sqlx::query(
		"UPDATE ai_knowledge_bases SET document_count = document_count + 1, chunk_count = chunk_count + $2, updated_at = NOW() WHERE id = $1",
	)
	.bind(knowledge_base_id)
	.bind(chunks.len() as i64)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	Ok(Json(row.into()))
}

pub async fn search_knowledge_base(
	State(state): State<AppState>,
	Path(knowledge_base_id): Path<Uuid>,
	Json(body): Json<SearchKnowledgeBaseRequest>,
) -> ServiceResult<SearchKnowledgeBaseResponse> {
	if body.query.trim().is_empty() {
		return Err(bad_request("search query is required"));
	}

	load_knowledge_base_row(&state.db, knowledge_base_id)
		.await
		.map_err(|cause| db_error(&cause))?
		.ok_or_else(|| not_found("knowledge base not found"))?;

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
		ORDER BY updated_at DESC, created_at DESC
		"#,
	)
	.bind(knowledge_base_id)
	.fetch_all(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

	let documents = rows.into_iter().map(Into::into).collect::<Vec<_>>();
	let results = rag::retriever::search(&body.query, &documents, body.top_k, body.min_score);

	Ok(Json(SearchKnowledgeBaseResponse {
		knowledge_base_id,
		query: body.query,
		results,
		retrieved_at: Utc::now(),
	}))
}
