use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::models::{
    decode_json,
    feature::{GeometryType, MapFeature},
    style::LayerStyle,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LayerSourceKind {
    Dataset,
    VectorTile,
    Reference,
}

impl LayerSourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dataset => "dataset",
            Self::VectorTile => "vector_tile",
            Self::Reference => "reference",
        }
    }
}

impl FromStr for LayerSourceKind {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "dataset" => Ok(Self::Dataset),
            "vector_tile" => Ok(Self::VectorTile),
            "reference" => Ok(Self::Reference),
            _ => Err(format!("unsupported layer source kind: {value}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerDefinition {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub source_kind: LayerSourceKind,
    pub source_dataset: String,
    pub geometry_type: GeometryType,
    pub style: LayerStyle,
    pub features: Vec<MapFeature>,
    pub tags: Vec<String>,
    pub indexed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateLayerRequest {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub source_kind: LayerSourceKind,
    pub source_dataset: String,
    pub geometry_type: GeometryType,
    #[serde(default)]
    pub style: LayerStyle,
    pub features: Vec<MapFeature>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_indexed")]
    pub indexed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateLayerRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source_kind: Option<LayerSourceKind>,
    pub source_dataset: Option<String>,
    pub geometry_type: Option<GeometryType>,
    pub style: Option<LayerStyle>,
    pub features: Option<Vec<MapFeature>>,
    pub tags: Option<Vec<String>>,
    pub indexed: Option<bool>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LayerRow {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub source_kind: String,
    pub source_dataset: String,
    pub geometry_type: String,
    pub style: Value,
    pub features: Value,
    pub tags: Value,
    pub indexed: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<LayerRow> for LayerDefinition {
    type Error = String;

    fn try_from(row: LayerRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            name: row.name,
            description: row.description,
            source_kind: LayerSourceKind::from_str(&row.source_kind)?,
            source_dataset: row.source_dataset,
            geometry_type: GeometryType::from_str(&row.geometry_type)?,
            style: decode_json(row.style, "style")?,
            features: decode_json(row.features, "features")?,
            tags: decode_json(row.tags, "tags")?,
            indexed: row.indexed,
            created_at: row.created_at,
            updated_at: row.updated_at,
        })
    }
}

fn default_indexed() -> bool {
    true
}
