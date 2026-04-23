use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use auth_middleware::{jwt, tenant::TenantContext, JwtConfig};
use reqwest::Client;

use crate::config::GatewayConfig;

/// Reverse-proxy handler: forwards requests to backend services based on URL prefix.
pub async fn proxy_handler(
    State((config, client)): State<(GatewayConfig, Client)>,
    mut req: Request,
) -> Response {
    let path = req.uri().path();
    let tenant = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .and_then(|token| jwt::decode_token(&JwtConfig::new(&config.jwt_secret), token).ok())
        .map(|claims| TenantContext::from_claims(&claims));

    let upstream_base = if path.starts_with("/api/v1/auth") {
        &config.auth_service_url
    } else if path.starts_with("/api/v1/datasets") {
        &config.dataset_service_url
    } else if path.starts_with("/api/v1/queries") {
        &config.query_service_url
    } else if path.starts_with("/api/v1/pipelines") {
        &config.pipeline_service_url
    } else if path.starts_with("/api/v1/ontology") {
        &config.ontology_service_url
    } else if path.starts_with("/api/v1/workflows") {
        &config.workflow_service_url
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
        return (StatusCode::NOT_FOUND, "unknown service route").into_response();
    };

    let uri = format!("{upstream_base}{}", req.uri().path_and_query().map(|pq| pq.as_str()).unwrap_or("/"));

    let Ok(uri) = uri.parse::<Uri>() else {
        return (StatusCode::BAD_GATEWAY, "invalid upstream URI").into_response();
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
        Err(_) => return (StatusCode::PAYLOAD_TOO_LARGE, "body too large").into_response(),
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
            .header("x-openfoundry-quota-query-limit", tenant.quotas.max_query_limit.to_string())
            .header(
                "x-openfoundry-quota-pipeline-workers",
                tenant.quotas.max_pipeline_workers.to_string(),
            )
            .header(
                "x-openfoundry-quota-requests-per-minute",
                tenant.quotas.requests_per_minute.to_string(),
            );
    }
    upstream_req = upstream_req.body(body_bytes);

    match upstream_req.send().await {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
            let headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut response = Response::builder().status(status);
            for (key, value) in headers.iter() {
                response = response.header(key, value);
            }
            response.body(Body::from(body)).unwrap_or_else(|_| {
                (StatusCode::INTERNAL_SERVER_ERROR, "proxy error").into_response()
            })
        }
        Err(e) => {
            tracing::error!("upstream request failed: {e}");
            (StatusCode::BAD_GATEWAY, "upstream unavailable").into_response()
        }
    }
}
