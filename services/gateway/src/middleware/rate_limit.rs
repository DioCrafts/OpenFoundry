use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use auth_middleware::{
    jwt::{self, JwtConfig},
    tenant::TenantContext,
};
use axum::{
    extract::{Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header::AUTHORIZATION},
    middleware::Next,
    response::{IntoResponse, Response},
};
use redis::{Script, aio::ConnectionManager};
use serde::Deserialize;
use serde_json::json;

static X_RATE_LIMIT_LIMIT: HeaderName = HeaderName::from_static("x-ratelimit-limit");
static X_RATE_LIMIT_REMAINING: HeaderName = HeaderName::from_static("x-ratelimit-remaining");
static X_RATE_LIMIT_RESET: HeaderName = HeaderName::from_static("x-ratelimit-reset");
static RETRY_AFTER: HeaderName = HeaderName::from_static("retry-after");

const REDIS_TOKEN_BUCKET_SCRIPT: &str = r#"
local key = KEYS[1]
local now_ms = tonumber(ARGV[1])
local ttl_secs = tonumber(ARGV[2])
local limit_per_minute = math.max(1, tonumber(ARGV[3]))
local configured_burst = math.max(1, tonumber(ARGV[4]))
local burst = math.min(configured_burst, limit_per_minute)
local refill_rate = limit_per_minute / 60000.0

local data = redis.call("HMGET", key, "tokens", "last_refill", "limit_per_minute")
local tokens = tonumber(data[1])
local last_refill = tonumber(data[2])
local previous_limit = tonumber(data[3])

if not tokens or not last_refill then
  tokens = burst
  last_refill = now_ms
  previous_limit = limit_per_minute
else
  local elapsed = math.max(0, now_ms - last_refill)
  if elapsed > 0 then
    tokens = math.min(burst, tokens + (elapsed * refill_rate))
    last_refill = now_ms
  end

  if previous_limit ~= limit_per_minute then
    tokens = math.min(tokens, burst)
    previous_limit = limit_per_minute
  end
end

local allowed = 0
local remaining = 0
local retry_after_ms = 0

if tokens >= 1 then
  tokens = tokens - 1
  allowed = 1
  remaining = math.max(0, math.floor(tokens))
else
  remaining = 0
  retry_after_ms = math.max(1000, math.ceil((1.0 - tokens) / refill_rate))
end

redis.call(
  "HSET",
  key,
  "tokens",
  tokens,
  "last_refill",
  last_refill,
  "limit_per_minute",
  limit_per_minute
)
redis.call("EXPIRE", key, ttl_secs)

return { allowed, limit_per_minute, remaining, retry_after_ms }
"#;

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitConfig {
    #[serde(default = "default_anonymous_requests_per_minute")]
    pub anonymous_requests_per_minute: u32,
    #[serde(default = "default_burst_size")]
    pub burst_size: u32,
    #[serde(default = "default_bucket_ttl_secs")]
    pub bucket_ttl_secs: u64,
    #[serde(default = "default_redis_key_prefix")]
    pub redis_key_prefix: String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            anonymous_requests_per_minute: default_anonymous_requests_per_minute(),
            burst_size: default_burst_size(),
            bucket_ttl_secs: default_bucket_ttl_secs(),
            redis_key_prefix: default_redis_key_prefix(),
        }
    }
}

fn default_anonymous_requests_per_minute() -> u32 {
    120
}

fn default_burst_size() -> u32 {
    30
}

fn default_bucket_ttl_secs() -> u64 {
    300
}

fn default_redis_key_prefix() -> String {
    "openfoundry:ratelimit".to_string()
}

#[derive(Clone)]
pub struct RateLimitState {
    jwt_config: JwtConfig,
    config: RateLimitConfig,
    backend: RateLimitBackend,
    fallback_store: Arc<Mutex<BucketStore>>,
}

#[derive(Clone)]
enum RateLimitBackend {
    InMemory(Arc<Mutex<BucketStore>>),
    Redis {
        connection: Arc<tokio::sync::Mutex<ConnectionManager>>,
        key_prefix: String,
    },
}

