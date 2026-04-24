use chrono::Utc;

use crate::models::access_grant::AccessGrant;

pub fn validate_access(grant: &AccessGrant, purpose: &str) -> Result<(), String> {
    if grant.expires_at < Utc::now() {
        return Err("access grant has expired".to_string());
    }

    if !grant
        .allowed_purposes
        .iter()
        .any(|candidate| candidate == purpose)
    {
        return Err(format!(
            "purpose '{purpose}' is not allowed by this contract"
        ));
    }

    Ok(())
}

pub fn resolve_limit(grant: &AccessGrant, requested: Option<usize>) -> usize {
    let grant_limit = usize::try_from(grant.max_rows_per_query).unwrap_or(1000);
    requested.unwrap_or(grant_limit).min(grant_limit).max(1)
}
