use serde::{Deserialize, Serialize};

/// Cursor-based pagination request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRequest {
    pub cursor: Option<String>,
    pub limit: u32,
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            cursor: None,
            limit: 50,
        }
    }
}

/// Cursor-based pagination response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageResponse<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub total: Option<u64>,
}
