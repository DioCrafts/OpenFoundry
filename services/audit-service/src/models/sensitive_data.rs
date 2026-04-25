use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataFinding {
    pub kind: String,
    pub value: String,
    pub redacted: String,
    pub match_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SensitiveDataScanRequest {
    pub content: String,
    #[serde(default = "default_redact")]
    pub redact: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensitiveDataScanResponse {
    pub findings: Vec<SensitiveDataFinding>,
    pub redacted_content: String,
    pub risk_score: u32,
}

fn default_redact() -> bool {
    true
}