#[derive(Debug)]
struct BucketStore {
    buckets: HashMap<String, TokenBucket>,
    last_cleanup: Instant,
}

#[derive(Debug, Clone)]
struct TokenBucket {
    tokens: f64,
    last_refill: Instant,
    last_seen: Instant,
    limit_per_minute: u32,
}

#[derive(Debug, Clone)]
struct RateLimitIdentity {
    key: String,
    limit_per_minute: u32,
}

#[derive(Debug, Clone)]
struct RateLimitDecision {
    allowed: bool,
    limit_per_minute: u32,
    remaining: u32,
    retry_after_secs: Option<u64>,
}

impl RateLimitState {
    pub async fn new(
        jwt_config: JwtConfig,
        config: RateLimitConfig,
        redis_url: Option<String>,
    ) -> Self {
        let fallback_store = Arc::new(Mutex::new(BucketStore {
            buckets: HashMap::new(),
            last_cleanup: Instant::now(),
        }));

        let backend = match redis_url {
            Some(url) => match build_redis_backend(&url, &config.redis_key_prefix).await {
                Ok(backend) => backend,
                Err(error) => {
                    tracing::warn!(
                        redis_url = %redact_url(&url),
                        %error,
                        "failed to initialize Redis rate limiting; falling back to local buckets"
                    );
                    RateLimitBackend::InMemory(fallback_store.clone())
                }
            },
            None => RateLimitBackend::InMemory(fallback_store.clone()),
        };

        Self {
            jwt_config,
            config,
            backend,
            fallback_store,
        }
    }

    #[cfg(test)]
    fn in_memory(jwt_config: JwtConfig, config: RateLimitConfig) -> Self {
        let store = Arc::new(Mutex::new(BucketStore {
            buckets: HashMap::new(),
            last_cleanup: Instant::now(),
        }));
        Self {
            jwt_config,
            config,
            backend: RateLimitBackend::InMemory(store.clone()),
            fallback_store: store,
        }
    }

    fn classify_request(&self, req: &Request) -> RateLimitIdentity {
        if let Some(tenant) = self.tenant_context(req) {
            return RateLimitIdentity {
                key: format!("tenant:{}", tenant.scope_id),
                limit_per_minute: tenant.quotas.requests_per_minute.max(1),
            };
        }

        let client_key = forwarded_client_id(req).unwrap_or_else(|| "global".to_string());
        RateLimitIdentity {
            key: format!("anonymous:{client_key}"),
            limit_per_minute: self.config.anonymous_requests_per_minute.max(1),
        }
    }

    fn tenant_context(&self, req: &Request) -> Option<TenantContext> {
        req.headers()
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .and_then(|token| jwt::decode_token(&self.jwt_config, token).ok())
            .map(|claims| TenantContext::from_claims(&claims))
    }

    async fn check(&self, identity: &RateLimitIdentity, now: Instant) -> RateLimitDecision {
        match &self.backend {
            RateLimitBackend::InMemory(store) => {
                check_in_memory(store, &self.config, identity, now)
            }
            RateLimitBackend::Redis {
                connection,
                key_prefix,
            } => match check_redis(connection, key_prefix, &self.config, identity).await {
                Ok(decision) => decision,
                Err(error) => {
                    tracing::warn!(
                        %error,
                        "Redis rate limit check failed; falling back to local bucket"
                    );
                    check_in_memory(&self.fallback_store, &self.config, identity, now)
                }
            },
        }
    }
}

pub async fn rate_limit_layer(
    State(state): State<RateLimitState>,
    req: Request,
    next: Next,
) -> Response {
    let identity = state.classify_request(&req);
    let decision = state.check(&identity, Instant::now()).await;

    if !decision.allowed {
        let mut response = (
            StatusCode::TOO_MANY_REQUESTS,
            axum::Json(json!({
                "error": {
                    "code": "rate_limit_exceeded",
                    "message": "rate limit exceeded",
                }
            })),
        )
            .into_response();
        apply_headers(&mut response, &decision);
        return response;
    }

    let mut response = next.run(req).await;
    apply_headers(&mut response, &decision);
    response
}

