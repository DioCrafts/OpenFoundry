use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{decode_json, merge_request::MergeRequestDefinition};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryVisibility {
    Public,
    Private,
}

impl RepositoryVisibility {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Private => "private",
        }
    }
}

impl FromStr for RepositoryVisibility {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "public" => Ok(Self::Public),
            "private" => Ok(Self::Private),
            _ => Err(format!("unsupported repository visibility: {value}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackageKind {
    Connector,
    Transform,
    Widget,
    AppTemplate,
    MlModel,
    AiAgent,
}

impl PackageKind {
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

    pub fn label(self) -> &'static str {
        match self {
            Self::Connector => "Connector",
            Self::Transform => "Transform",
            Self::Widget => "Widget",
            Self::AppTemplate => "App Template",
            Self::MlModel => "ML Model",
            Self::AiAgent => "AI Agent",
        }
    }
}

impl Display for PackageKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PackageKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "connector" => Ok(Self::Connector),
            "transform" => Ok(Self::Transform),
            "widget" => Ok(Self::Widget),
            "app_template" => Ok(Self::AppTemplate),
            "ml_model" => Ok(Self::MlModel),
            "ai_agent" => Ok(Self::AiAgent),
            _ => Err(format!("unsupported package kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryDefinition {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub owner: String,
    pub default_branch: String,
    pub visibility: RepositoryVisibility,
    pub object_store_backend: String,
    pub package_kind: PackageKind,
    pub tags: Vec<String>,
    pub settings: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRepositoryRequest {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub description: String,
    pub owner: String,
    #[serde(default = "default_branch")]
    pub default_branch: String,
    pub visibility: RepositoryVisibility,
    pub object_store_backend: String,
    pub package_kind: PackageKind,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub settings: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRepositoryRequest {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub default_branch: Option<String>,
    pub visibility: Option<RepositoryVisibility>,
    pub object_store_backend: Option<String>,
    pub package_kind: Option<PackageKind>,
    pub tags: Option<Vec<String>>,
    pub settings: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryOverview {
    pub repository_count: usize,
    pub private_repository_count: usize,
    pub package_kind_mix: Vec<String>,
    pub open_merge_request_count: usize,
    pub latest_merge_request: Option<MergeRequestDefinition>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RepositoryRow {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: String,
    pub owner: String,
    pub default_branch: String,
    pub visibility: String,
    pub object_store_backend: String,
    pub package_kind: String,
    pub tags: Value,
    pub settings: Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<RepositoryRow> for RepositoryDefinition {
    type Error = String;

    fn try_from(row: RepositoryRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            slug: row.slug,
            description: row.description,
            owner: row.owner,
            default_branch: row.default_branch,
            visibility: RepositoryVisibility::from_str(&row.visibility)?,
            object_store_backend: row.object_store_backend,
            package_kind: PackageKind::from_str(&row.package_kind)?,
            tags: decode_json(row.tags, "tags")?,
            settings: row.settings,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

impl RepositoryDefinition {
    pub fn ci_required(&self) -> bool {
        self.settings
            .get("ci_required")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    }

    pub fn allow_direct_commits_on_protected(&self) -> bool {
        self.settings
            .get("allow_direct_commits_on_protected")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    pub fn required_checks_for_branch(&self, branch_name: &str) -> Vec<String> {
        let mut checks = self
            .settings
            .get("required_checks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();

        if let Some(rules) = self
            .settings
            .get("required_checks_by_branch")
            .and_then(Value::as_object)
        {
            for (pattern, values) in rules {
                if !branch_rule_matches(pattern, branch_name) {
                    continue;
                }
                let branch_checks = values
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                if !branch_checks.is_empty() {
                    checks = branch_checks;
                }
            }
        }

        checks.sort();
        checks.dedup();
        checks
    }
}

fn branch_rule_matches(pattern: &str, branch_name: &str) -> bool {
    if pattern == branch_name {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return branch_name.starts_with(prefix);
    }
    false
}

fn default_branch() -> String {
    "main".to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{PackageKind, RepositoryDefinition, RepositoryVisibility};

    fn repository_with_settings(settings: serde_json::Value) -> RepositoryDefinition {
        RepositoryDefinition {
            id: uuid::Uuid::nil(),
            name: "Repo".to_string(),
            slug: "repo".to_string(),
            description: String::new(),
            owner: "owner".to_string(),
            default_branch: "main".to_string(),
            visibility: RepositoryVisibility::Private,
            object_store_backend: "s3".to_string(),
            package_kind: PackageKind::Transform,
            tags: Vec::new(),
            settings,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn branch_specific_checks_override_global_defaults() {
        let repository = repository_with_settings(json!({
            "required_checks": ["package-validation"],
            "required_checks_by_branch": {
                "main": ["package-validation", "policy"],
                "release/*": ["package-validation", "policy", "security"]
            }
        }));

        assert_eq!(
            repository.required_checks_for_branch("main"),
            vec!["package-validation".to_string(), "policy".to_string()]
        );
        assert_eq!(
            repository.required_checks_for_branch("release/2026.04"),
            vec![
                "package-validation".to_string(),
                "policy".to_string(),
                "security".to_string()
            ]
        );
        assert_eq!(
            repository.required_checks_for_branch("feature/demo"),
            vec!["package-validation".to_string()]
        );
    }
}
