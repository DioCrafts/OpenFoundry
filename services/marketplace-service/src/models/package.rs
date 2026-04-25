use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{decode_json, devops::PackagedResource};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageType {
    Connector,
    Transform,
    Widget,
    AppTemplate,
    MlModel,
    AiAgent,
}

impl PackageType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connector => "connector",
            Self::Transform => "transform",
            Self::Widget => "widget",
            Self::AppTemplate => "app_template",
            Self::MlModel => "ml_model",
            Self::AiAgent => "ai_agent",
        }
    }
}

impl Display for PackageType {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PackageType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "connector" => Ok(Self::Connector),
            "transform" => Ok(Self::Transform),
            "widget" => Ok(Self::Widget),
            "app_template" => Ok(Self::AppTemplate),
            "ml_model" => Ok(Self::MlModel),
            "ai_agent" => Ok(Self::AiAgent),
            _ => Err(format!("unsupported package type: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyRequirement {
    pub package_slug: String,
    pub version_req: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageVersion {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub version: String,
    pub release_channel: String,
    pub changelog: String,
    pub dependency_mode: String,
    pub dependencies: Vec<DependencyRequirement>,
    pub packaged_resources: Vec<PackagedResource>,
    pub manifest: Value,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishVersionRequest {
    pub version: String,
    #[serde(default = "default_release_channel")]
    pub release_channel: String,
    pub changelog: String,
    #[serde(default = "default_dependency_mode")]
    pub dependency_mode: String,
    #[serde(default)]
    pub dependencies: Vec<DependencyRequirement>,
    #[serde(default)]
    pub packaged_resources: Vec<PackagedResource>,
    #[serde(default)]
    pub manifest: Value,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PackageVersionRow {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub version: String,
    pub release_channel: String,
    pub changelog: String,
    pub dependency_mode: String,
    pub dependencies: Value,
    pub packaged_resources: Value,
    pub manifest: Value,
    pub published_at: DateTime<Utc>,
}

impl TryFrom<PackageVersionRow> for PackageVersion {
    type Error = String;

    fn try_from(row: PackageVersionRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            listing_id: row.listing_id,
            version: row.version,
            release_channel: row.release_channel,
            changelog: row.changelog,
            dependency_mode: row.dependency_mode,
            dependencies: decode_json(row.dependencies, "dependencies")?,
            packaged_resources: decode_json(row.packaged_resources, "packaged_resources")?,
            manifest: row.manifest,
            published_at: row.published_at,
        })
    }
}

fn default_dependency_mode() -> String {
    "strict".to_string()
}

fn default_release_channel() -> String {
    "stable".to_string()
}
