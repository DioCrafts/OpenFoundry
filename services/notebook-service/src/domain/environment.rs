use std::path::{Component, Path, PathBuf};

use chrono::{DateTime, Utc};
use tokio::fs;
use uuid::Uuid;

use crate::models::workspace::NotebookWorkspaceFile;

pub fn notebook_workspace_root(data_dir: &str, notebook_id: Uuid) -> PathBuf {
    Path::new(data_dir)
        .join("workspaces")
        .join(notebook_id.to_string())
}

pub async fn ensure_workspace_seed(data_dir: &str, notebook_id: Uuid) -> Result<(), String> {
    let root = notebook_workspace_root(data_dir, notebook_id);
    fs::create_dir_all(&root)
        .await
        .map_err(|error| format!("failed to create notebook workspace: {error}"))?;

    let readme = root.join("README.md");
    if fs::metadata(&readme).await.is_err() {
        let seed = format!(
            "# Notebook Workspace\n\nNotebook `{}` now has a persisted workspace.\n\nUse this area for helper scripts, prompts, notes, and analysis artifacts that live next to your notebook cells.\n",
            notebook_id
        );
        fs::write(&readme, seed)
            .await
            .map_err(|error| format!("failed to seed notebook workspace: {error}"))?;
    }

    Ok(())
}

pub async fn list_workspace_files(
    data_dir: &str,
    notebook_id: Uuid,
) -> Result<Vec<NotebookWorkspaceFile>, String> {
    ensure_workspace_seed(data_dir, notebook_id).await?;
    let root = notebook_workspace_root(data_dir, notebook_id);
    let mut files = Vec::new();
    collect_workspace_files(&root, &root, &mut files).await?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
}

pub async fn upsert_workspace_file(
    data_dir: &str,
    notebook_id: Uuid,
    path: &str,
    content: &str,
) -> Result<NotebookWorkspaceFile, String> {
    ensure_workspace_seed(data_dir, notebook_id).await?;
    let normalized = normalize_workspace_path(path)?;
    let root = notebook_workspace_root(data_dir, notebook_id);
    let absolute_path = root.join(&normalized);

    if let Some(parent) = absolute_path.parent() {
        fs::create_dir_all(parent)
            .await
            .map_err(|error| format!("failed to create workspace directories: {error}"))?;
    }

    fs::write(&absolute_path, content)
        .await
        .map_err(|error| format!("failed to write workspace file: {error}"))?;

    load_workspace_file(&root, &absolute_path).await
}

pub async fn delete_workspace_file(
    data_dir: &str,
    notebook_id: Uuid,
    path: &str,
) -> Result<bool, String> {
    ensure_workspace_seed(data_dir, notebook_id).await?;
    let normalized = normalize_workspace_path(path)?;
    let root = notebook_workspace_root(data_dir, notebook_id);
    let absolute_path = root.join(normalized);

    match fs::metadata(&absolute_path).await {
        Ok(metadata) if metadata.is_file() => {
            fs::remove_file(&absolute_path)
                .await
                .map_err(|error| format!("failed to delete workspace file: {error}"))?;
            Ok(true)
        }
        Ok(_) => Err("workspace path is not a file".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!("failed to inspect workspace file: {error}")),
    }
}

fn normalize_workspace_path(path: &str) -> Result<PathBuf, String> {
    let candidate = path.trim().replace('\\', "/");
    if candidate.is_empty() {
        return Err("workspace path is required".to_string());
    }

    let raw = Path::new(&candidate);
    if raw.is_absolute() {
        return Err("workspace paths must be relative".to_string());
    }

    let mut normalized = PathBuf::new();
    for component in raw.components() {
        match component {
            Component::Normal(part) => normalized.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err("workspace paths cannot escape the notebook root".to_string());
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err("workspace paths must stay inside the notebook root".to_string());
            }
        }
    }

    if normalized.as_os_str().is_empty() {
        return Err("workspace path is required".to_string());
    }

    Ok(normalized)
}

async fn collect_workspace_files(
    root: &Path,
    current: &Path,
    files: &mut Vec<NotebookWorkspaceFile>,
) -> Result<(), String> {
    let mut entries = fs::read_dir(current)
        .await
        .map_err(|error| format!("failed to read workspace directory: {error}"))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| format!("failed to iterate workspace directory: {error}"))?
    {
        let path = entry.path();
        let metadata = entry
            .metadata()
            .await
            .map_err(|error| format!("failed to stat workspace entry: {error}"))?;

        if metadata.is_dir() {
            Box::pin(collect_workspace_files(root, &path, files)).await?;
            continue;
        }

        files.push(load_workspace_file(root, &path).await?);
    }

    Ok(())
}

async fn load_workspace_file(
    root: &Path,
    absolute_path: &Path,
) -> Result<NotebookWorkspaceFile, String> {
    let content = fs::read_to_string(absolute_path)
        .await
        .map_err(|error| format!("failed to read workspace file: {error}"))?;
    let metadata = fs::metadata(absolute_path)
        .await
        .map_err(|error| format!("failed to stat workspace file: {error}"))?;
    let relative = absolute_path
        .strip_prefix(root)
        .map_err(|error| format!("failed to compute workspace relative path: {error}"))?;
    let modified = metadata
        .modified()
        .ok()
        .map(DateTime::<Utc>::from)
        .unwrap_or_else(Utc::now);

    Ok(NotebookWorkspaceFile {
        path: relative.to_string_lossy().replace('\\', "/"),
        language: infer_workspace_language(relative),
        content,
        size_bytes: metadata.len() as usize,
        updated_at: modified,
    })
}

fn infer_workspace_language(path: &Path) -> String {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|value| value.to_ascii_lowercase())
    {
        Some(extension) if extension == "py" => "python".to_string(),
        Some(extension) if extension == "sql" => "sql".to_string(),
        Some(extension) if extension == "r" => "r".to_string(),
        Some(extension) if extension == "md" => "markdown".to_string(),
        Some(extension) if extension == "json" => "json".to_string(),
        Some(extension) if extension == "ts" || extension == "tsx" => "typescript".to_string(),
        Some(extension) if extension == "js" || extension == "jsx" => "javascript".to_string(),
        Some(extension) if extension == "toml" => "toml".to_string(),
        _ => "text".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_directory_escapes() {
        let error = normalize_workspace_path("../secret.txt").expect_err("path should fail");
        assert!(error.contains("escape"));
    }

    #[test]
    fn normalizes_relative_workspace_paths() {
        let path = normalize_workspace_path("src/./analysis.py").expect("path should normalize");
        assert_eq!(path.to_string_lossy(), "src/analysis.py");
    }
}
