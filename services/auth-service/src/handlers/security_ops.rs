use auth_middleware::layer::AuthUser;
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::{AppState, domain::security};

use super::common::json_error;

#[derive(Debug, Deserialize)]
pub struct HashContentRequest {
    pub content: String,
    pub salt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HashContentResponse {
    pub algorithm: String,
    pub digest: String,
}

#[derive(Debug, Deserialize)]
pub struct SignContentRequest {
    pub content: String,
    pub key_material: String,
}

#[derive(Debug, Serialize)]
pub struct SignContentResponse {
    pub algorithm: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifySignatureRequest {
    pub content: String,
    pub key_material: String,
    pub signature: String,
}

#[derive(Debug, Serialize)]
pub struct VerifySignatureResponse {
    pub algorithm: String,
    pub valid: bool,
}

pub async fn hash_content(
    State(_state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<HashContentRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_security_write(&claims) {
        return response;
    }
    if body.content.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "content is required");
    }

    Json(HashContentResponse {
        algorithm: "sha256".to_string(),
        digest: security::hash_content(&body.content, body.salt.as_deref()),
    })
    .into_response()
}

pub async fn sign_content(
    State(_state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<SignContentRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_security_write(&claims) {
        return response;
    }
    if body.content.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "content is required");
    }
    if body.key_material.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "key_material is required");
    }

    Json(SignContentResponse {
        algorithm: "hmac-sha256".to_string(),
        signature: security::sign_content(&body.content, &body.key_material),
    })
    .into_response()
}

pub async fn verify_signature(
    State(_state): State<AppState>,
    AuthUser(claims): AuthUser,
    Json(body): Json<VerifySignatureRequest>,
) -> impl IntoResponse {
    if let Err(response) = require_security_write(&claims) {
        return response;
    }
    if body.content.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "content is required");
    }
    if body.key_material.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "key_material is required");
    }
    if body.signature.trim().is_empty() {
        return json_error(StatusCode::BAD_REQUEST, "signature is required");
    }

    Json(VerifySignatureResponse {
        algorithm: "hmac-sha256".to_string(),
        valid: security::verify_signature(&body.content, &body.key_material, &body.signature),
    })
    .into_response()
}

fn require_security_write(
    claims: &auth_middleware::Claims,
) -> Result<(), axum::response::Response> {
    if claims.has_role("admin") || claims.has_permission("control_panel", "write") {
        Ok(())
    } else {
        Err(json_error(
            StatusCode::FORBIDDEN,
            "missing permission control_panel:write",
        ))
    }
}
