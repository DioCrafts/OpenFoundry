use chrono::{Duration, Utc};

use crate::models::{
    devops::MaintenanceWindow,
    install::{InstallActivation, InstallRecord},
    listing::ListingDefinition,
    package::{DependencyRequirement, PackageVersion},
};

pub fn latest_version(
    listing: &ListingDefinition,
    versions: &[PackageVersion],
) -> Option<PackageVersion> {
    versions
        .iter()
        .filter(|version| version.listing_id == listing.id)
        .max_by(|left, right| left.published_at.cmp(&right.published_at))
        .cloned()
}

pub fn install_preview(
    install_id: uuid::Uuid,
    listing: &ListingDefinition,
    version: &PackageVersion,
    workspace_name: &str,
    activation: InstallActivation,
    fleet_id: Option<uuid::Uuid>,
    fleet_name: Option<String>,
    maintenance_window: Option<MaintenanceWindow>,
    auto_upgrade_enabled: bool,
    enrollment_branch: Option<String>,
) -> InstallRecord {
    let now = Utc::now();
    InstallRecord {
        id: install_id,
        listing_id: listing.id,
        listing_name: listing.name.clone(),
        version: version.version.clone(),
        release_channel: version.release_channel.clone(),
        workspace_name: workspace_name.to_string(),
        status: "installed".to_string(),
        dependency_plan: version.dependencies.clone(),
        activation,
        fleet_id,
        fleet_name,
        auto_upgrade_enabled,
        maintenance_window,
        enrollment_branch,
        installed_at: now,
        ready_at: Some(now + Duration::minutes(2)),
    }
}

pub fn normalize_dependencies(
    dependencies: &[DependencyRequirement],
) -> Vec<DependencyRequirement> {
    dependencies
        .iter()
        .map(|dependency| DependencyRequirement {
            package_slug: dependency.package_slug.trim().to_string(),
            version_req: dependency.version_req.trim().to_string(),
            required: dependency.required,
        })
        .collect()
}
