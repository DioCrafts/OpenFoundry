use crate::models::{
    devops::{
        CreateEnrollmentBranchRequest, CreateProductFleetRequest, CreatePromotionGateRequest,
    },
    listing::CreateListingRequest,
    package::PublishVersionRequest,
};

pub fn validate_listing(request: &CreateListingRequest) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("listing name is required".to_string());
    }
    if request.slug.trim().is_empty() {
        return Err("listing slug is required".to_string());
    }
    if request.category_slug.trim().is_empty() {
        return Err("category is required".to_string());
    }
    Ok(())
}

pub fn validate_version(request: &PublishVersionRequest) -> Result<(), String> {
    if request.version.trim().is_empty() {
        return Err("version is required".to_string());
    }
    if request.release_channel.trim().is_empty() {
        return Err("release channel is required".to_string());
    }
    if request.changelog.trim().is_empty() {
        return Err("changelog is required".to_string());
    }
    Ok(())
}

pub fn validate_product_fleet(request: &CreateProductFleetRequest) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("fleet name is required".to_string());
    }
    if request.release_channel.trim().is_empty() {
        return Err("fleet release channel is required".to_string());
    }
    if request.workspace_targets.is_empty() {
        return Err("at least one workspace target is required".to_string());
    }
    for cell in &request.deployment_cells {
        if cell.name.trim().is_empty() {
            return Err("deployment cells require a name".to_string());
        }
        if cell.cloud.trim().is_empty() || cell.region.trim().is_empty() {
            return Err("deployment cells require cloud and region".to_string());
        }
    }
    Ok(())
}

pub fn validate_enrollment_branch(request: &CreateEnrollmentBranchRequest) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("branch name is required".to_string());
    }
    Ok(())
}

pub fn validate_promotion_gate(request: &CreatePromotionGateRequest) -> Result<(), String> {
    if request.name.trim().is_empty() {
        return Err("promotion gate name is required".to_string());
    }
    if request.gate_kind.trim().is_empty() {
        return Err("promotion gate kind is required".to_string());
    }
    if let Some(status) = request.status.as_deref() {
        validate_gate_status(status)?;
    }
    Ok(())
}

pub fn validate_gate_status(status: &str) -> Result<(), String> {
    match status {
        "pending" | "passed" | "failed" | "waived" => Ok(()),
        other => Err(format!(
            "promotion gate status must be one of pending, passed, failed, waived; got '{other}'"
        )),
    }
}
