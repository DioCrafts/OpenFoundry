use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayerStyle {
    pub color: String,
    pub opacity: f64,
    pub radius: f64,
    pub line_width: f64,
    pub heatmap_intensity: f64,
    pub cluster_color: String,
    pub show_labels: bool,
}

impl Default for LayerStyle {
    fn default() -> Self {
        Self {
            color: "#D97706".to_string(),
            opacity: 0.78,
            radius: 9.0,
            line_width: 2.0,
            heatmap_intensity: 0.65,
            cluster_color: "#0F766E".to_string(),
            show_labels: true,
        }
    }
}
