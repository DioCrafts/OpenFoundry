use auth_middleware::claims::Claims;

use crate::handlers::objects::ObjectInstance;

pub const VALID_MARKINGS: &[&str] = &["public", "confidential", "pii"];

pub fn validate_marking(marking: &str) -> Result<(), String> {
    if VALID_MARKINGS.contains(&marking) {
        Ok(())
    } else {
        Err(format!(
            "invalid marking '{marking}', valid markings: {VALID_MARKINGS:?}"
        ))
    }
}

pub fn ensure_object_access(claims: &Claims, object: &ObjectInstance) -> Result<(), String> {
    if claims.has_role("admin") {
        return Ok(());
    }

    if let Some(org_id) = claims.org_id {
        if let Some(object_org_id) = object.organization_id {
            if object_org_id != org_id {
                return Err("forbidden: object belongs to a different organization".to_string());
            }
        }
    }

    let required = marking_rank(&object.marking)
        .ok_or_else(|| format!("forbidden: unsupported object marking '{}'", object.marking))?;
    let granted = clearance_rank(claims);
    if granted < required {
        return Err("forbidden: insufficient classification clearance".to_string());
    }

    Ok(())
}

pub fn clearance_rank(claims: &Claims) -> u8 {
    claims
        .attribute("classification_clearance")
        .and_then(|value| value.as_str())
        .and_then(marking_rank)
        .unwrap_or(0)
}

pub fn marking_rank(marking: &str) -> Option<u8> {
    match marking {
        "public" => Some(0),
        "confidential" => Some(1),
        "pii" => Some(2),
        _ => None,
    }
}
