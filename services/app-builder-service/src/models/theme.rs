use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppTheme {
    #[serde(default = "default_theme_name")]
    pub name: String,
    #[serde(default = "default_primary_color")]
    pub primary_color: String,
    #[serde(default = "default_accent_color")]
    pub accent_color: String,
    #[serde(default = "default_background_color")]
    pub background_color: String,
    #[serde(default = "default_surface_color")]
    pub surface_color: String,
    #[serde(default = "default_text_color")]
    pub text_color: String,
    #[serde(default = "default_heading_font")]
    pub heading_font: String,
    #[serde(default = "default_body_font")]
    pub body_font: String,
    #[serde(default = "default_radius")]
    pub border_radius: u16,
    #[serde(default)]
    pub logo_url: Option<String>,
}

impl Default for AppTheme {
    fn default() -> Self {
        Self {
            name: default_theme_name(),
            primary_color: default_primary_color(),
            accent_color: default_accent_color(),
            background_color: default_background_color(),
            surface_color: default_surface_color(),
            text_color: default_text_color(),
            heading_font: default_heading_font(),
            body_font: default_body_font(),
            border_radius: default_radius(),
            logo_url: None,
        }
    }
}

fn default_theme_name() -> String {
    "Signal".to_string()
}

fn default_primary_color() -> String {
    "#0f766e".to_string()
}

fn default_accent_color() -> String {
    "#f97316".to_string()
}

fn default_background_color() -> String {
    "#f8fafc".to_string()
}

fn default_surface_color() -> String {
    "#ffffff".to_string()
}

fn default_text_color() -> String {
    "#0f172a".to_string()
}

fn default_heading_font() -> String {
    "Space Grotesk".to_string()
}

fn default_body_font() -> String {
    "Manrope".to_string()
}

fn default_radius() -> u16 {
    20
}