fn check_in_memory(
    store: &Arc<Mutex<BucketStore>>,
    config: &RateLimitConfig,
    identity: &RateLimitIdentity,
    now: Instant,
) -> RateLimitDecision {
    let mut store = store.lock().expect("rate limit store mutex poisoned");

    maybe_cleanup(&mut store, now, config.bucket_ttl_secs);

    let bucket_capacity = burst_capacity(identity.limit_per_minute, config.burst_size);
    let entry = store
        .buckets
        .entry(identity.key.clone())
        .or_insert_with(|| TokenBucket {
            tokens: bucket_capacity,
            last_refill: now,
            last_seen: now,
            limit_per_minute: identity.limit_per_minute,
        });

    refill(entry, now, config.burst_size);

    if entry.limit_per_minute != identity.limit_per_minute {
        entry.limit_per_minute = identity.limit_per_minute;
        entry.tokens = entry
            .tokens
            .min(burst_capacity(identity.limit_per_minute, config.burst_size));
    }
    entry.last_seen = now;

    if entry.tokens >= 1.0 {
        entry.tokens -= 1.0;
        return RateLimitDecision {
            allowed: true,
            limit_per_minute: identity.limit_per_minute,
            remaining: entry.tokens.floor().max(0.0) as u32,
            retry_after_secs: None,
        };
    }

    let refill_rate = refill_rate_per_second(identity.limit_per_minute);
    let retry_after_secs = if refill_rate > 0.0 {
        ((1.0 - entry.tokens) / refill_rate).ceil().max(1.0) as u64
    } else {
        60
    };

    RateLimitDecision {
        allowed: false,
        limit_per_minute: identity.limit_per_minute,
        remaining: 0,
        retry_after_secs: Some(retry_after_secs),
    }
}

async fn check_redis(
    connection: &Arc<tokio::sync::Mutex<ConnectionManager>>,
    key_prefix: &str,
    config: &RateLimitConfig,
    identity: &RateLimitIdentity,
) -> Result<RateLimitDecision, redis::RedisError> {
    let key = format!("{key_prefix}:{}", identity.key);
    let now_ms = now_unix_millis();
    let script = Script::new(REDIS_TOKEN_BUCKET_SCRIPT);
    let mut connection = connection.lock().await;
    let (allowed, limit, remaining, retry_after_ms): (i64, i64, i64, i64) = script
        .key(key)
        .arg(now_ms)
        .arg(config.bucket_ttl_secs.max(60))
        .arg(i64::from(identity.limit_per_minute.max(1)))
        .arg(i64::from(config.burst_size.max(1)))
        .invoke_async(&mut *connection)
        .await?;

    Ok(RateLimitDecision {
        allowed: allowed == 1,
        limit_per_minute: limit.max(1) as u32,
        remaining: remaining.max(0) as u32,
        retry_after_secs: if retry_after_ms > 0 {
            Some(((retry_after_ms as u64) + 999) / 1000)
        } else {
            None
        },
    })
}

async fn build_redis_backend(
    redis_url: &str,
    key_prefix: &str,
) -> Result<RateLimitBackend, redis::RedisError> {
    let client = redis::Client::open(redis_url)?;
    let connection = ConnectionManager::new(client).await?;
    Ok(RateLimitBackend::Redis {
        connection: Arc::new(tokio::sync::Mutex::new(connection)),
        key_prefix: key_prefix.to_string(),
    })
}

fn forwarded_client_id(req: &Request) -> Option<String> {
    for header in ["x-forwarded-for", "x-real-ip", "cf-connecting-ip"] {
        let value = req.headers().get(header)?.to_str().ok()?;
        let first = value.split(',').next()?.trim();
        if !first.is_empty() {
            return Some(first.to_string());
        }
    }
    None
}

