use chrono::{Duration, Utc};

use crate::models::commit::{CiRun, CommitDefinition};

pub fn simulate_ci_run(
    repository_id: uuid::Uuid,
    branch_name: &str,
    commit: Option<&CommitDefinition>,
) -> CiRun {
    let now = Utc::now();
    CiRun {
        id: uuid::Uuid::now_v7(),
        repository_id,
        branch_name: branch_name.to_string(),
        commit_sha: commit
            .map(|entry| entry.sha.clone())
            .unwrap_or_else(|| "manual-trigger".to_string()),
        pipeline_name: "package-validation".to_string(),
        status: "passed".to_string(),
        trigger: "push".to_string(),
        started_at: now,
        completed_at: Some(now + Duration::minutes(4)),
        checks: vec![
            "cargo check".to_string(),
            "package lint".to_string(),
            "widget smoke test".to_string(),
        ],
    }
}
