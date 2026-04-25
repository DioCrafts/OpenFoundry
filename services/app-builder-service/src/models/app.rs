use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json};
use uuid::Uuid;

use crate::models::{
    page::AppPage, theme::AppTheme, version::AppSnapshot, widget_type::WidgetCatalogItem,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsumerModeSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub allow_guest_access: bool,
    #[serde(default)]
    pub portal_title: Option<String>,
    #[serde(default)]
    pub portal_subtitle: Option<String>,
    #[serde(default)]
    pub primary_cta_label: Option<String>,
    #[serde(default)]
    pub primary_cta_url: Option<String>,
}

impl Default for ConsumerModeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            allow_guest_access: false,
            portal_title: None,
            portal_subtitle: None,
            primary_cta_label: None,
            primary_cta_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlateSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_slate_framework")]
    pub framework: String,
    #[serde(default = "default_slate_package_name")]
    pub package_name: String,
    #[serde(default = "default_slate_entry_file")]
    pub entry_file: String,
    #[serde(default = "default_slate_sdk_import")]
    pub sdk_import: String,
    #[serde(default)]
    pub workspace: SlateWorkspaceSettings,
    #[serde(default)]
    pub quiver_embed: QuiverEmbedSettings,
}

impl Default for SlateSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            framework: default_slate_framework(),
            package_name: default_slate_package_name(),
            entry_file: default_slate_entry_file(),
            sdk_import: default_slate_sdk_import(),
            workspace: SlateWorkspaceSettings::default(),
            quiver_embed: QuiverEmbedSettings::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlateWorkspaceSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default = "default_workspace_layout")]
    pub layout: String,
    #[serde(default = "default_workspace_runtime")]
    pub runtime: String,
    #[serde(default = "default_workspace_dev_command")]
    pub dev_command: String,
    #[serde(default = "default_workspace_preview_command")]
    pub preview_command: String,
    #[serde(default)]
    pub files: Vec<SlatePackageFile>,
}

impl Default for SlateWorkspaceSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            repository_id: None,
            layout: default_workspace_layout(),
            runtime: default_workspace_runtime(),
            dev_command: default_workspace_dev_command(),
            preview_command: default_workspace_preview_command(),
            files: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuiverEmbedSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub primary_type_id: Option<String>,
    #[serde(default)]
    pub secondary_type_id: Option<String>,
    #[serde(default)]
    pub join_field: Option<String>,
    #[serde(default)]
    pub secondary_join_field: Option<String>,
    #[serde(default)]
    pub date_field: Option<String>,
    #[serde(default)]
    pub metric_field: Option<String>,
    #[serde(default)]
    pub group_field: Option<String>,
    #[serde(default)]
    pub selected_group: Option<String>,
}

impl Default for QuiverEmbedSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            primary_type_id: None,
            secondary_type_id: None,
            join_field: None,
            secondary_join_field: None,
            date_field: None,
            metric_field: None,
            group_field: None,
            selected_group: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkshopScenarioPreset {
    #[serde(default)]
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub parameters: BTreeMap<String, String>,
    #[serde(default)]
    pub prompt_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkshopInteractiveSettings {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub subtitle: Option<String>,
    #[serde(default)]
    pub briefing_template: Option<String>,
    #[serde(default)]
    pub primary_scenario_widget_id: Option<String>,
    #[serde(default)]
    pub primary_agent_widget_id: Option<String>,
    #[serde(default)]
    pub suggested_questions: Vec<String>,
    #[serde(default)]
    pub scenario_presets: Vec<WorkshopScenarioPreset>,
}

impl Default for WorkshopInteractiveSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            title: Some("Interactive Workshop".to_string()),
            subtitle: Some(
                "Coordinate scenario presets, decision briefs, and AI copilots from one runtime surface."
                    .to_string(),
            ),
            briefing_template: Some(
                "Current scenario context:\n{{demand_multiplier}} demand multiplier\n{{service_level}} service level\nUse these assumptions to brief the operator."
                    .to_string(),
            ),
            primary_scenario_widget_id: None,
            primary_agent_widget_id: None,
            suggested_questions: vec![
                "What changed versus the baseline scenario?".to_string(),
                "Which mitigations should the team prioritize first?".to_string(),
            ],
            scenario_presets: Vec::new(),
        }
    }
}

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
    #[serde(default = "default_builder_experience")]
    pub builder_experience: String,
    #[serde(default)]
    pub consumer_mode: ConsumerModeSettings,
    #[serde(default)]
    pub interactive_workshop: WorkshopInteractiveSettings,
    #[serde(default)]
    pub slate: SlateSettings,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            home_page_id: None,
            navigation_style: default_navigation_style(),
            max_width: default_max_width(),
            show_branding: default_show_branding(),
            custom_css: None,
            builder_experience: default_builder_experience(),
            consumer_mode: ConsumerModeSettings::default(),
            interactive_workshop: WorkshopInteractiveSettings::default(),
            slate: SlateSettings::default(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SlatePackageFile {
    pub path: String,
    pub language: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlatePackageResponse {
    pub app_id: Uuid,
    pub app_slug: String,
    pub framework: String,
    pub package_name: String,
    pub entry_file: String,
    pub sdk_import: String,
    pub files: Vec<SlatePackageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportSlatePackageRequest {
    #[serde(default)]
    pub package_name: Option<String>,
    #[serde(default)]
    pub entry_file: Option<String>,
    #[serde(default)]
    pub sdk_import: Option<String>,
    #[serde(default)]
    pub framework: Option<String>,
    #[serde(default)]
    pub repository_id: Option<String>,
    #[serde(default)]
    pub layout: Option<String>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub dev_command: Option<String>,
    #[serde(default)]
    pub preview_command: Option<String>,
    #[serde(default)]
    pub files: Vec<SlatePackageFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlateRoundTripResponse {
    pub app: App,
    pub slate_package: SlatePackageResponse,
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

fn default_builder_experience() -> String {
    "workshop".to_string()
}

fn default_max_width() -> String {
    "1280px".to_string()
}

fn default_show_branding() -> bool {
    true
}

fn default_slate_framework() -> String {
    "react".to_string()
}

fn default_slate_package_name() -> String {
    "@open-foundry/slate-app".to_string()
}

fn default_slate_entry_file() -> String {
    "src/App.tsx".to_string()
}

fn default_slate_sdk_import() -> String {
    "@open-foundry/sdk/react".to_string()
}

fn default_workspace_layout() -> String {
    "split".to_string()
}

fn default_workspace_runtime() -> String {
    "typescript-react".to_string()
}

fn default_workspace_dev_command() -> String {
    "pnpm dev".to_string()
}

fn default_workspace_preview_command() -> String {
    "pnpm build".to_string()
}