fn apply_headers(response: &mut Response, decision: &RateLimitDecision) {
    insert_header(
        response,
        &X_RATE_LIMIT_LIMIT,
        &decision.limit_per_minute.to_string(),
    );
    insert_header(
        response,
        &X_RATE_LIMIT_REMAINING,
        &decision.remaining.to_string(),
    );

    if let Some(retry_after_secs) = decision.retry_after_secs {
        insert_header(response, &RETRY_AFTER, &retry_after_secs.to_string());
        insert_header(response, &X_RATE_LIMIT_RESET, &retry_after_secs.to_string());
    } else {
        insert_header(response, &X_RATE_LIMIT_RESET, "0");
    }
}

fn insert_header(response: &mut Response, name: &'static HeaderName, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name.clone(), value);
    }
}

fn maybe_cleanup(store: &mut BucketStore, now: Instant, ttl_secs: u64) {
    let cleanup_interval = Duration::from_secs(ttl_secs.max(60));
    if now.duration_since(store.last_cleanup) < cleanup_interval {
        return;
    }

    let ttl = Duration::from_secs(ttl_secs.max(60));
    store
        .buckets
        .retain(|_, bucket| now.duration_since(bucket.last_seen) <= ttl);
    store.last_cleanup = now;
}

fn refill(bucket: &mut TokenBucket, now: Instant, burst_size: u32) {
    let elapsed = now.duration_since(bucket.last_refill).as_secs_f64();
    if elapsed <= 0.0 {
        return;
    }

    let refill = elapsed * refill_rate_per_second(bucket.limit_per_minute);
    bucket.tokens =
        (bucket.tokens + refill).min(burst_capacity(bucket.limit_per_minute, burst_size));
    bucket.last_refill = now;
}

fn burst_capacity(limit_per_minute: u32, configured_burst_size: u32) -> f64 {
    configured_burst_size.max(1).min(limit_per_minute.max(1)) as f64
}

fn refill_rate_per_second(limit_per_minute: u32) -> f64 {
    limit_per_minute.max(1) as f64 / 60.0
}

fn now_unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn redact_url(url: &str) -> String {
    if let Some((scheme, rest)) = url.split_once("://") {
        if let Some((_, host)) = rest.rsplit_once('@') {
            return format!("{scheme}://***@{host}");
        }
    }
    url.to_string()
}

#[cfg(test)]
mod tests {
    use auth_middleware::{
        claims::Claims,
        jwt::{build_access_claims, encode_token},
    };
    use axum::{
        Router,
        body::{Body, to_bytes},
        http::{Request as HttpRequest, StatusCode},
        middleware as axum_mw,
        routing::get,
    };
    use serde_json::json;
    use tower::ServiceExt;
    use uuid::Uuid;

    use super::*;

    fn test_app(state: RateLimitState) -> Router {
        Router::new()
            .route("/api/v1/test", get(|| async { "ok" }))
            .route_layer(axum_mw::from_fn_with_state(state, rate_limit_layer))
    }

    fn request(ip: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .uri("/api/v1/test")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .expect("valid request")
    }

    fn bearer_request(ip: &str, token: &str) -> HttpRequest<Body> {
        HttpRequest::builder()
            .uri("/api/v1/test")
            .header("x-forwarded-for", ip)
            .header(AUTHORIZATION, format!("Bearer {token}"))
            .body(Body::empty())
            .expect("valid request")
    }

    fn token(secret: &str, requests_per_minute: u32) -> String {
        let jwt_config = JwtConfig::new(secret);
        let claims: Claims = build_access_claims(
            &jwt_config,
            Uuid::now_v7(),
            "demo@example.com",
            "Demo User",
            vec!["member".to_string()],
            vec![],
            Some(Uuid::now_v7()),
            json!({
                "tenant_quotas": {
                    "requests_per_minute": requests_per_minute,
                }
            }),
            vec!["password".to_string()],
        );

        encode_token(&jwt_config, &claims).expect("token should encode")
    }

