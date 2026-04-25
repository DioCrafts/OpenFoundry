use chrono::{DateTime, Datelike, Timelike, Utc};

use crate::models::{
    devops::{
        DeploymentCell, MaintenanceWindow, PackagedResource, PromotionGateRecord,
        PromotionGateSummary, ResidencyPolicy,
    },
    package::PackageVersion,
};

pub fn normalize_release_channel(value: &str) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed.is_empty() {
        "stable".to_string()
    } else {
        trimmed
    }
}

pub fn normalize_packaged_resources(resources: &[PackagedResource]) -> Vec<PackagedResource> {
    resources
        .iter()
        .filter_map(|resource| {
            let kind = resource.kind.trim().to_ascii_lowercase();
            let name = resource.name.trim().to_string();
            let resource_ref = resource.resource_ref.trim().to_string();

            if kind.is_empty() || name.is_empty() || resource_ref.is_empty() {
                return None;
            }

            Some(PackagedResource {
                kind,
                name,
                resource_ref,
                source_branch: resource
                    .source_branch
                    .as_ref()
                    .map(|branch| branch.trim().to_string())
                    .filter(|branch| !branch.is_empty()),
                required: resource.required,
            })
        })
        .collect()
}

pub fn normalize_workspace_targets(workspaces: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();

    for workspace in workspaces {
        let candidate = workspace.trim();
        if candidate.is_empty() || normalized.iter().any(|entry| entry == candidate) {
            continue;
        }
        normalized.push(candidate.to_string());
    }

    normalized
}

pub fn latest_version_for_channel(
    versions: &[PackageVersion],
    channel: &str,
) -> Option<PackageVersion> {
    let normalized_channel = normalize_release_channel(channel);
    versions
        .iter()
        .filter(|version| normalize_release_channel(&version.release_channel) == normalized_channel)
        .max_by(|left, right| left.published_at.cmp(&right.published_at))
        .cloned()
        .or_else(|| {
            versions
                .iter()
                .max_by(|left, right| left.published_at.cmp(&right.published_at))
                .cloned()
        })
}

pub fn maintenance_window_is_open(window: &MaintenanceWindow, now: DateTime<Utc>) -> bool {
    let days = if window.days.is_empty() {
        vec!["sun".to_string()]
    } else {
        window
            .days
            .iter()
            .map(|day| normalize_day(day))
            .collect::<Vec<_>>()
    };
    let current_day = match now.weekday() {
        chrono::Weekday::Mon => "mon",
        chrono::Weekday::Tue => "tue",
        chrono::Weekday::Wed => "wed",
        chrono::Weekday::Thu => "thu",
        chrono::Weekday::Fri => "fri",
        chrono::Weekday::Sat => "sat",
        chrono::Weekday::Sun => "sun",
    };

    if !days.iter().any(|day| day == current_day) {
        return false;
    }

    let start_minutes = i32::from(window.start_hour_utc) * 60;
    let current_minutes = (now.hour() as i32) * 60 + (now.minute() as i32);
    let end_minutes = start_minutes + window.duration_minutes.max(0);

    current_minutes >= start_minutes && current_minutes < end_minutes
}

pub fn derive_repository_branch(fleet_name: &str, branch_name: &str) -> String {
    format!("release/{}/{}", slugify(fleet_name), slugify(branch_name))
}

pub fn normalize_deployment_cells(
    cells: &[DeploymentCell],
    workspace_targets: &[String],
    environment: &str,
) -> Vec<DeploymentCell> {
    if !cells.is_empty() {
        let mut normalized = Vec::new();
        for cell in cells {
            let name = cell.name.trim().to_string();
            let cloud = cell.cloud.trim().to_ascii_lowercase();
            let region = cell.region.trim().to_ascii_lowercase();
            if name.is_empty() || cloud.is_empty() || region.is_empty() {
                continue;
            }
            let workspace_targets = normalize_workspace_targets(&cell.workspace_targets);
            if normalized
                .iter()
                .any(|entry: &DeploymentCell| entry.name == name)
            {
                continue;
            }
            normalized.push(DeploymentCell {
                name,
                cloud,
                region,
                workspace_targets,
                traffic_weight: cell.traffic_weight.max(1),
                status: cell.status.trim().to_ascii_lowercase(),
                sovereign_boundary: cell
                    .sovereign_boundary
                    .as_ref()
                    .map(|value| value.trim().to_ascii_lowercase())
                    .filter(|value| !value.is_empty()),
            });
        }
        return normalized;
    }

    let mut grouped = std::collections::BTreeMap::<String, Vec<String>>::new();
    for workspace in workspace_targets {
        let region = infer_workspace_region(workspace).unwrap_or_else(|| "global".to_string());
        grouped.entry(region).or_default().push(workspace.clone());
    }

    grouped
        .into_iter()
        .map(|(region, workspaces)| DeploymentCell {
            name: format!("{}-{}", slugify(environment), slugify(&region)),
            cloud: infer_cloud_from_region(&region).to_string(),
            region: region.clone(),
            workspace_targets: workspaces,
            traffic_weight: 100,
            status: "ready".to_string(),
            sovereign_boundary: infer_boundary_from_region_key(&region),
        })
        .collect()
}

