use crate::models::{listing::CreateListingRequest, package::PublishVersionRequest};

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
	if request.changelog.trim().is_empty() {
		return Err("changelog is required".to_string());
	}
	Ok(())
}