    #[tokio::test]
    async fn allows_requests_within_anonymous_burst() {
        let app = test_app(RateLimitState::in_memory(
            JwtConfig::new("test-secret"),
            RateLimitConfig {
                anonymous_requests_per_minute: 2,
                burst_size: 2,
                bucket_ttl_secs: 300,
                redis_key_prefix: default_redis_key_prefix(),
            },
        ));

        let first = app
            .clone()
            .oneshot(request("203.0.113.10"))
            .await
            .expect("first request should succeed");
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first
                .headers()
                .get("x-ratelimit-limit")
                .and_then(|v| v.to_str().ok()),
            Some("2")
        );
        assert_eq!(
            first
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok()),
            Some("1")
        );

        let second = app
            .clone()
            .oneshot(request("203.0.113.10"))
            .await
            .expect("second request should succeed");
        assert_eq!(second.status(), StatusCode::OK);
        assert_eq!(
            second
                .headers()
                .get("x-ratelimit-remaining")
                .and_then(|v| v.to_str().ok()),
            Some("0")
        );
    }

    #[tokio::test]
    async fn rejects_requests_when_burst_is_exhausted() {
        let app = test_app(RateLimitState::in_memory(
            JwtConfig::new("test-secret"),
            RateLimitConfig {
                anonymous_requests_per_minute: 2,
                burst_size: 2,
                bucket_ttl_secs: 300,
                redis_key_prefix: default_redis_key_prefix(),
            },
        ));

        app.clone()
            .oneshot(request("198.51.100.5"))
            .await
            .expect("first request should succeed");
        app.clone()
            .oneshot(request("198.51.100.5"))
            .await
            .expect("second request should succeed");

        let denied = app
            .clone()
            .oneshot(request("198.51.100.5"))
            .await
            .expect("third request should return a response");

        assert_eq!(denied.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            denied
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok()),
            Some("30")
        );
        let body = to_bytes(denied.into_body(), usize::MAX)
            .await
            .expect("rate limit response body should be readable");
        assert_eq!(
            body,
            r#"{"error":{"code":"rate_limit_exceeded","message":"rate limit exceeded"}}"#
        );
    }

    #[tokio::test]
    async fn isolates_anonymous_buckets_by_forwarded_ip() {
        let app = test_app(RateLimitState::in_memory(
            JwtConfig::new("test-secret"),
            RateLimitConfig {
                anonymous_requests_per_minute: 1,
                burst_size: 1,
                bucket_ttl_secs: 300,
                redis_key_prefix: default_redis_key_prefix(),
            },
        ));

        let first_client = app
            .clone()
            .oneshot(request("192.0.2.10"))
            .await
            .expect("first client request should succeed");
        assert_eq!(first_client.status(), StatusCode::OK);

        let second_client = app
            .clone()
            .oneshot(request("192.0.2.11"))
            .await
            .expect("second client request should succeed");
        assert_eq!(second_client.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn honors_tenant_quota_from_jwt_claims() {
        let app = test_app(RateLimitState::in_memory(
            JwtConfig::new("test-secret"),
            RateLimitConfig {
                anonymous_requests_per_minute: 20,
                burst_size: 10,
                bucket_ttl_secs: 300,
                redis_key_prefix: default_redis_key_prefix(),
            },
        ));
        let token = token("test-secret", 1);

        let first = app
            .clone()
            .oneshot(bearer_request("203.0.113.77", &token))
            .await
            .expect("tenant request should succeed");
        assert_eq!(first.status(), StatusCode::OK);
        assert_eq!(
            first
                .headers()
                .get("x-ratelimit-limit")
                .and_then(|v| v.to_str().ok()),
            Some("1")
        );

        let denied = app
            .clone()
            .oneshot(bearer_request("203.0.113.77", &token))
            .await
            .expect("second tenant request should return a response");
        assert_eq!(denied.status(), StatusCode::TOO_MANY_REQUESTS);
    }
}
