use axum::{Json, extract::State};
use chrono::Utc;

use crate::{
    AppState,
    handlers::{
        ServiceResult, db_error, internal_error, load_all_repositories, load_merge_requests,
    },
    models::{
        ListResponse,
        repository::{
            CreateRepositoryRequest, RepositoryDefinition, RepositoryOverview,
            UpdateRepositoryRequest,
        },
    },
};

pub async fn get_overview(State(state): State<AppState>) -> ServiceResult<RepositoryOverview> {
    let repositories = load_all_repositories(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let merge_requests = load_merge_requests(&state.db, None)
        .await
        .map_err(|cause| db_error(&cause))?;

    Ok(Json(RepositoryOverview {
        repository_count: repositories.len(),
        private_repository_count: repositories
            .iter()
            .filter(|repo| {
                repo.visibility == crate::models::repository::RepositoryVisibility::Private
            })
            .count(),
        package_kind_mix: repositories
            .iter()
            .map(|repo| repo.package_kind.label().to_string())
            .collect(),
        open_merge_request_count: merge_requests
            .iter()
            .filter(|mr| mr.status == crate::models::merge_request::MergeRequestStatus::Open)
            .count(),
        latest_merge_request: merge_requests.first().cloned(),
    }))
}

pub async fn list_repositories(
    State(state): State<AppState>,
) -> ServiceResult<ListResponse<RepositoryDefinition>> {
    let repositories = load_all_repositories(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    Ok(Json(ListResponse {
        items: repositories,
    }))
}

pub async fn create_repository(
    State(state): State<AppState>,
    Json(request): Json<CreateRepositoryRequest>,
) -> ServiceResult<RepositoryDefinition> {
    if request.name.trim().is_empty() {
        return Err(crate::handlers::bad_request("repository name is required"));
    }

    crate::domain::git::ensure_storage_root(&state.repo_storage_root)
        .map_err(|cause| internal_error(cause.to_string()))?;

    let id = uuid::Uuid::now_v7();
    let now = Utc::now();
    let tags =
        serde_json::to_value(&request.tags).map_err(|cause| internal_error(cause.to_string()))?;
    let repository = RepositoryDefinition {
        id,
        name: request.name.clone(),
        slug: request.slug.clone(),
        description: request.description.clone(),
        owner: request.owner.clone(),
        default_branch: request.default_branch.clone(),
        visibility: request.visibility,
        object_store_backend: request.object_store_backend.clone(),
        package_kind: request.package_kind,
        tags: request.tags.clone(),
        settings: request.settings.clone(),
        created_at: now,
        updated_at: now,
    };

    let (head_sha, _) =
        crate::domain::git::initialize_repository(&state.repo_storage_root, &repository)
            .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"INSERT INTO code_repositories (id, name, slug, description, owner, default_branch, visibility, object_store_backend, package_kind, tags, settings, created_at, updated_at)
		 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10::jsonb, $11::jsonb, $12, $13)",
	)
	.bind(id)
	.bind(&request.name)
	.bind(&request.slug)
	.bind(&request.description)
	.bind(&request.owner)
	.bind(&request.default_branch)
	.bind(request.visibility.as_str())
	.bind(&request.object_store_backend)
	.bind(request.package_kind.as_str())
	.bind(tags)
	.bind(request.settings)
	.bind(now)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    sqlx::query(
		"INSERT INTO code_repository_branches (id, repository_id, name, head_sha, base_branch, is_default, protected, ahead_by, pending_reviews, updated_at)
		 VALUES ($1, $2, $3, $4, NULL, true, true, 0, 0, $5)",
	)
	.bind(uuid::Uuid::now_v7())
	.bind(id)
	.bind(&request.default_branch)
	.bind(&head_sha)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = crate::handlers::load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("created repository could not be reloaded"))?;
    let repository =
        RepositoryDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(repository))
}

pub async fn update_repository(
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
    State(state): State<AppState>,
    Json(request): Json<UpdateRepositoryRequest>,
) -> ServiceResult<RepositoryDefinition> {
    let row = crate::handlers::load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| crate::handlers::not_found("repository not found"))?;
    let mut repository =
        RepositoryDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;

    if let Some(name) = request.name {
        repository.name = name;
    }
    if let Some(slug) = request.slug {
        repository.slug = slug;
    }
    if let Some(description) = request.description {
        repository.description = description;
    }
    if let Some(owner) = request.owner {
        repository.owner = owner;
    }
    if let Some(default_branch) = request.default_branch {
        repository.default_branch = default_branch;
    }
    if let Some(visibility) = request.visibility {
        repository.visibility = visibility;
    }
    if let Some(object_store_backend) = request.object_store_backend {
        repository.object_store_backend = object_store_backend;
    }
    if let Some(package_kind) = request.package_kind {
        repository.package_kind = package_kind;
    }
    if let Some(tags) = request.tags {
        repository.tags = tags;
    }
    if let Some(settings) = request.settings {
        repository.settings = settings;
    }

    let now = Utc::now();
    let tags = serde_json::to_value(&repository.tags)
        .map_err(|cause| internal_error(cause.to_string()))?;

    sqlx::query(
		"UPDATE code_repositories
		 SET name = $2, slug = $3, description = $4, owner = $5, default_branch = $6, visibility = $7, object_store_backend = $8, package_kind = $9, tags = $10::jsonb, settings = $11::jsonb, updated_at = $12
		 WHERE id = $1",
	)
	.bind(id)
	.bind(&repository.name)
	.bind(&repository.slug)
	.bind(&repository.description)
	.bind(&repository.owner)
	.bind(&repository.default_branch)
	.bind(repository.visibility.as_str())
	.bind(&repository.object_store_backend)
	.bind(repository.package_kind.as_str())
	.bind(tags)
	.bind(&repository.settings)
	.bind(now)
	.execute(&state.db)
	.await
	.map_err(|cause| db_error(&cause))?;

    let row = crate::handlers::load_repository_row(&state.db, id)
        .await
        .map_err(|cause| db_error(&cause))?
        .ok_or_else(|| internal_error("updated repository could not be reloaded"))?;
    let repository =
        RepositoryDefinition::try_from(row).map_err(|cause| internal_error(cause.to_string()))?;
    Ok(Json(repository))
}
