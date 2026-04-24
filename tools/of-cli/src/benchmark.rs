use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct ScenarioSuite {
    base_url: String,
    #[serde(default)]
    default_headers: BTreeMap<String, String>,
    #[serde(default = "default_warmup_iterations")]
    warmup_iterations: usize,
    #[serde(default = "default_measure_iterations")]
    measure_iterations: usize,
    scenarios: Vec<ScenarioDefinition>,
}

#[derive(Debug, Deserialize)]
struct ScenarioDefinition {
    name: String,
    method: String,
    path: String,
    #[serde(default)]
    headers: BTreeMap<String, String>,
    #[serde(default)]
    body: Option<Value>,
    expected_status: u16,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct SuiteReport {
    base_url: String,
    started_at_epoch_ms: u128,
    warmup_iterations: usize,
    measure_iterations: usize,
    scenarios: Vec<ScenarioReport>,
}

#[derive(Debug, Serialize)]
struct ScenarioReport {
    name: String,
    method: String,
    path: String,
    expected_status: u16,
    statuses: Vec<u16>,
    iterations: usize,
    mean_latency_ms: f64,
    p95_latency_ms: f64,
    fastest_latency_ms: f64,
    slowest_latency_ms: f64,
    tags: Vec<String>,
}

pub async fn run_suite(scenario_path: &Path, output_path: &Path) -> Result<()> {
    let raw = fs::read_to_string(scenario_path).with_context(|| {
        format!(
            "failed to read benchmark scenario {}",
            scenario_path.display()
        )
    })?;
    let resolved = resolve_env_templates(&raw)?;
    let suite: ScenarioSuite = serde_json::from_str(&resolved).with_context(|| {
        format!(
            "failed to parse benchmark scenario {}",
            scenario_path.display()
        )
    })?;

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .context("failed to build HTTP client")?;

    let mut reports = Vec::new();
    let mut mismatches = Vec::new();
    for scenario in &suite.scenarios {
        for _ in 0..suite.warmup_iterations {
            let _ = execute_request(&client, &suite, scenario).await;
        }

        let mut latencies = Vec::new();
        let mut statuses = Vec::new();
        for _ in 0..suite.measure_iterations {
            let started = Instant::now();
            let status = execute_request(&client, &suite, scenario).await?;
            latencies.push(started.elapsed().as_secs_f64() * 1000.0);
            statuses.push(status);
        }

        if statuses
            .iter()
            .any(|status| *status != scenario.expected_status)
        {
            mismatches.push(format!(
                "{} returned statuses {:?} but expected {}",
                scenario.name, statuses, scenario.expected_status,
            ));
        }

        reports.push(ScenarioReport {
            name: scenario.name.clone(),
            method: scenario.method.clone(),
            path: scenario.path.clone(),
            expected_status: scenario.expected_status,
            statuses,
            iterations: latencies.len(),
            mean_latency_ms: mean(&latencies),
            p95_latency_ms: percentile(&latencies, 0.95),
            fastest_latency_ms: latencies.iter().copied().fold(f64::MAX, f64::min),
            slowest_latency_ms: latencies.iter().copied().fold(0.0, f64::max),
            tags: scenario.tags.clone(),
        });
    }

    let report = SuiteReport {
        base_url: suite.base_url,
        started_at_epoch_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock before unix epoch")?
            .as_millis(),
        warmup_iterations: suite.warmup_iterations,
        measure_iterations: suite.measure_iterations,
        scenarios: reports,
    };

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    println!("benchmark report written to {}", output_path.display());

    if !mismatches.is_empty() {
        bail!(mismatches.join("; "));
    }

    Ok(())
}

async fn execute_request(
    client: &Client,
    suite: &ScenarioSuite,
    scenario: &ScenarioDefinition,
) -> Result<u16> {
    let method = Method::from_bytes(scenario.method.as_bytes())
        .with_context(|| format!("invalid HTTP method {}", scenario.method))?;
    let base_url = suite.base_url.trim_end_matches('/');
    let path = scenario.path.trim_start_matches('/');
    let url = format!("{base_url}/{path}");

    let mut request = client.request(method, url);
    for (name, value) in &suite.default_headers {
        request = request.header(name, value);
    }
    for (name, value) in &scenario.headers {
        request = request.header(name, value);
    }
    if let Some(body) = &scenario.body {
        request = request.json(body);
    }

    let response = request
        .send()
        .await
        .with_context(|| format!("request failed for benchmark scenario {}", scenario.name))?;
    Ok(response.status().as_u16())
}

fn resolve_env_templates(raw: &str) -> Result<String> {
    let pattern = Regex::new(r"\$\{([A-Z0-9_]+)\}")?;
    let mut missing = Vec::new();
    let resolved = pattern.replace_all(raw, |captures: &regex::Captures<'_>| {
        let key = captures
            .get(1)
            .map(|value| value.as_str())
            .unwrap_or_default();
        match std::env::var(key) {
            Ok(value) => value,
            Err(_) => {
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
        bail!(
            "missing environment variables for benchmark scenario: {}",
            missing.join(", ")
        );
    }

    Ok(resolved.into_owned())
}

fn mean(samples: &[f64]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.iter().sum::<f64>() / samples.len() as f64
}

fn percentile(samples: &[f64], percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let mut ordered = samples.to_vec();
    ordered.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let index = ((ordered.len() - 1) as f64 * percentile).round() as usize;
    ordered[index]
}

fn default_warmup_iterations() -> usize {
    1
}
fn default_measure_iterations() -> usize {
    5
}
