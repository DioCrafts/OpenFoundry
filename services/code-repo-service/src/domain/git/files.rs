use crate::models::file::{RepositoryFile, SearchResult};

pub fn file_search_results(files: &[RepositoryFile], query: &str) -> Vec<SearchResult> {
    let normalized = query.to_lowercase();
    files
        .iter()
        .filter_map(|file| {
            let haystack = format!(
                "{}\n{}",
                file.path.to_lowercase(),
                file.content.to_lowercase()
            );
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
