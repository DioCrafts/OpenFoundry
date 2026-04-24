use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDefinition {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub listing_count: usize,
}

impl CategoryDefinition {
    pub fn new(slug: &str, name: &str, description: &str) -> Self {
        Self {
            slug: slug.to_string(),
            name: name.to_string(),
            description: description.to_string(),
            listing_count: 0,
        }
    }
}