pub fn normalize_residency_policy(
    policy: &ResidencyPolicy,
    cells: &[DeploymentCell],
) -> ResidencyPolicy {
    let mut normalized = policy.clone();
    normalized.mode = if normalized.mode.trim().is_empty() {
        "preferred_cell".to_string()
    } else {
        normalized.mode.trim().to_ascii_lowercase()
    };
    normalized.allowed_regions = normalized
        .allowed_regions
        .iter()
        .map(|region| region.trim().to_ascii_lowercase())
        .filter(|region| !region.is_empty())
        .collect();
    normalized.failover_regions = normalized
        .failover_regions
        .iter()
        .map(|region| region.trim().to_ascii_lowercase())
        .filter(|region| !region.is_empty())
        .collect();

    if normalized.allowed_regions.is_empty() && normalized.mode == "strict" {
        normalized.allowed_regions = cells.iter().map(|cell| cell.region.clone()).collect();
        normalized.allowed_regions.sort();
        normalized.allowed_regions.dedup();
    }

    normalized
}

pub fn summarize_promotion_gates(gates: &[PromotionGateRecord]) -> PromotionGateSummary {
    PromotionGateSummary {
        total: gates.len(),
        passed: gates
            .iter()
            .filter(|gate| matches!(gate.status.as_str(), "passed" | "waived"))
            .count(),
        blocking: promotion_gate_blockers(gates).len(),
    }
}

pub fn promotion_gate_blockers(gates: &[PromotionGateRecord]) -> Vec<String> {
    gates
        .iter()
        .filter(|gate| gate.required && !matches!(gate.status.as_str(), "passed" | "waived"))
        .map(|gate| format!("{} ({})", gate.name, gate.status))
        .collect()
}

pub fn workspace_allowed_by_residency_policy(workspace: &str, policy: &ResidencyPolicy) -> bool {
    if policy.allowed_regions.is_empty() {
        return true;
    }
    infer_workspace_region(workspace)
        .map(|region| {
            policy
                .allowed_regions
                .iter()
                .any(|allowed| allowed == &region)
        })
        .unwrap_or(policy.mode != "strict")
}

pub fn assign_workspace_to_cell(
    workspace: &str,
    cells: &[DeploymentCell],
    policy: &ResidencyPolicy,
) -> Option<DeploymentCell> {
    if cells.is_empty() || !workspace_allowed_by_residency_policy(workspace, policy) {
        return None;
    }

    if let Some(cell) = cells.iter().find(|cell| {
        cell.workspace_targets
            .iter()
            .any(|target| target.eq_ignore_ascii_case(workspace))
            && cell.status == "ready"
    }) {
        return Some(cell.clone());
    }

    let workspace_region = infer_workspace_region(workspace);
    let workspace_boundary = infer_workspace_boundary(workspace);
    cells
        .iter()
        .filter(|cell| cell.status == "ready")
        .filter(|cell| {
            workspace_region
                .as_ref()
                .map(|region| {
                    &cell.region == region || policy.failover_regions.contains(&cell.region)
                })
                .unwrap_or(true)
        })
        .filter(|cell| {
            if !policy.require_same_sovereign_boundary {
                return true;
            }
            workspace_boundary
                .as_ref()
                .zip(cell.sovereign_boundary.as_ref())
                .map(|(left, right)| left == right)
                .unwrap_or(false)
        })
        .max_by_key(|cell| cell.traffic_weight)
        .cloned()
}

pub fn infer_workspace_region(workspace: &str) -> Option<String> {
    let normalized = workspace.trim().to_ascii_lowercase();
    if normalized.contains("eu") {
        Some("eu-west-1".to_string())
    } else if normalized.contains("uk") {
        Some("eu-west-2".to_string())
    } else if normalized.contains("us") || normalized.contains("na") {
        Some("us-east-1".to_string())
    } else if normalized.contains("apac") || normalized.contains("apj") {
        Some("ap-southeast-1".to_string())
    } else if normalized.contains("latam") {
        Some("sa-east-1".to_string())
    } else {
        None
    }
}

fn infer_workspace_boundary(workspace: &str) -> Option<String> {
    let normalized = workspace.trim().to_ascii_lowercase();
    if normalized.contains("eu") || normalized.contains("uk") {
        Some("eu".to_string())
    } else if normalized.contains("us") || normalized.contains("na") || normalized.contains("latam")
    {
        Some("amer".to_string())
    } else if normalized.contains("apac") || normalized.contains("apj") {
        Some("apac".to_string())
    } else {
        None
    }
}

