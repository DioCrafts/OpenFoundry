pub mod access_grant;
pub mod contract;
pub mod peer;
pub mod share;
pub mod space;
pub mod sync_status;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
}

pub(crate) fn decode_json<T: DeserializeOwned>(
    value: serde_json::Value,
    field: &str,
) -> Result<T, String> {
    serde_json::from_value(value).map_err(|cause| format!("failed to decode {field}: {cause}"))
}
