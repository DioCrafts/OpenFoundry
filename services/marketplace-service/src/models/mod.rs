pub mod category;
pub mod install;
pub mod listing;
pub mod package;
pub mod review;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
	pub items: Vec<T>,
}

pub(crate) fn decode_json<T: DeserializeOwned>(value: serde_json::Value, field: &str) -> Result<T, String> {
	serde_json::from_value(value).map_err(|cause| format!("failed to decode {field}: {cause}"))
}