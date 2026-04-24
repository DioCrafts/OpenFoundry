use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::report::GeneratorKind;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleCadence {
    Manual,
    Cron,
    Daily,
    Weekly,
    Monthly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportSchedule {
    pub cadence: ScheduleCadence,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default = "default_timezone")]
    pub timezone: String,
    #[serde(default = "default_anchor_time")]
    pub anchor_time: String,
    #[serde(default)]
    pub interval_minutes: Option<i64>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<DateTime<Utc>>,
}

impl Default for ReportSchedule {
    fn default() -> Self {
        Self {
            cadence: ScheduleCadence::Manual,
            expression: None,
            timezone: default_timezone(),
            anchor_time: default_anchor_time(),
            interval_minutes: None,
            enabled: false,
            next_run_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledRun {
    pub report_id: Uuid,
    pub report_name: String,
    pub generator_kind: GeneratorKind,
    pub next_run_at: DateTime<Utc>,
    pub recipient_count: usize,
    pub cadence: ScheduleCadence,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

fn default_anchor_time() -> String {
    "09:00".to_string()
}

fn default_enabled() -> bool {
    true
}
