use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistributionChannel {
    Email,
    S3,
    Slack,
    Teams,
    Webhook,
}

impl DistributionChannel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Email => "email",
            Self::S3 => "s3",
            Self::Slack => "slack",
            Self::Teams => "teams",
            Self::Webhook => "webhook",
        }
    }
}

impl Display for DistributionChannel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for DistributionChannel {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "email" => Ok(Self::Email),
            "s3" => Ok(Self::S3),
            "slack" => Ok(Self::Slack),
            "teams" => Ok(Self::Teams),
            "webhook" => Ok(Self::Webhook),
            _ => Err(format!("unsupported distribution channel: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionRecipient {
    pub id: String,
    pub channel: DistributionChannel,
    pub target: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionChannelCatalogEntry {
    pub channel: DistributionChannel,
    pub display_name: String,
    pub description: String,
    pub configuration_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    pub channel: DistributionChannel,
    pub target: String,
    pub status: String,
    pub delivered_at: DateTime<Utc>,
    pub detail: String,
}
