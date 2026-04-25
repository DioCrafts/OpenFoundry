use std::sync::OnceLock;

use regex::Regex;

use crate::models::sensitive_data::{
    SensitiveDataFinding, SensitiveDataScanRequest, SensitiveDataScanResponse,
};

pub fn scan(request: &SensitiveDataScanRequest) -> SensitiveDataScanResponse {
    let detectors = detector_catalog();
    let mut findings = Vec::new();
    let mut redacted_content = request.content.clone();

    for (kind, regex) in detectors {
        let mut match_count = 0;
        let mut first_value = None;
        for capture in regex.find_iter(&request.content) {
            match_count += 1;
            if first_value.is_none() {
                first_value = Some(capture.as_str().to_string());
            }
            if request.redact {
                redacted_content = redacted_content
                    .replace(capture.as_str(), &redact_value(kind, capture.as_str()));
            }
        }

        if let Some(value) = first_value {
            findings.push(SensitiveDataFinding {
                kind: kind.to_string(),
                redacted: redact_value(kind, &value),
                value,
                match_count,
            });
        }
    }

    findings.sort_by(|left, right| left.kind.cmp(&right.kind));

    SensitiveDataScanResponse {
        risk_score: findings
            .iter()
            .map(|finding| score_for_kind(&finding.kind))
            .sum(),
        findings,
        redacted_content,
    }
}

fn detector_catalog() -> &'static [(&'static str, Regex)] {
    static DETECTORS: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    DETECTORS.get_or_init(|| {
        vec![
            (
                "email",
                Regex::new(r"(?i)\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b")
                    .expect("email regex should compile"),
            ),
            (
                "ssn",
                Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("ssn regex should compile"),
            ),
            (
                "credit_card",
                Regex::new(r"\b(?:\d[ -]*?){13,16}\b").expect("card regex should compile"),
            ),
            (
                "api_key",
                Regex::new(r"\bofk_[A-Za-z0-9_-]{8,}\b").expect("api key regex should compile"),
            ),
            (
                "bearer_token",
                Regex::new(r"Bearer\s+[A-Za-z0-9._=-]{16,}").expect("bearer regex should compile"),
            ),
        ]
    })
}

fn redact_value(kind: &str, value: &str) -> String {
    match kind {
        "email" => {
            let parts = value.split('@').collect::<Vec<_>>();
            if parts.len() == 2 {
                format!(
                    "{}***@{}",
                    &parts[0].chars().take(2).collect::<String>(),
                    parts[1]
                )
            } else {
                "[redacted-email]".to_string()
            }
        }
        "ssn" => "***-**-****".to_string(),
        "credit_card" => format!(
            "**** **** **** {}",
            value
                .chars()
                .rev()
                .take(4)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ),
        "api_key" => "ofk_[redacted]".to_string(),
        "bearer_token" => "Bearer [redacted]".to_string(),
        _ => "[redacted]".to_string(),
    }
}

fn score_for_kind(kind: &str) -> u32 {
    match kind {
        "ssn" => 40,
        "credit_card" => 40,
        "api_key" => 35,
        "bearer_token" => 35,
        "email" => 10,
        _ => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_detects_and_redacts_sensitive_content() {
        let response = scan(&SensitiveDataScanRequest {
            content: "Contact jane@example.com with SSN 123-45-6789 and token ofk_abcdefghi"
                .to_string(),
            redact: true,
        });

        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.kind == "email")
        );
        assert!(
            response
                .findings
                .iter()
                .any(|finding| finding.kind == "ssn")
        );
        assert!(response.redacted_content.contains("***-**-****"));
        assert!(response.redacted_content.contains("ofk_[redacted]"));
        assert!(response.risk_score >= 50);
    }
}
