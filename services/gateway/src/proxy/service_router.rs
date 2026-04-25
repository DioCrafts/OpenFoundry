use auth_middleware::{Claims, JwtConfig, jwt, tenant::TenantContext};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{Method, StatusCode, Uri, header::AUTHORIZATION},
    response::{IntoResponse, Response},
};
use reqwest::Client;
use serde_json::json;

use crate::config::GatewayConfig;

/// Reverse-proxy handler: forwards requests to backend services based on URL prefix.
pub async fn proxy_handler(
    State((config, client, jwt_config)): State<(GatewayConfig, Client, JwtConfig)>,
    mut req: Request,
) -> Response {
    let path = req.uri().path();
    let claims = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| jwt::decode_token(&jwt_config, token).ok());
    if let Some(claims) = claims.as_ref() {
        if let Err(response) = enforce_zero_trust_scope(claims, req.method(), path) {
            return response;
        }
    }
    let tenant = claims.as_ref().map(TenantContext::from_claims);

    let upstream_base = if path.starts_with("/api/v1/auth")
        || path.starts_with("/api/v1/users")
        || path.starts_with("/api/v1/roles")
        || path.starts_with("/api/v1/permissions")
        || path.starts_with("/api/v1/groups")
        || path.starts_with("/api/v1/policies")
        || path.starts_with("/api/v1/restricted-views")
        || path.starts_with("/api/v1/api-keys")
        || path.starts_with("/api/v1/control-panel")
        || path.starts_with("/api/v2/admin")
    {
        &config.auth_service_url
    } else if path.starts_with("/api/v1/connections")
        || path.starts_with("/api/v1/connector-agents")
    {
        &config.data_connector_service_url
    } else if path.starts_with("/api/v1/datasets") || path.starts_with("/api/v2/filesystem") {
        &config.dataset_service_url
    } else if path.starts_with("/api/v1/queries") {
        &config.query_service_url
    } else if path.starts_with("/api/v1/pipelines") {
        &config.pipeline_service_url
    } else if path.starts_with("/api/v1/lineage") {
        &config.pipeline_service_url
    } else if path.starts_with("/api/v1/ontology") {
        &config.ontology_service_url
    } else if path.starts_with("/api/v1/workflows") {
        &config.workflow_service_url
    } else if path.starts_with("/api/v1/notebooks") || path.starts_with("/api/v1/notepad") {
        &config.notebook_service_url
    } else if path.starts_with("/api/v1/notifications") {
        &config.notification_service_url
    } else if path.starts_with("/api/v1/ml") {
        &config.ml_service_url
    } else if path.starts_with("/api/v1/ai") {
        &config.ai_service_url
    } else if path.starts_with("/api/v1/fusion") {
        &config.fusion_service_url
    } else if path.starts_with("/api/v1/streaming") {
        &config.streaming_service_url
    } else if path.starts_with("/api/v1/reports") {
        &config.report_service_url
    } else if path.starts_with("/api/v1/geospatial") {
        &config.geospatial_service_url
    } else if path.starts_with("/api/v1/code-repos") {
        &config.code_repo_service_url
    } else if path.starts_with("/api/v1/marketplace") {
        &config.marketplace_service_url
    } else if path.starts_with("/api/v1/audit") {
        &config.audit_service_url
    } else if path.starts_with("/api/v1/nexus") {
        &config.nexus_service_url
    } else if path.starts_with("/api/v1/apps") || path.starts_with("/api/v1/widgets") {
        &config.app_builder_service_url
    } else {
        return gateway_error(
            StatusCode::NOT_FOUND,
            "unknown_service_route",
            "unknown service route",
        );
    };

    let uri = format!(
        "{upstream_base}{}",
        req.uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/")
    );

    let Ok(uri) = uri.parse::<Uri>() else {
        return gateway_error(
            StatusCode::BAD_GATEWAY,
            "invalid_upstream_uri",
            "invalid upstream URI",
        );
    };
    *req.uri_mut() = uri;

    // Forward the request via reqwest
    let method = req.method().clone();
    let url = req.uri().to_string();
    let headers = req.headers().clone();
    let body_limit = tenant
        .as_ref()
        .map(|tenant| tenant.clamp_request_body_bytes(10 * 1024 * 1024))
        .unwrap_or(10 * 1024 * 1024);

    let body_bytes = match axum::body::to_bytes(req.into_body(), body_limit).await {
        Ok(b) => b,
        Err(_) => {
            return gateway_error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "body_too_large",
                "body too large",
            );
        }
    };

    let mut upstream_req = client.request(method, &url);
    for (key, value) in headers.iter() {
        if key != "host" {
            upstream_req = upstream_req.header(key, value);
        }
    }
    if let Some(tenant) = tenant {
        upstream_req = upstream_req
            .header("x-openfoundry-tenant-scope", tenant.scope_id)
            .header("x-openfoundry-tenant-tier", tenant.tier)
            .header(
                "x-openfoundry-quota-query-limit",
                tenant.quotas.max_query_limit.to_string(),
            )
            .header(
                "x-openfoundry-quota-pipeline-workers",
                tenant.quotas.max_pipeline_workers.to_string(),
            )
            .header(
                "x-openfoundry-quota-requests-per-minute",
                tenant.quotas.requests_per_minute.to_string(),
            );
    }
    if let Some(claims) = claims.as_ref() {
        upstream_req = apply_auth_context_headers(upstream_req, claims);
    }
    upstream_req = upstream_req.body(body_bytes);

    match upstream_req.send().await {
        Ok(resp) => {
            let status =
                StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut response = Response::builder().status(status);
            for (key, value) in headers.iter() {
                response = response.header(key, value);
            }
            response.body(Body::from(body)).unwrap_or_else(|_| {
                gateway_error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "proxy_response_build_failed",
                    "proxy error",
                )
            })
        }
        Err(e) => {
            tracing::error!("upstream request failed: {e}");
            gateway_error(
                StatusCode::BAD_GATEWAY,
                "upstream_unavailable",
                "upstream unavailable",
            )
        }
    }
}

