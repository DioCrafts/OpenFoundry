use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::{
    AppState,
    models::{branch::DatasetBranch, dataset::Dataset, version::DatasetVersion, view::DatasetView},
};

#[derive(Debug, Deserialize)]
pub struct FilesQuery {
    pub prefix: Option<String>,
    pub path: Option<String>,
}

pub async fn list_files(
    State(state): State<AppState>,
    Path(dataset_id): Path<Uuid>,
    Query(query): Query<FilesQuery>,
) -> impl IntoResponse {
    let dataset = match sqlx::query_as::<_, Dataset>("SELECT * FROM datasets WHERE id = $1")
        .bind(dataset_id)
        .fetch_optional(&state.db)
        .await
    {
        Ok(Some(dataset)) => dataset,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => {
            tracing::error!("dataset files lookup failed: {error}");
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
    };

    let versions = sqlx::query_as::<_, DatasetVersion>(
        "SELECT * FROM dataset_versions WHERE dataset_id = $1 ORDER BY version DESC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let branches = sqlx::query_as::<_, DatasetBranch>(
        "SELECT * FROM dataset_branches WHERE dataset_id = $1 ORDER BY is_default DESC, name ASC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let views = sqlx::query_as::<_, DatasetView>(
        "SELECT * FROM dataset_views WHERE dataset_id = $1 ORDER BY created_at DESC",
    )
    .bind(dataset_id)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let requested_path = query
        .path
        .as_deref()
        .or(query.prefix.as_deref())
        .map(|value| value.trim().trim_matches('/').to_string())
        .unwrap_or_default();

    match build_filesystem_response(
        &state,
        &dataset,
        &versions,
        &branches,
        &views,
        &requested_path,
    )
    .await
    {
        Ok(response) => Json(response).into_response(),
        Err(error) => {
            tracing::error!("dataset filesystem listing failed: {error}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

async fn build_filesystem_response(
    state: &AppState,
    dataset: &Dataset,
    versions: &[DatasetVersion],
    branches: &[DatasetBranch],
    views: &[DatasetView],
    requested_path: &str,
) -> Result<serde_json::Value, String> {
    let breadcrumbs = build_breadcrumbs(requested_path);

    let entries = if requested_path.is_empty() {
        root_entries(dataset, versions, branches, views)
    } else if requested_path == "current" {
        version_object_entry(
            state,
            requested_path,
            &format!("{}/v{}", dataset.storage_path, dataset.current_version),
            json!({
                "scope": "current",
                "version": dataset.current_version,
                "active_branch": dataset.active_branch,
            }),
        )
        .await?
        .into_iter()
        .collect()
    } else if requested_path == "versions" {
        versions
            .iter()
            .map(|version| {
                logical_dir_entry(
                    &format!("versions/v{}", version.version),
                    &format!("v{}", version.version),
                    json!({
                        "scope": "version",
                        "version": version.version,
                        "row_count": version.row_count,
                        "size_bytes": version.size_bytes,
                    }),
                )
            })
            .collect()
    } else if let Some(version) = requested_path.strip_prefix("versions/v") {
        let version = version.parse::<i32>().map_err(|error| error.to_string())?;
        let Some(version_record) = versions.iter().find(|item| item.version == version) else {
            return Ok(json!({
                "dataset_id": dataset.id,
                "requested_path": requested_path,
                "root": dataset.storage_path,
                "entries": [],
                "items": [],
                "breadcrumbs": breadcrumbs,
            }));
        };
        version_object_entry(
            state,
            requested_path,
            &version_record.storage_path,
            json!({
                "scope": "version",
                "version": version_record.version,
                "row_count": version_record.row_count,
            }),
        )
        .await?
        .into_iter()
        .collect()
    } else if requested_path == "branches" {
        branches
            .iter()
            .map(|branch| {
                logical_dir_entry(
                    &format!("branches/{}", branch.name),
                    &branch.name,
                    json!({
                        "scope": "branch",
                        "version": branch.version,
                        "is_default": branch.is_default,
                    }),
                )
            })
            .collect()
    } else if let Some(branch_name) = requested_path.strip_prefix("branches/") {
        let Some(branch) = branches.iter().find(|item| item.name == branch_name) else {
            return Ok(json!({
                "dataset_id": dataset.id,
                "requested_path": requested_path,
                "root": dataset.storage_path,
                "entries": [],
                "items": [],
                "breadcrumbs": breadcrumbs,
            }));
        };
        let storage_path = if branch.version == dataset.current_version {
            format!("{}/v{}", dataset.storage_path, dataset.current_version)
        } else {
            versions
                .iter()
                .find(|item| item.version == branch.version)
                .map(|item| item.storage_path.clone())
                .unwrap_or_else(|| format!("{}/v{}", dataset.storage_path, branch.version))
        };
        version_object_entry(
            state,
            requested_path,
            &storage_path,
            json!({
                "scope": "branch",
                "branch": branch.name,
                "version": branch.version,
            }),
        )
        .await?
        .into_iter()
        .collect()
    } else if requested_path == "views" {
        views
            .iter()
            .map(|view| {
                logical_dir_entry(
                    &format!("views/{}", view.name),
                    &view.name,
                    json!({
                        "scope": "view",
                        "view_id": view.id,
                        "materialized": view.materialized,
                        "current_version": view.current_version,
                    }),
                )
            })
            .collect()
    } else if let Some(view_name) = requested_path.strip_prefix("views/") {
        let Some(view) = views.iter().find(|item| item.name == view_name) else {
            return Ok(json!({
                "dataset_id": dataset.id,
                "requested_path": requested_path,
                "root": dataset.storage_path,
                "entries": [],
                "items": [],
                "breadcrumbs": breadcrumbs,
            }));
        };

        if let Some(storage_prefix) = view_prefix_path(view, &dataset.storage_path) {
            actual_object_entries(state, requested_path, &storage_prefix).await?
        } else {
            Vec::new()
        }
    } else {
        let actual_prefix = format!("{}/{}", dataset.storage_path, requested_path);
        actual_object_entries(state, requested_path, &actual_prefix).await?
    };

    let items = entries
        .iter()
        .filter(|entry| entry["entry_type"] == "file")
        .cloned()
        .collect::<Vec<_>>();

    Ok(json!({
        "dataset_id": dataset.id,
        "requested_path": requested_path,
        "root": dataset.storage_path,
        "current_version": dataset.current_version,
        "active_branch": dataset.active_branch,
        "entries": entries,
        "items": items,
        "breadcrumbs": breadcrumbs,
        "sections": {
            "versions": versions.len(),
            "branches": branches.len(),
            "views": views.len(),
        }
    }))
}

fn root_entries(
    dataset: &Dataset,
    versions: &[DatasetVersion],
    branches: &[DatasetBranch],
    views: &[DatasetView],
) -> Vec<serde_json::Value> {
    let mut entries = vec![
        logical_dir_entry(
            "current",
            "current",
            json!({
                "scope": "current",
                "version": dataset.current_version,
                "active_branch": dataset.active_branch,
            }),
        ),
        logical_dir_entry(
            "versions",
            "versions",
            json!({
                "scope": "versions",
                "count": versions.len(),
            }),
        ),
        logical_dir_entry(
            "branches",
            "branches",
            json!({
                "scope": "branches",
                "count": branches.len(),
            }),
        ),
    ];
    if !views.is_empty() {
        entries.push(logical_dir_entry(
            "views",
            "views",
            json!({
                "scope": "views",
                "count": views.len(),
            }),
        ));
    }
    entries
}

async fn version_object_entry(
    state: &AppState,
    requested_path: &str,
    storage_path: &str,
    metadata: serde_json::Value,
) -> Result<Vec<serde_json::Value>, String> {
    match state.storage.head(storage_path).await {
        Ok(meta) => Ok(vec![json!({
            "entry_type": "file",
            "name": storage_path.rsplit('/').next().unwrap_or("data"),
            "path": format!("{requested_path}/{}", storage_path.rsplit('/').next().unwrap_or("data")).trim_matches('/'),
            "storage_path": meta.path,
            "size_bytes": meta.size,
            "last_modified": meta.last_modified,
            "content_type": meta.content_type,
            "metadata": metadata,
        })]),
        Err(_) => Ok(Vec::new()),
    }
}

async fn actual_object_entries(
    state: &AppState,
    requested_path: &str,
    prefix: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let files = state
        .storage
        .list(prefix)
        .await
        .map_err(|error| error.to_string())?;
    Ok(files
        .into_iter()
        .map(|file| {
            let relative_name = file
                .path
                .strip_prefix(prefix)
                .unwrap_or(&file.path)
                .trim_start_matches('/')
                .to_string();
            json!({
                "entry_type": "file",
                "name": if relative_name.is_empty() { file.path.rsplit('/').next().unwrap_or("data").to_string() } else { relative_name.clone() },
                "path": if requested_path.is_empty() {
                    relative_name
                } else if relative_name.is_empty() {
                    requested_path.to_string()
                } else {
                    format!("{requested_path}/{relative_name}")
                },
                "storage_path": file.path,
                "size_bytes": file.size,
                "last_modified": file.last_modified,
                "content_type": file.content_type,
                "metadata": json!({}),
            })
        })
        .collect())
}

fn view_prefix_path(view: &DatasetView, dataset_root: &str) -> Option<String> {
    view.storage_path.as_ref().map(|storage_path| {
        storage_path
            .rsplit_once("/v")
            .map(|(prefix, _)| prefix.to_string())
            .unwrap_or_else(|| format!("{dataset_root}/views/{}", view.id))
    })
}

fn logical_dir_entry(path: &str, name: &str, metadata: serde_json::Value) -> serde_json::Value {
    json!({
        "entry_type": "directory",
        "name": name,
        "path": path,
        "metadata": metadata,
    })
}

fn build_breadcrumbs(requested_path: &str) -> Vec<serde_json::Value> {
    let mut breadcrumbs = vec![json!({
        "name": "root",
        "path": "",
    })];
    let mut current = String::new();
    for part in requested_path.split('/').filter(|part| !part.is_empty()) {
        if !current.is_empty() {
            current.push('/');
        }
        current.push_str(part);
        breadcrumbs.push(json!({
            "name": part,
            "path": current,
        }));
    }
    breadcrumbs
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use crate::models::{
        branch::DatasetBranch, dataset::Dataset, version::DatasetVersion, view::DatasetView,
    };

    use super::{build_breadcrumbs, root_entries, view_prefix_path};

    fn sample_dataset() -> Dataset {
        Dataset {
            id: Uuid::nil(),
            name: "sales".to_string(),
            description: String::new(),
            format: "json".to_string(),
            storage_path: "datasets/0000".to_string(),
            size_bytes: 10,
            row_count: 2,
            owner_id: Uuid::nil(),
            tags: Vec::new(),
            current_version: 2,
            active_branch: "main".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn builds_root_entries_with_views_section() {
        let dataset = sample_dataset();
        let entries = root_entries(
            &dataset,
            &[DatasetVersion {
                id: Uuid::nil(),
                dataset_id: dataset.id,
                version: 1,
                message: String::new(),
                size_bytes: 10,
                row_count: 1,
                storage_path: "datasets/0000/v1".to_string(),
                created_at: chrono::Utc::now(),
                transaction_id: None,
            }],
            &[DatasetBranch {
                id: Uuid::nil(),
                dataset_id: dataset.id,
                name: "main".to_string(),
                version: 2,
                base_version: 2,
                description: String::new(),
                is_default: true,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
            &[DatasetView {
                id: Uuid::nil(),
                dataset_id: dataset.id,
                name: "top_customers".to_string(),
                description: String::new(),
                sql_text: "SELECT 1".to_string(),
                source_branch: None,
                source_version: None,
                materialized: true,
                refresh_on_source_update: false,
                format: "json".to_string(),
                current_version: 1,
                storage_path: Some("datasets/0000/views/0000/v1.json".to_string()),
                row_count: 1,
                schema_fields: json!([]),
                last_refreshed_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            }],
        );
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[3]["path"], json!("views"));
    }

    #[test]
    fn derives_view_prefix_from_materialized_path() {
        let prefix = view_prefix_path(
            &DatasetView {
                id: Uuid::nil(),
                dataset_id: Uuid::nil(),
                name: "top_customers".to_string(),
                description: String::new(),
                sql_text: "SELECT 1".to_string(),
                source_branch: None,
                source_version: None,
                materialized: true,
                refresh_on_source_update: false,
                format: "json".to_string(),
                current_version: 2,
                storage_path: Some("datasets/0000/views/abcd/v2.json".to_string()),
                row_count: 10,
                schema_fields: json!([]),
                last_refreshed_at: None,
                created_at: chrono::Utc::now(),
                updated_at: chrono::Utc::now(),
            },
            "datasets/0000",
        );
        assert_eq!(prefix, Some("datasets/0000/views/abcd".to_string()));
    }

    #[test]
    fn builds_breadcrumb_chain() {
        let breadcrumbs = build_breadcrumbs("views/top_customers");
        assert_eq!(breadcrumbs.len(), 3);
        assert_eq!(breadcrumbs[2]["path"], json!("views/top_customers"));
    }
}
