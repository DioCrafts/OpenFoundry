use crate::models::{comment::ReviewComment, merge_request::MergeRequestDefinition};

pub fn approval_summary(
    merge_request: &MergeRequestDefinition,
    comments: &[ReviewComment],
) -> (usize, usize) {
    let approvals = merge_request
        .reviewers
        .iter()
        .filter(|reviewer| reviewer.approved)
        .count();
    let threads = comments
        .iter()
        .filter(|comment| comment.line_number.is_some())
        .count();
    (approvals, threads)
}
