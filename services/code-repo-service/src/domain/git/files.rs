use crate::models::file::{RepositoryFile, SearchResult};

pub fn default_repository_files(repository_id: uuid::Uuid, default_branch: &str) -> Vec<RepositoryFile> {
	vec![
		RepositoryFile {
			id: uuid::Uuid::now_v7(),
			repository_id,
			path: "README.md".to_string(),
			branch_name: default_branch.to_string(),
			language: "markdown".to_string(),
			size_bytes: 1820,
			content: "# OpenFoundry Package\n\nThis repository tracks a deployable package with CI metadata.".to_string(),
			last_commit_sha: "init000".to_string(),
		},
		RepositoryFile {
			id: uuid::Uuid::now_v7(),
			repository_id,
			path: "src/lib.rs".to_string(),
			branch_name: default_branch.to_string(),
			language: "rust".to_string(),
			size_bytes: 960,
			content: "pub fn package_entry() -> &'static str {\n    \"openfoundry\"\n}\n".to_string(),
			last_commit_sha: "init000".to_string(),
		},
		RepositoryFile {
			id: uuid::Uuid::now_v7(),
			repository_id,
			path: "openfoundry.toml".to_string(),
			branch_name: default_branch.to_string(),
			language: "toml".to_string(),
			size_bytes: 320,
			content: "[package]\nname = \"sample-package\"\nkind = \"widget\"\n".to_string(),
			last_commit_sha: "init000".to_string(),
		},
	]
}

pub fn file_search_results(files: &[RepositoryFile], query: &str) -> Vec<SearchResult> {
	let normalized = query.to_lowercase();
	files
		.iter()
		.filter_map(|file| {
			let haystack = format!("{}\n{}", file.path.to_lowercase(), file.content.to_lowercase());
			if !haystack.contains(&normalized) {
				return None;
			}
			let snippet = file
				.content
				.lines()
				.find(|line| line.to_lowercase().contains(&normalized))
				.unwrap_or(file.content.as_str())
				.to_string();
			Some(SearchResult {
				path: file.path.clone(),
				branch_name: file.branch_name.clone(),
				snippet,
				score: 0.72 + ((normalized.len() % 10) as f64 / 100.0),
			})
		})
		.collect()
}