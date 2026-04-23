use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::claims::Claims;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuotaPolicy {
	pub max_query_limit: usize,
	pub max_distributed_query_workers: usize,
	pub max_pipeline_workers: usize,
	pub max_request_body_bytes: usize,
	pub requests_per_minute: u32,
}

impl TenantQuotaPolicy {
	pub fn standard() -> Self {
		Self {
			max_query_limit: 2_000,
			max_distributed_query_workers: 2,
			max_pipeline_workers: 2,
			max_request_body_bytes: 10 * 1024 * 1024,
			requests_per_minute: 300,
		}
	}

	pub fn team() -> Self {
		Self {
			max_query_limit: 5_000,
			max_distributed_query_workers: 4,
			max_pipeline_workers: 4,
			max_request_body_bytes: 20 * 1024 * 1024,
			requests_per_minute: 900,
		}
	}

	pub fn enterprise() -> Self {
		Self {
			max_query_limit: 10_000,
			max_distributed_query_workers: 8,
			max_pipeline_workers: 8,
			max_request_body_bytes: 50 * 1024 * 1024,
			requests_per_minute: 5_000,
		}
	}
}

impl Default for TenantQuotaPolicy {
	fn default() -> Self {
		Self::standard()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantContext {
	pub tenant_id: Option<Uuid>,
	pub scope_id: String,
	pub tier: String,
	pub workspace: Option<String>,
	pub quotas: TenantQuotaPolicy,
}

impl TenantContext {
	pub fn from_claims(claims: &Claims) -> Self {
		let tier = claims
			.attribute("tenant_tier")
			.and_then(Value::as_str)
			.map(str::to_owned)
			.unwrap_or_else(|| {
				if claims.has_role("admin") {
					"enterprise".to_string()
				} else {
					"standard".to_string()
				}
			});

		let mut quotas = match tier.as_str() {
			"enterprise" => TenantQuotaPolicy::enterprise(),
			"team" => TenantQuotaPolicy::team(),
			_ => TenantQuotaPolicy::standard(),
		};
		apply_quota_overrides(&mut quotas, claims.attribute("tenant_quotas"));

		let workspace = claims
			.attribute("workspace")
			.and_then(Value::as_str)
			.map(str::to_owned);
		let scope_uuid = claims.org_id.unwrap_or(claims.sub);

		Self {
			tenant_id: claims.org_id,
			scope_id: scope_uuid.to_string(),
			tier,
			workspace,
			quotas,
		}
	}

	pub fn clamp_query_limit(&self, requested: usize) -> usize {
		requested.min(self.quotas.max_query_limit.max(1))
	}

	pub fn clamp_query_workers(&self, requested: usize) -> usize {
		requested.min(self.quotas.max_distributed_query_workers.max(1))
	}

	pub fn clamp_pipeline_workers(&self, requested: usize) -> usize {
		requested.min(self.quotas.max_pipeline_workers.max(1))
	}

	pub fn clamp_request_body_bytes(&self, requested: usize) -> usize {
		requested.min(self.quotas.max_request_body_bytes.max(1))
	}
	}

fn apply_quota_overrides(quotas: &mut TenantQuotaPolicy, overrides: Option<&Value>) {
	let Some(overrides) = overrides.and_then(Value::as_object) else {
		return;
	};

	if let Some(value) = overrides.get("max_query_limit").and_then(Value::as_u64) {
		quotas.max_query_limit = value as usize;
	}
	if let Some(value) = overrides
		.get("max_distributed_query_workers")
		.and_then(Value::as_u64)
	{
		quotas.max_distributed_query_workers = value as usize;
	}
	if let Some(value) = overrides.get("max_pipeline_workers").and_then(Value::as_u64) {
		quotas.max_pipeline_workers = value as usize;
	}
	if let Some(value) = overrides.get("max_request_body_bytes").and_then(Value::as_u64) {
		quotas.max_request_body_bytes = value as usize;
	}
	if let Some(value) = overrides.get("requests_per_minute").and_then(Value::as_u64) {
		quotas.requests_per_minute = value as u32;
	}
}