use chrono::{DateTime, Utc};
use semver::Version;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use uuid::Uuid;

fn default_true() -> bool {
    true
}

fn default_timeout_seconds() -> u64 {
    15
}

fn default_max_source_bytes() -> usize {
    64 * 1024
}

pub const DEFAULT_FUNCTION_PACKAGE_VERSION: &str = "0.1.0";

pub fn default_function_package_version() -> String {
    DEFAULT_FUNCTION_PACKAGE_VERSION.to_string()
}

pub fn parse_function_package_version(version: &str) -> Result<Version, String> {
    Version::parse(version.trim())
        .map_err(|error| format!("function package version must be valid semver: {error}"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCapabilities {
    #[serde(default = "default_true")]
    pub allow_ontology_read: bool,
    #[serde(default)]
    pub allow_ontology_write: bool,
    #[serde(default)]
    pub allow_ai: bool,
    #[serde(default)]
    pub allow_network: bool,
    #[serde(default = "default_timeout_seconds")]
    pub timeout_seconds: u64,
    #[serde(default = "default_max_source_bytes")]
    pub max_source_bytes: usize,
}

impl Default for FunctionCapabilities {
    fn default() -> Self {
        Self {
            allow_ontology_read: true,
            allow_ontology_write: true,
            allow_ai: true,
            allow_network: true,
            timeout_seconds: default_timeout_seconds(),
            max_source_bytes: default_max_source_bytes(),
        }
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct FunctionPackageRow {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub runtime: String,
    pub source: String,
    pub entrypoint: String,
    pub capabilities: Value,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionPackage {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub runtime: String,
    pub source: String,
    pub entrypoint: String,
    pub capabilities: FunctionCapabilities,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<FunctionPackageRow> for FunctionPackage {
    type Error = serde_json::Error;

    fn try_from(row: FunctionPackageRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            version: row.version,
            display_name: row.display_name,
            description: row.description,
            runtime: row.runtime,
            source: row.source,
            entrypoint: row.entrypoint,
            capabilities: serde_json::from_value(row.capabilities).unwrap_or_default(),
            owner_id: row.owner_id,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionPackageSummary {
    pub id: Uuid,
    pub name: String,
    pub version: String,
    pub display_name: String,
    pub runtime: String,
    pub entrypoint: String,
    pub capabilities: FunctionCapabilities,
}

impl From<&FunctionPackage> for FunctionPackageSummary {
    fn from(value: &FunctionPackage) -> Self {
        Self {
            id: value.id,
            name: value.name.clone(),
            version: value.version.clone(),
            display_name: value.display_name.clone(),
            runtime: value.runtime.clone(),
            entrypoint: value.entrypoint.clone(),
            capabilities: value.capabilities.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateFunctionPackageRequest {
    pub name: String,
    pub version: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub runtime: String,
    pub source: String,
    pub entrypoint: Option<String>,
    pub capabilities: Option<FunctionCapabilities>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateFunctionPackageRequest {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub runtime: Option<String>,
    pub source: Option<String>,
    pub entrypoint: Option<String>,
    pub capabilities: Option<FunctionCapabilities>,
}

#[derive(Debug, Deserialize)]
pub struct ListFunctionPackagesQuery {
    pub runtime: Option<String>,
    pub search: Option<String>,
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ListFunctionPackagesResponse {
    pub data: Vec<FunctionPackage>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Deserialize)]
pub struct ValidateFunctionPackageRequest {
    pub object_type_id: Option<Uuid>,
    pub target_object_id: Option<Uuid>,
    #[serde(default)]
    pub parameters: Value,
    pub justification: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ValidateFunctionPackageResponse {
    pub valid: bool,
    pub package: FunctionPackageSummary,
    pub preview: Value,
    pub errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SimulateFunctionPackageRequest {
    pub object_type_id: Uuid,
    pub target_object_id: Option<Uuid>,
    #[serde(default)]
    pub parameters: Value,
    pub justification: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SimulateFunctionPackageResponse {
    pub package: FunctionPackageSummary,
    pub preview: Value,
    pub result: Value,
}
