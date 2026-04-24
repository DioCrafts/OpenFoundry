use crate::models::{commit::CommitDefinition, file::RepositoryFile};

pub fn repository_diff(
    files: &[RepositoryFile],
    branch_name: &str,
    commits: &[CommitDefinition],
) -> String {
    let head_sha = commits
        .iter()
        .find(|commit| commit.branch_name == branch_name)
        .map(|commit| commit.sha.clone())
        .unwrap_or_else(|| "unknown".to_string());

    files
		.iter()
		.filter(|file| file.branch_name == branch_name)
		.map(|file| format!("diff --git a/{0} b/{0}\nindex {1}..{2} 100644\n--- a/{0}\n+++ b/{0}\n@@\n+ updated from {2}\n", file.path, file.last_commit_sha, head_sha))
		.collect::<Vec<_>>()
		.join("\n")
}