fn enforce_zero_trust_scope(claims: &Claims, method: &Method, path: &str) -> Result<(), Response> {
    if !claims.allows_http_method(method.as_str()) {
        return Err(gateway_error(
            StatusCode::FORBIDDEN,
            "scoped_session_method_denied",
            "session scope does not allow this HTTP method",
        ));
    }
    if !claims.allows_path(path) {
        return Err(gateway_error(
            StatusCode::FORBIDDEN,
            "scoped_session_path_denied",
            "session scope does not allow this path",
        ));
    }
    Ok(())
}

fn apply_auth_context_headers(
    mut request: reqwest::RequestBuilder,
    claims: &Claims,
) -> reqwest::RequestBuilder {
    request = request
        .header("x-openfoundry-auth-sub", claims.sub.to_string())
        .header("x-openfoundry-auth-email", claims.email.as_str())
        .header("x-openfoundry-auth-methods", claims.auth_methods.join(","))
        .header(
            "x-openfoundry-zero-trust",
            if claims.session_scope.is_some() {
                "scoped"
            } else {
                "standard"
            },
        );

    if let Some(org_id) = claims.org_id {
        request = request.header("x-openfoundry-org-id", org_id.to_string());
    }
    if let Some(session_kind) = claims.session_kind.as_deref() {
        request = request.header("x-openfoundry-session-kind", session_kind);
    }
    if let Some(clearance) = claims.classification_clearance() {
        request = request.header("x-openfoundry-classification-clearance", clearance);
    }
    if let Some(scope) = claims.session_scope.as_ref() {
        if let Some(workspace) = scope.workspace.as_deref() {
            request = request.header("x-openfoundry-scope-workspace", workspace);
        }
        if !scope.allowed_path_prefixes.is_empty() {
            request = request.header(
                "x-openfoundry-scope-path-prefixes",
                scope.allowed_path_prefixes.join(","),
            );
        }
        if !scope.allowed_org_ids.is_empty() {
            request = request.header(
                "x-openfoundry-allowed-org-ids",
                scope
                    .allowed_org_ids
                    .iter()
                    .map(uuid::Uuid::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        if !scope.allowed_markings.is_empty() {
            request = request.header(
                "x-openfoundry-allowed-markings",
                scope.allowed_markings.join(","),
            );
        }
        if !scope.restricted_view_ids.is_empty() {
            request = request.header(
                "x-openfoundry-restricted-view-ids",
                scope
                    .restricted_view_ids
                    .iter()
                    .map(uuid::Uuid::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
        }
        if scope.consumer_mode {
            request = request.header("x-openfoundry-consumer-mode", "true");
        }
        if let Some(guest_email) = scope.guest_email.as_deref() {
            request = request
                .header("x-openfoundry-guest-email", guest_email)
                .header("x-openfoundry-guest-access", "true");
        }
    }

    request
}

fn gateway_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        axum::Json(json!({
            "error": {
                "code": code,
                "message": message,
            }
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use uuid::Uuid;

    use super::*;
    use auth_middleware::claims::SessionScope;

    fn scoped_claims() -> Claims {
        Claims {
            sub: Uuid::nil(),
            iat: 0,
            exp: i64::MAX,
            iss: None,
            aud: None,
            jti: Uuid::nil(),
            email: "guest@example.com".to_string(),
            name: "Guest".to_string(),
            roles: vec!["guest".to_string()],
            permissions: vec!["datasets:read".to_string()],
            org_id: Some(Uuid::nil()),
            attributes: json!({ "classification_clearance": "confidential" }),
            auth_methods: vec!["guest".to_string()],
            token_use: Some("access".to_string()),
            api_key_id: None,
            session_kind: Some("guest_session".to_string()),
            session_scope: Some(SessionScope {
                allowed_methods: vec!["GET".to_string()],
                allowed_path_prefixes: vec!["/api/v1/datasets".to_string()],
                allowed_subject_ids: vec![],
                allowed_org_ids: vec![Uuid::nil()],
                workspace: Some("shared".to_string()),
                classification_clearance: Some("public".to_string()),
                allowed_markings: vec!["public".to_string()],
                restricted_view_ids: vec![Uuid::nil()],
                consumer_mode: true,
                guest_email: Some("guest@example.com".to_string()),
                guest_display_name: Some("Guest".to_string()),
            }),
        }
    }

    #[test]
    fn zero_trust_scope_blocks_disallowed_requests() {
        let claims = scoped_claims();
        assert!(enforce_zero_trust_scope(&claims, &Method::GET, "/api/v1/datasets").is_ok());
        assert!(enforce_zero_trust_scope(&claims, &Method::POST, "/api/v1/datasets").is_err());
        assert!(enforce_zero_trust_scope(&claims, &Method::GET, "/api/v1/pipelines").is_err());
    }
}
