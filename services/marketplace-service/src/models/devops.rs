use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::decode_json;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackagedResource {
    pub kind: String,
    pub name: String,
    pub resource_ref: String,
    #[serde(default)]
    pub source_branch: Option<String>,
    #[serde(default = "default_required")]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaintenanceWindow {
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_days")]
    pub days: Vec<String>,
    #[serde(default = "default_start_hour_utc")]
    pub start_hour_utc: u8,
    #[serde(default = "default_duration_minutes")]
    pub duration_minutes: i32,
}

impl Default for MaintenanceWindow {
    fn default() -> Self {
        Self {
            timezone: default_timezone(),
            days: default_days(),
            start_hour_utc: default_start_hour_utc(),
            duration_minutes: default_duration_minutes(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeploymentCell {
    pub name: String,
    pub cloud: String,
    pub region: String,
    #[serde(default)]
    pub workspace_targets: Vec<String>,
    #[serde(default = "default_traffic_weight")]
    pub traffic_weight: u8,
    #[serde(default = "default_cell_status")]
    pub status: String,
    #[serde(default)]
    pub sovereign_boundary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResidencyPolicy {
    #[serde(default = "default_residency_mode")]
    pub mode: String,
    #[serde(default)]
    pub allowed_regions: Vec<String>,
    #[serde(default)]
    pub failover_regions: Vec<String>,
    #[serde(default)]
    pub require_same_sovereign_boundary: bool,
}

impl Default for ResidencyPolicy {
    fn default() -> Self {
        Self {
            mode: default_residency_mode(),
            allowed_regions: Vec::new(),
            failover_regions: Vec::new(),
            require_same_sovereign_boundary: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PromotionGateSummary {
    pub total: usize,
    pub passed: usize,
    pub blocking: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductFleetRecord {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub name: String,
    pub environment: String,
    pub workspace_targets: Vec<String>,
    pub release_channel: String,
    pub auto_upgrade_enabled: bool,
    pub maintenance_window: MaintenanceWindow,
    pub branch_strategy: String,
    pub rollout_strategy: String,
    pub deployment_cells: Vec<DeploymentCell>,
    pub residency_policy: ResidencyPolicy,
    pub promotion_gate_summary: PromotionGateSummary,
    pub status: String,
    pub install_count: usize,
    pub current_version: Option<String>,
    pub target_version: Option<String>,
    pub pending_upgrade_count: usize,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductFleetRequest {
    pub listing_id: Uuid,
    pub name: String,
    #[serde(default = "default_environment")]
    pub environment: String,
    #[serde(default)]
    pub workspace_targets: Vec<String>,
    #[serde(default = "default_release_channel")]
    pub release_channel: String,
    #[serde(default)]
    pub auto_upgrade_enabled: bool,
    #[serde(default)]
    pub maintenance_window: MaintenanceWindow,
    #[serde(default = "default_branch_strategy")]
    pub branch_strategy: String,
    #[serde(default = "default_rollout_strategy")]
    pub rollout_strategy: String,
    #[serde(default)]
    pub deployment_cells: Vec<DeploymentCell>,
    #[serde(default)]
    pub residency_policy: ResidencyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFleetRequest {
    #[serde(default)]
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetSyncResult {
    pub fleet: ProductFleetRecord,
    pub target_version: Option<String>,
    pub upgraded_workspaces: Vec<String>,
    pub skipped_workspaces: Vec<String>,
    pub blocked_workspaces: Vec<String>,
    pub workspace_cell_assignments: std::collections::HashMap<String, String>,
    pub blocking_gates: Vec<String>,
    pub blocked_reason: Option<String>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromotionGateRecord {
    pub id: Uuid,
    pub fleet_id: Uuid,
    pub fleet_name: String,
    pub name: String,
    pub gate_kind: String,
    pub required: bool,
    pub status: String,
    pub evidence: Value,
    pub notes: String,
    pub last_evaluated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePromotionGateRequest {
    pub name: String,
    pub gate_kind: String,
    #[serde(default = "default_required")]
    pub required: bool,
    pub status: Option<String>,
    #[serde(default)]
    pub evidence: Value,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePromotionGateRequest {
    pub required: Option<bool>,
    pub status: Option<String>,
    pub evidence: Option<Value>,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentBranchRecord {
    pub id: Uuid,
    pub fleet_id: Uuid,
    pub fleet_name: String,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub name: String,
    pub repository_branch: String,
    pub source_release_channel: String,
    pub source_version: Option<String>,
    pub workspace_targets: Vec<String>,
    pub status: String,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnrollmentBranchRequest {
    pub fleet_id: Uuid,
    pub name: String,
    #[serde(default)]
    pub repository_branch: Option<String>,
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ProductFleetRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub name: String,
    pub environment: String,
    pub workspace_targets: Value,
    pub release_channel: String,
    pub auto_upgrade_enabled: bool,
    pub maintenance_window: Value,
    pub branch_strategy: String,
    pub rollout_strategy: String,
    pub deployment_cells: Value,
    pub residency_policy: Value,
    pub status: String,
    pub last_synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EnrollmentBranchRow {
    pub id: Uuid,
    pub fleet_id: Uuid,
    pub fleet_name: String,
    pub listing_id: Uuid,
    pub listing_name: String,
    pub name: String,
    pub repository_branch: String,
    pub source_release_channel: String,
    pub source_version: Option<String>,
    pub workspace_targets: Value,
    pub status: String,
    pub notes: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PromotionGateRow {
    pub id: Uuid,
    pub fleet_id: Uuid,
    pub fleet_name: String,
    pub name: String,
    pub gate_kind: String,
    pub required: bool,
    pub status: String,
    pub evidence: Value,
    pub notes: String,
    pub last_evaluated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProductFleetRecord {
    pub fn from_row(
        row: ProductFleetRow,
        install_count: usize,
        current_version: Option<String>,
        target_version: Option<String>,
        pending_upgrade_count: usize,
        promotion_gate_summary: PromotionGateSummary,
    ) -> Result<Self, String> {
        let maintenance_window = if row.maintenance_window == serde_json::json!({})
            || row.maintenance_window.is_null()
        {
            MaintenanceWindow::default()
        } else {
            decode_json(row.maintenance_window, "maintenance_window")?
        };
        let deployment_cells =
            if row.deployment_cells == serde_json::json!([]) || row.deployment_cells.is_null() {
                Vec::new()
            } else {
                decode_json(row.deployment_cells, "deployment_cells")?
            };
        let residency_policy =
            if row.residency_policy == serde_json::json!({}) || row.residency_policy.is_null() {
                ResidencyPolicy::default()
            } else {
                decode_json(row.residency_policy, "residency_policy")?
            };

        Ok(Self {
            id: row.id,
            listing_id: row.listing_id,
            listing_name: row.listing_name,
            name: row.name,
            environment: row.environment,
            workspace_targets: decode_json(row.workspace_targets, "workspace_targets")?,
            release_channel: row.release_channel,
            auto_upgrade_enabled: row.auto_upgrade_enabled,
            maintenance_window,
            branch_strategy: row.branch_strategy,
            rollout_strategy: row.rollout_strategy,
            deployment_cells,
            residency_policy,
            promotion_gate_summary,
            status: row.status,
            install_count,
            current_version,
            target_version,
            pending_upgrade_count,
            last_synced_at: row.last_synced_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<EnrollmentBranchRow> for EnrollmentBranchRecord {
    type Error = String;

    fn try_from(row: EnrollmentBranchRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            fleet_id: row.fleet_id,
            fleet_name: row.fleet_name,
            listing_id: row.listing_id,
            listing_name: row.listing_name,
            name: row.name,
            repository_branch: row.repository_branch,
            source_release_channel: row.source_release_channel,
            source_version: row.source_version,
            workspace_targets: decode_json(row.workspace_targets, "workspace_targets")?,
            status: row.status,
            notes: row.notes,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl TryFrom<PromotionGateRow> for PromotionGateRecord {
    type Error = String;

    fn try_from(row: PromotionGateRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            fleet_id: row.fleet_id,
            fleet_name: row.fleet_name,
            name: row.name,
            gate_kind: row.gate_kind,
            required: row.required,
            status: row.status,
            evidence: row.evidence,
            notes: row.notes,
            last_evaluated_at: row.last_evaluated_at,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn default_release_channel() -> String {
    "stable".to_string()
}

fn default_environment() -> String {
    "production".to_string()
}

fn default_branch_strategy() -> String {
    "isolated_branch_per_feature".to_string()
}

fn default_rollout_strategy() -> String {
    "rolling".to_string()
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_days() -> Vec<String> {
    vec!["sun".to_string()]
}

fn default_start_hour_utc() -> u8 {
    2
}

fn default_duration_minutes() -> i32 {
    120
}

fn default_required() -> bool {
    true
}

fn default_traffic_weight() -> u8 {
    100
}

fn default_cell_status() -> String {
    "ready".to_string()
}

fn default_residency_mode() -> String {
    "preferred_cell".to_string()
}