fn normalize_day(value: &str) -> String {
    let trimmed = value.trim().to_ascii_lowercase();
    match trimmed.as_str() {
        "monday" => "mon".to_string(),
        "tuesday" => "tue".to_string(),
        "wednesday" => "wed".to_string(),
        "thursday" => "thu".to_string(),
        "friday" => "fri".to_string(),
        "saturday" => "sat".to_string(),
        "sunday" => "sun".to_string(),
        other => other.chars().take(3).collect(),
    }
}

fn slugify(value: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;

    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }

    slug.trim_matches('-').to_string()
}

fn infer_cloud_from_region(region: &str) -> &'static str {
    if region.starts_with("eu-")
        || region.starts_with("us-")
        || region.starts_with("sa-")
        || region.starts_with("ap-")
    {
        "aws"
    } else {
        "hybrid"
    }
}

fn infer_boundary_from_region_key(region: &str) -> Option<String> {
    if region.starts_with("eu-") {
        Some("eu".to_string())
    } else if region.starts_with("us-") || region.starts_with("sa-") {
        Some("amer".to_string())
    } else if region.starts_with("ap-") {
        Some("apac".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use uuid::Uuid;

    fn version(
        version: &str,
        release_channel: &str,
        published_at: DateTime<Utc>,
    ) -> PackageVersion {
        PackageVersion {
            id: Uuid::now_v7(),
            listing_id: Uuid::now_v7(),
            version: version.to_string(),
            release_channel: release_channel.to_string(),
            changelog: "notes".to_string(),
            dependency_mode: "strict".to_string(),
            dependencies: Vec::new(),
            packaged_resources: Vec::new(),
            manifest: serde_json::json!({}),
            published_at,
        }
    }

    #[test]
    fn prefers_latest_version_in_requested_channel() {
        let versions = vec![
            version(
                "1.0.0",
                "stable",
                Utc.with_ymd_and_hms(2026, 4, 20, 0, 0, 0).unwrap(),
            ),
            version(
                "1.1.0",
                "stable",
                Utc.with_ymd_and_hms(2026, 4, 24, 0, 0, 0).unwrap(),
            ),
            version(
                "1.2.0-beta.1",
                "beta",
                Utc.with_ymd_and_hms(2026, 4, 25, 0, 0, 0).unwrap(),
            ),
        ];

        let selected = latest_version_for_channel(&versions, "stable").expect("version");
        assert_eq!(selected.version, "1.1.0");
    }

    #[test]
    fn maintenance_window_respects_day_and_hour() {
        let window = MaintenanceWindow {
            timezone: "UTC".to_string(),
            days: vec!["sun".to_string()],
            start_hour_utc: 2,
            duration_minutes: 180,
        };

        assert!(maintenance_window_is_open(
            &window,
            Utc.with_ymd_and_hms(2026, 4, 26, 3, 0, 0).unwrap()
        ));
        assert!(!maintenance_window_is_open(
            &window,
            Utc.with_ymd_and_hms(2026, 4, 27, 3, 0, 0).unwrap()
        ));
    }

    #[test]
    fn derives_repository_branch_from_fleet_and_feature_name() {
        assert_eq!(
            derive_repository_branch("Ops Center Fleet", "Feature Shift Handovers"),
            "release/ops-center-fleet/feature-shift-handovers"
        );
    }

    #[test]
    fn promotion_gate_blockers_only_include_required_non_passed_gates() {
        let gates = vec![
            PromotionGateRecord {
                id: Uuid::now_v7(),
                fleet_id: Uuid::now_v7(),
                fleet_name: "Ops".to_string(),
                name: "observability".to_string(),
                gate_kind: "metrics".to_string(),
                required: true,
                status: "passed".to_string(),
                evidence: serde_json::json!({}),
                notes: String::new(),
                last_evaluated_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            PromotionGateRecord {
                id: Uuid::now_v7(),
                fleet_id: Uuid::now_v7(),
                fleet_name: "Ops".to_string(),
                name: "smoke".to_string(),
                gate_kind: "synthetic".to_string(),
                required: true,
                status: "failed".to_string(),
                evidence: serde_json::json!({}),
                notes: String::new(),
                last_evaluated_at: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let blockers = promotion_gate_blockers(&gates);
        assert_eq!(blockers, vec!["smoke (failed)".to_string()]);
    }

    #[test]
    fn assigns_workspace_to_matching_residency_cell() {
        let cells = normalize_deployment_cells(
            &[],
            &[
                "Operations Center - EU".to_string(),
                "Operations Center - US".to_string(),
            ],
            "production",
        );
        let policy = ResidencyPolicy {
            mode: "strict".to_string(),
            allowed_regions: vec!["eu-west-1".to_string()],
            failover_regions: vec![],
            require_same_sovereign_boundary: true,
        };

        let cell = assign_workspace_to_cell("Operations Center - EU", &cells, &policy)
            .expect("EU workspace should resolve to an EU cell");
        assert_eq!(cell.region, "eu-west-1");
        assert!(assign_workspace_to_cell("Operations Center - US", &cells, &policy).is_none());
    }
}
