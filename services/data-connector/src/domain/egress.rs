use reqwest::Url;
use serde_json::Value;

use crate::AppState;

#[derive(Debug, Clone)]
pub struct EgressPolicy {
    pub allowed_hosts: Vec<String>,
    pub blocked_hosts: Vec<String>,
    pub allow_private_networks: bool,
    pub allow_insecure_http: bool,
}

impl EgressPolicy {
    pub fn from_state_and_config(state: &AppState, config: &Value) -> Self {
        let egress = config.get("egress");
        Self {
            allowed_hosts: string_list(egress.and_then(|value| value.get("allowed_hosts")))
                .or_else(|| {
                    (!state.allowed_egress_hosts.is_empty())
                        .then(|| state.allowed_egress_hosts.clone())
                })
                .unwrap_or_default(),
            blocked_hosts: string_list(egress.and_then(|value| value.get("blocked_hosts")))
                .unwrap_or_default(),
            allow_private_networks: egress
                .and_then(|value| value.get("allow_private_networks"))
                .and_then(Value::as_bool)
                .unwrap_or(state.allow_private_network_egress),
            allow_insecure_http: egress
                .and_then(|value| value.get("allow_insecure_http"))
                .and_then(Value::as_bool)
                .unwrap_or(false),
        }
    }
}

pub fn validate_url(url: &Url, policy: &EgressPolicy) -> Result<(), String> {
    let host = url
        .host_str()
        .ok_or_else(|| format!("URL '{}' has no host", url))?
        .to_ascii_lowercase();

    if !policy.allow_insecure_http
        && url.scheme() == "http"
        && !is_localhost(&host)
        && !is_private_ip(&host)
    {
        return Err(format!(
            "egress policy blocks insecure HTTP to host '{host}'"
        ));
    }

    if !policy.allow_private_networks && is_private_ip(&host) && !is_localhost(&host) {
        return Err(format!(
            "egress policy blocks private-network host '{host}'"
        ));
    }

    if host_matches_any(&host, &policy.blocked_hosts) {
        return Err(format!("egress policy blocks host '{host}'"));
    }

    if !policy.allowed_hosts.is_empty() && !host_matches_any(&host, &policy.allowed_hosts) {
        return Err(format!("egress policy does not allow host '{host}'"));
    }

    Ok(())
}

fn string_list(value: Option<&Value>) -> Option<Vec<String>> {
    value.and_then(Value::as_array).map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .map(|item| item.trim().to_ascii_lowercase())
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
    })
}

fn host_matches_any(host: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|pattern| host_matches(host, pattern))
}

fn host_matches(host: &str, pattern: &str) -> bool {
    let normalized = pattern.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    if normalized == "*" {
        return true;
    }
    if let Some(suffix) = normalized.strip_prefix("*.") {
        return host == suffix || host.ends_with(&format!(".{suffix}"));
    }
    host == normalized || host.ends_with(&format!(".{normalized}"))
}

fn is_localhost(host: &str) -> bool {
    matches!(host, "localhost" | "127.0.0.1" | "::1")
}

fn is_private_ip(host: &str) -> bool {
    let Ok(ip) = host.parse::<std::net::IpAddr>() else {
        return false;
    };
    match ip {
        std::net::IpAddr::V4(v4) => {
            let octets = v4.octets();
            octets[0] == 10
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || octets[0] == 127
                || (octets[0] == 169 && octets[1] == 254)
        }
        std::net::IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

#[cfg(test)]
mod tests {
    use reqwest::Url;

    use super::{EgressPolicy, validate_url};

    #[test]
    fn blocks_hosts_outside_allowlist() {
        let policy = EgressPolicy {
            allowed_hosts: vec!["api.example.com".to_string()],
            blocked_hosts: Vec::new(),
            allow_private_networks: false,
            allow_insecure_http: false,
        };
        let url = Url::parse("https://internal.example.net/v1/data").expect("url");
        assert!(validate_url(&url, &policy).is_err());
    }

    #[test]
    fn allows_wildcard_subdomains() {
        let policy = EgressPolicy {
            allowed_hosts: vec!["*.example.com".to_string()],
            blocked_hosts: Vec::new(),
            allow_private_networks: false,
            allow_insecure_http: false,
        };
        let url = Url::parse("https://sales.eu.example.com/odata").expect("url");
        assert!(validate_url(&url, &policy).is_ok());
    }

    #[test]
    fn blocks_private_network_when_disabled() {
        let policy = EgressPolicy {
            allowed_hosts: Vec::new(),
            blocked_hosts: Vec::new(),
            allow_private_networks: false,
            allow_insecure_http: true,
        };
        let url = Url::parse("http://10.0.0.4/feed").expect("url");
        assert!(validate_url(&url, &policy).is_err());
    }
}
