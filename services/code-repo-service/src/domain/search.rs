use crate::models::file::{RepositoryFile, SearchResult};

pub fn search(files: &[RepositoryFile], query: &str) -> Vec<SearchResult> {
    crate::domain::git::file_search_results(files, query)
}
