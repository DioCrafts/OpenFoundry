use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{types::Json, FromRow};
use uuid::Uuid;

use crate::models::{page::AppPage, theme::AppTheme, version::AppSnapshot, widget_type::WidgetCatalogItem};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppSettings {
	#[serde(default)]
	pub home_page_id: Option<String>,
	#[serde(default = "default_navigation_style")]
	pub navigation_style: String,
	#[serde(default = "default_max_width")]
	pub max_width: String,
	#[serde(default = "default_show_branding")]
	pub show_branding: bool,
	#[serde(default)]
	pub custom_css: Option<String>,
}

impl Default for AppSettings {
	fn default() -> Self {
		Self {
			home_page_id: None,
			navigation_style: default_navigation_style(),
			max_width: default_max_width(),
			show_branding: default_show_branding(),
			custom_css: None,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct App {
	pub id: Uuid,
	pub name: String,
	pub slug: String,
	pub description: String,
	pub status: String,
	pub pages: Vec<AppPage>,
	pub theme: AppTheme,
	pub settings: AppSettings,
	pub template_key: Option<String>,
	pub created_by: Option<Uuid>,
	pub published_version_id: Option<Uuid>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl App {
	pub fn page_count(&self) -> usize {
		self.pages.len()
	}

	pub fn widget_count(&self) -> usize {
		self.pages.iter().map(AppPage::widget_count).sum()
	}

	pub fn snapshot(&self) -> AppSnapshot {
		AppSnapshot {
			name: self.name.clone(),
			slug: self.slug.clone(),
			description: self.description.clone(),
			status: self.status.clone(),
			pages: self.pages.clone(),
			theme: self.theme.clone(),
			settings: self.settings.clone(),
			template_key: self.template_key.clone(),
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSummary {
	pub id: Uuid,
	pub name: String,
	pub slug: String,
	pub description: String,
	pub status: String,
	pub page_count: usize,
	pub widget_count: usize,
	pub template_key: Option<String>,
	pub published_version_id: Option<Uuid>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

impl From<&App> for AppSummary {
	fn from(value: &App) -> Self {
		Self {
			id: value.id,
			name: value.name.clone(),
			slug: value.slug.clone(),
			description: value.description.clone(),
			status: value.status.clone(),
			page_count: value.page_count(),
			widget_count: value.widget_count(),
			template_key: value.template_key.clone(),
			published_version_id: value.published_version_id,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAppsResponse {
	pub data: Vec<AppSummary>,
	pub total: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListAppsQuery {
	#[serde(default = "default_page")]
	pub page: i64,
	#[serde(default = "default_per_page")]
	pub per_page: i64,
	#[serde(default)]
	pub search: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAppRequest {
	pub name: String,
	#[serde(default)]
	pub slug: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default)]
	pub pages: Option<Vec<AppPage>>,
	#[serde(default)]
	pub theme: Option<AppTheme>,
	#[serde(default)]
	pub settings: Option<AppSettings>,
	#[serde(default)]
	pub template_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAppRequest {
	#[serde(default)]
	pub name: Option<String>,
	#[serde(default)]
	pub slug: Option<String>,
	#[serde(default)]
	pub description: Option<String>,
	#[serde(default)]
	pub status: Option<String>,
	#[serde(default)]
	pub pages: Option<Vec<AppPage>>,
	#[serde(default)]
	pub theme: Option<AppTheme>,
	#[serde(default)]
	pub settings: Option<AppSettings>,
	#[serde(default)]
	pub template_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTemplateDefinition {
	#[serde(default)]
	pub pages: Vec<AppPage>,
	#[serde(default)]
	pub theme: AppTheme,
	#[serde(default)]
	pub settings: AppSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTemplate {
	pub id: Uuid,
	pub key: String,
	pub name: String,
	pub description: String,
	pub category: String,
	pub preview_image_url: Option<String>,
	pub definition: AppTemplateDefinition,
	pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAppTemplatesResponse {
	pub data: Vec<AppTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEmbedInfo {
	pub url: String,
	pub iframe_html: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPreviewResponse {
	pub app: App,
	pub widget_catalog: Vec<WidgetCatalogItem>,
	pub embed: AppEmbedInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishedAppResponse {
	pub app: App,
	pub embed: AppEmbedInfo,
	pub published_version_number: i32,
	pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct AppRow {
	pub id: Uuid,
	pub name: String,
	pub slug: String,
	pub description: String,
	pub status: String,
	pub pages: Json<Vec<AppPage>>,
	pub theme: Json<AppTheme>,
	pub settings: Json<AppSettings>,
	pub template_key: Option<String>,
	pub created_by: Option<Uuid>,
	pub published_version_id: Option<Uuid>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub(crate) struct AppTemplateRow {
	pub id: Uuid,
	pub key: String,
	pub name: String,
	pub description: String,
	pub category: String,
	pub preview_image_url: Option<String>,
	pub definition: Json<AppTemplateDefinition>,
	pub created_at: DateTime<Utc>,
}

impl From<AppRow> for App {
	fn from(value: AppRow) -> Self {
		Self {
			id: value.id,
			name: value.name,
			slug: value.slug,
			description: value.description,
			status: value.status,
			pages: value.pages.0,
			theme: value.theme.0,
			settings: value.settings.0,
			template_key: value.template_key,
			created_by: value.created_by,
			published_version_id: value.published_version_id,
			created_at: value.created_at,
			updated_at: value.updated_at,
		}
	}
}

impl From<AppTemplateRow> for AppTemplate {
	fn from(value: AppTemplateRow) -> Self {
		Self {
			id: value.id,
			key: value.key,
			name: value.name,
			description: value.description,
			category: value.category,
			preview_image_url: value.preview_image_url,
			definition: value.definition.0,
			created_at: value.created_at,
		}
	}
}

fn default_page() -> i64 {
	1
}

fn default_per_page() -> i64 {
	20
}

fn default_navigation_style() -> String {
	"tabs".to_string()
}

fn default_max_width() -> String {
	"1280px".to_string()
}

fn default_show_branding() -> bool {
	true
}
