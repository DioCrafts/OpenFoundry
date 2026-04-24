use crate::models::branch::BranchDefinition;

pub fn synthetic_signature(author: &str) -> String {
    format!(
        "{}@openfoundry.dev",
        author.to_lowercase().replace(' ', ".")
    )
}

pub fn branch_metrics(branch: &BranchDefinition, commit_count: usize) -> (i32, usize) {
    let ahead = if branch.is_default {
        0
    } else {
        (commit_count as i32 / 2).max(1)
    };
    let pending_reviews = if branch.is_default {
        0
    } else {
        1 + (branch.name.len() % 3)
    };
    (ahead, pending_reviews)
}

pub fn commit_files_changed(message: &str) -> usize {
    2 + (message.len() % 5)
}
