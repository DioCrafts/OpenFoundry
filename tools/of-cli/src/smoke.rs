use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use auth_middleware::jwt::{JwtConfig, build_access_claims, encode_token};
use regex::Regex;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct SmokeSuite {
    base_url: String,
    #[serde(default)]
    default_headers: BTreeMap<String, String>,
    #[serde(default)]
    variables: BTreeMap<String, Value>,
    #[serde(default)]
    auth: Option<SmokeAuth>,
    steps: Vec<SmokeStep>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum SmokeAuth {
    DevJwt {
        secret_env: String,
        #[serde(default = "default_smoke_email")]
        email: String,
        #[serde(default = "default_smoke_name")]
        name: String,
        #[serde(default)]
        roles: Vec<String>,
        #[serde(default)]
        permissions: Vec<String>,
        #[serde(default)]
        org_id: Option<Uuid>,
        #[serde(default)]
        attributes: Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
struct SmokeStep {
    name: String,
    method: String,
    path: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: Option<Value>,
    expected_status: u16,
    #[serde(default)]
    retry_attempts: Option<usize>,
    #[serde(default)]
    retry_delay_ms: Option<u64>,
    #[serde(default)]
    capture: BTreeMap<String, String>,
    #[serde(default)]
    expect: Vec<SmokeExpectation>,
}

#[derive(Debug, Clone, Deserialize)]
struct SmokeExpectation {
    path: String,
    #[serde(default)]
    equals: Option<Value>,
    #[serde(default)]
    exists: bool,
}

#[derive(Debug, Serialize)]
struct SmokeReport {
    base_url: String,
    started_at_epoch_ms: u128,
    success: bool,
    failure_message: Option<String>,
    steps: Vec<SmokeStepReport>,
}

#[derive(Debug, Clone, Serialize)]
struct SmokeStepReport {
    name: String,
    method: String,
    path: String,
    url: String,
    expected_status: u16,
    actual_status: Option<u16>,
    assertions: Vec<SmokeAssertionReport>,
    captured: BTreeMap<String, Value>,
    response_preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct SmokeAssertionReport {
    path: String,
    status: String,
    expected: Option<Value>,
    actual: Option<Value>,
}

#[derive(Debug, Default)]
struct ExecutionContext {
    values: BTreeMap<String, Value>,
}

impl ExecutionContext {
    fn new(seed: BTreeMap<String, Value>) -> Result<Self> {
        let started_at = epoch_ms()?;
        let mut values = BTreeMap::from([
            (
                "RUN_ID".to_string(),
                Value::String(Uuid::now_v7().to_string()),
            ),
            (
                "STARTED_AT_EPOCH_MS".to_string(),
                Value::Number(serde_json::Number::from(started_at as u64)),
            ),
        ]);
        for (key, value) in seed {
            values.insert(key, value);
        }
        Ok(Self { values })
    }

    fn get(&self, key: &str) -> Option<&Value> {
        self.values.get(key)
    }

    fn set(&mut self, key: impl Into<String>, value: Value) {
        self.values.insert(key.into(), value);
    }

    fn resolve_string(&self, raw: &str) -> Result<String> {
        let pattern = Regex::new(r"\$\{([A-Za-z0-9_]+)\}")?;
        let mut missing = Vec::new();
        let resolved = pattern.replace_all(raw, |captures: &regex::Captures<'_>| {
            let key = captures
                .get(1)
                .map(|value| value.as_str())
                .unwrap_or_default();
            match self.get(key) {
                Some(value) => stringify_value(value),
                None => {
                    missing.push(key.to_string());
                    captures
                        .get(0)
                        .map(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string()
                }
            }
        });

        if !missing.is_empty() {
            missing.sort();
            missing.dedup();
            bail!("missing smoke variables: {}", missing.join(", "));
        }

        Ok(resolved.into_owned())
    }

    fn resolve_json(&self, value: &Value) -> Result<Value> {
        match value {
            Value::String(raw) => {
                if let Some(key) = exact_template_key(raw) {
                    return self
                        .get(key)
                        .cloned()
                        .with_context(|| format!("missing smoke variable: {key}"));
                }
                Ok(Value::String(self.resolve_string(raw)?))
            }
            Value::Array(items) => items
                .iter()
                .map(|item| self.resolve_json(item))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array),
            Value::Object(map) => {
                let mut resolved = serde_json::Map::new();
                for (key, item) in map {
                    resolved.insert(key.clone(), self.resolve_json(item)?);
                }
                Ok(Value::Object(resolved))
            }
            other => Ok(other.clone()),
        }
    }
}

pub async fn run_suite(scenario_path: &Path, output_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(scenario_path)
        .with_context(|| format!("failed to read smoke scenario {}", scenario_path.display()))?;
    let resolved = resolve_env_templates(&raw)?;
    let suite: SmokeSuite = serde_json::from_str(&resolved)
        .with_context(|| format!("failed to parse smoke scenario {}", scenario_path.display()))?;
    let SmokeSuite {
        base_url,
        default_headers,
        variables,
        auth,
        steps: suite_steps,
    } = suite;

    let started_at_epoch_ms = epoch_ms()?;
    let mut context = ExecutionContext::new(variables)?;
    let auth_header = auth
        .as_ref()
        .map(SmokeAuth::authorization_header)
        .transpose()?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("failed to build HTTP client")?;

    let base_url = base_url.trim_end_matches('/').to_string();
    let mut steps = Vec::new();
    let mut failure_message = None;

    for step in suite_steps {
        let path = match context.resolve_string(&step.path) {
            Ok(path) => path,
            Err(error) => {
                failure_message = Some(format!(
                    "step '{}' failed before request: {error}",
                    step.name
                ));
                steps.push(SmokeStepReport {
                    name: step.name,
                    method: step.method,
                    path: step.path,
                    url: String::new(),
                    expected_status: step.expected_status,
                    actual_status: None,
                    assertions: Vec::new(),
                    captured: BTreeMap::new(),
                    response_preview: None,
                });
                break;
            }
        };
        let url = format!("{base_url}/{}", path.trim_start_matches('/'));

        match execute_step(
            &client,
            &context,
            &default_headers,
            auth_header.as_deref(),
            &url,
            step,
        )
        .await
        {
            Ok(outcome) => {
                for (key, value) in &outcome.captured {
                    context.set(key.clone(), value.clone());
                }
                steps.push(outcome.report);
            }
            Err((report, error)) => {
                failure_message = Some(error);
                steps.push(report);
                break;
            }
        }
    }

    let report = SmokeReport {
        base_url,
        started_at_epoch_ms,
        success: failure_message.is_none(),
        failure_message: failure_message.clone(),
        steps,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    println!("smoke report written to {}", output_path.display());

    if let Some(message) = failure_message {
        bail!(message);
    }

    Ok(())
}

struct StepOutcome {
    report: SmokeStepReport,
    captured: BTreeMap<String, Value>,
}

async fn execute_step(
    client: &Client,
    context: &ExecutionContext,
    default_headers: &BTreeMap<String, String>,
    auth_header: Option<&str>,
    url: &str,
    step: SmokeStep,
) -> std::result::Result<StepOutcome, (SmokeStepReport, String)> {
    let attempts = step.retry_attempts.unwrap_or(1).max(1);
    let retry_delay = step.retry_delay_ms.unwrap_or(250);
    let mut last_error = None;

    for attempt in 1..=attempts {
        match execute_step_once(
            client,
            context,
            default_headers,
            auth_header,
            url,
            step.clone(),
        )
        .await
        {
            Ok(outcome) => return Ok(outcome),
            Err((report, error)) => {
                last_error = Some((report, error, attempt));
                if attempt < attempts {
                    tokio::time::sleep(std::time::Duration::from_millis(retry_delay)).await;
                }
            }
        }
    }

    let (report, error, attempt) = last_error.expect("smoke step should execute at least once");
    Err((
        report,
        format!(
            "{error} after {attempt} attempt{}",
            if attempt == 1 { "" } else { "s" }
        ),
    ))
}

async fn execute_step_once(
    client: &Client,
    context: &ExecutionContext,
    default_headers: &BTreeMap<String, String>,
    auth_header: Option<&str>,
    url: &str,
    step: SmokeStep,
) -> std::result::Result<StepOutcome, (SmokeStepReport, String)> {
    let method = Method::from_bytes(step.method.as_bytes()).map_err(|error| {
        let report = SmokeStepReport {
            name: step.name.clone(),
            method: step.method.clone(),
            path: step.path.clone(),
            url: url.to_string(),
            expected_status: step.expected_status,
            actual_status: None,
            assertions: Vec::new(),
            captured: BTreeMap::new(),
            response_preview: None,
        };
        (
            report,
            format!("step '{}' has invalid HTTP method: {error}", step.name),
        )
    })?;

    let mut request = client.request(method, url);
    let mut has_authorization = false;

    for (name, value) in default_headers {
        if name.eq_ignore_ascii_case("authorization") {
            has_authorization = true;
        }
        let value = match context.resolve_string(value) {
            Ok(value) => value,
            Err(error) => {
                let report = SmokeStepReport {
                    name: step.name.clone(),
                    method: step.method.clone(),
                    path: step.path.clone(),
                    url: url.to_string(),
                    expected_status: step.expected_status,
                    actual_status: None,
                    assertions: Vec::new(),
                    captured: BTreeMap::new(),
                    response_preview: None,
                };
                return Err((
                    report,
                    format!(
                        "step '{}' failed to resolve default header '{name}': {error}",
                        step.name
                    ),
                ));
            }
        };
        request = request.header(name, value);
    }

    for (name, value) in &step.headers {
        if name.eq_ignore_ascii_case("authorization") {
            has_authorization = true;
        }
        let value = match context.resolve_string(value) {
            Ok(value) => value,
            Err(error) => {
                let report = SmokeStepReport {
                    name: step.name.clone(),
                    method: step.method.clone(),
                    path: step.path.clone(),
                    url: url.to_string(),
                    expected_status: step.expected_status,
                    actual_status: None,
                    assertions: Vec::new(),
                    captured: BTreeMap::new(),
                    response_preview: None,
                };
                return Err((
                    report,
                    format!(
                        "step '{}' failed to resolve header '{name}': {error}",
                        step.name
                    ),
                ));
            }
        };
        request = request.header(name, value);
    }

    if !has_authorization {
        if let Some(auth_header) = auth_header {
            request = request.header("authorization", auth_header);
        }
    }

    let body = match step
        .body
        .as_ref()
        .map(|value| context.resolve_json(value))
        .transpose()
    {
        Ok(body) => body,
        Err(error) => {
            let report = SmokeStepReport {
                name: step.name.clone(),
                method: step.method.clone(),
                path: step.path.clone(),
                url: url.to_string(),
                expected_status: step.expected_status,
                actual_status: None,
                assertions: Vec::new(),
                captured: BTreeMap::new(),
                response_preview: None,
            };
            return Err((
                report,
                format!(
                    "step '{}' failed to resolve request body: {error}",
                    step.name
                ),
            ));
        }
    };
    if let Some(body) = body {
        request = request.json(&body);
    }

    let response = request.send().await.map_err(|error| {
        let report = SmokeStepReport {
            name: step.name.clone(),
            method: step.method.clone(),
            path: step.path.clone(),
            url: url.to_string(),
            expected_status: step.expected_status,
            actual_status: None,
            assertions: Vec::new(),
            captured: BTreeMap::new(),
            response_preview: None,
        };
        (
            report,
            format!("step '{}' request failed: {error}", step.name),
        )
    })?;

    let status = response.status().as_u16();
    let response_text = response.text().await.unwrap_or_default();
    let response_json = serde_json::from_str::<Value>(&response_text).ok();
    let response_preview = (!response_text.is_empty()).then(|| truncate_preview(&response_text));

    let mut report = SmokeStepReport {
        name: step.name.clone(),
        method: step.method.clone(),
        path: step.path.clone(),
        url: url.to_string(),
        expected_status: step.expected_status,
        actual_status: Some(status),
        assertions: Vec::new(),
        captured: BTreeMap::new(),
        response_preview,
    };

    if status != step.expected_status {
        return Err((
            report,
            format!(
                "step '{}' returned status {} but expected {}",
                step.name, status, step.expected_status
            ),
        ));
    }

    let Some(response_json) = response_json.as_ref() else {
        if !step.expect.is_empty() || !step.capture.is_empty() {
            return Err((
                report,
                format!("step '{}' expected a JSON response body", step.name),
            ));
        }

        return Ok(StepOutcome {
            report,
            captured: BTreeMap::new(),
        });
    };

    match evaluate_expectations(context, response_json, &step.expect) {
        Ok(assertions) => report.assertions = assertions,
        Err((assertions, error)) => {
            report.assertions = assertions;
            return Err((
                report,
                format!("step '{}' assertion failed: {error}", step.name),
            ));
        }
    }

    let mut captured = BTreeMap::new();
    for (name, pointer) in &step.capture {
        let value = response_json
            .pointer(pointer)
            .cloned()
            .with_context(|| format!("response pointer '{pointer}' not found"))
            .map_err(|error| {
                (
                    report.clone(),
                    format!("step '{}' capture failed: {error}", step.name),
                )
            })?;
        captured.insert(name.clone(), value);
    }
    report.captured = captured.clone();

    Ok(StepOutcome { report, captured })
}

fn evaluate_expectations(
    context: &ExecutionContext,
    response_json: &Value,
    expectations: &[SmokeExpectation],
) -> std::result::Result<Vec<SmokeAssertionReport>, (Vec<SmokeAssertionReport>, String)> {
    let mut reports = Vec::new();
    for expectation in expectations {
        let actual = response_json.pointer(&expectation.path).cloned();
        let exists = actual.is_some();
        let expected = expectation
            .equals
            .as_ref()
            .map(|value| context.resolve_json(value))
            .transpose()
            .map_err(|error| {
                (
                    reports.clone(),
                    format!(
                        "failed to resolve expectation '{}': {error}",
                        expectation.path
                    ),
                )
            })?;

        if expectation.exists && !exists {
            reports.push(SmokeAssertionReport {
                path: expectation.path.clone(),
                status: "failed".to_string(),
                expected,
                actual,
            });
            return Err((
                reports,
                format!("response pointer '{}' not found", expectation.path),
            ));
        }

        if let Some(expected) = expected {
            if actual.as_ref() != Some(&expected) {
                reports.push(SmokeAssertionReport {
                    path: expectation.path.clone(),
                    status: "failed".to_string(),
                    expected: Some(expected),
                    actual,
                });
                return Err((
                    reports,
                    format!(
                        "pointer '{}' did not match expected value",
                        expectation.path
                    ),
                ));
            }

            reports.push(SmokeAssertionReport {
                path: expectation.path.clone(),
                status: "passed".to_string(),
                expected: Some(expected),
                actual,
            });
            continue;
        }

        reports.push(SmokeAssertionReport {
            path: expectation.path.clone(),
            status: if exists { "passed" } else { "skipped" }.to_string(),
            expected: None,
            actual,
        });
    }

    Ok(reports)
}

impl SmokeAuth {
    fn authorization_header(&self) -> Result<String> {
        match self {
            Self::DevJwt {
                secret_env,
                email,
                name,
                roles,
                permissions,
                org_id,
                attributes,
            } => {
                let secret = std::env::var(secret_env).with_context(|| {
                    format!("missing environment variable {secret_env} for smoke auth")
                })?;
                let jwt_config = JwtConfig::new(&secret);
                let claims = build_access_claims(
                    &jwt_config,
                    Uuid::now_v7(),
                    email,
                    name,
                    if roles.is_empty() {
                        vec!["admin".to_string()]
                    } else {
                        roles.clone()
                    },
                    if permissions.is_empty() {
                        vec!["*:*".to_string()]
                    } else {
                        permissions.clone()
                    },
                    *org_id,
                    if attributes.is_null() {
                        serde_json::json!({})
                    } else {
                        attributes.clone()
                    },
                    vec!["smoke".to_string()],
                );
                let token = encode_token(&jwt_config, &claims)?;
                Ok(format!("Bearer {token}"))
            }
        }
    }
}

fn resolve_env_templates(raw: &str) -> Result<String> {
    let pattern = Regex::new(r"\$\{([A-Z0-9_]+)\}")?;
    let resolved = pattern.replace_all(raw, |captures: &regex::Captures<'_>| {
        let key = captures
            .get(1)
            .map(|value| value.as_str())
            .unwrap_or_default();
        match std::env::var(key) {
            Ok(value) => value,
            Err(_) => captures
                .get(0)
                .map(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
        }
    });

    Ok(resolved.into_owned())
}

fn exact_template_key(raw: &str) -> Option<&str> {
    let key = raw.strip_prefix("${")?.strip_suffix('}')?;
    if !key.is_empty()
        && key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        Some(key)
    } else {
        None
    }
}

fn stringify_value(value: &Value) -> String {
    match value {
        Value::String(inner) => inner.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(inner) => inner.to_string(),
        Value::Number(inner) => inner.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn truncate_preview(raw: &str) -> String {
    const LIMIT: usize = 400;
    if raw.chars().count() <= LIMIT {
        raw.to_string()
    } else {
        format!("{}...", raw.chars().take(LIMIT).collect::<String>())
    }
}

fn epoch_ms() -> Result<u128> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_millis())
}

fn default_smoke_email() -> String {
    "smoke@example.com".to_string()
}

fn default_smoke_name() -> String {
    "OpenFoundry Smoke Runner".to_string()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{ExecutionContext, SmokeExpectation, evaluate_expectations, exact_template_key};

    #[test]
    fn resolves_json_templates_and_preserves_non_string_types() {
        let context = ExecutionContext::new(std::collections::BTreeMap::from([
            ("dataset_id".to_string(), json!("123")),
            ("workers".to_string(), json!(3)),
            ("label".to_string(), json!("smoke")),
        ]))
        .expect("context");

        let resolved = context
            .resolve_json(&json!({
                "id": "${dataset_id}",
                "workers": "${workers}",
                "name": "run-${label}",
            }))
            .expect("resolved body");

        assert_eq!(
            resolved,
            json!({
                "id": "123",
                "workers": 3,
                "name": "run-smoke",
            })
        );
    }

    #[test]
    fn evaluates_expectations_against_json_pointers() {
        let context = ExecutionContext::new(Default::default()).expect("context");
        let expectations = vec![
            SmokeExpectation {
                path: "/status".to_string(),
                equals: Some(json!("completed")),
                exists: false,
            },
            SmokeExpectation {
                path: "/items/0/id".to_string(),
                equals: None,
                exists: true,
            },
        ];

        let reports = evaluate_expectations(
            &context,
            &json!({
                "status": "completed",
                "items": [{"id": "abc"}],
            }),
            &expectations,
        )
        .expect("expectations pass");

        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|report| report.status == "passed"));
    }

    #[test]
    fn detects_exact_template_keys() {
        assert_eq!(exact_template_key("${RUN_ID}"), Some("RUN_ID"));
        assert_eq!(exact_template_key("prefix-${RUN_ID}"), None);
    }
}
