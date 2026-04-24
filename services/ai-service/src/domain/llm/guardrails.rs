use crate::models::conversation::{GuardrailFlag, GuardrailVerdict};

const TOXIC_TERMS: &[&str] = &["idiot", "moron", "stupid", "hate", "kill", "attack"];

pub fn evaluate_text(content: &str) -> GuardrailVerdict {
    let mut flags = Vec::new();
    let mut sanitized_tokens = Vec::new();

    for token in content.split_whitespace() {
        let lowered = token.to_lowercase();
        let numeric_count = token
            .chars()
            .filter(|character| character.is_ascii_digit())
            .count();

        if looks_like_email(token) {
            flags.push(GuardrailFlag {
                kind: "pii_email".to_string(),
                severity: "medium".to_string(),
                excerpt: token.to_string(),
            });
            sanitized_tokens.push("[redacted-email]".to_string());
            continue;
        }

        if numeric_count >= 9 {
            flags.push(GuardrailFlag {
                kind: "pii_phone".to_string(),
                severity: "medium".to_string(),
                excerpt: token.to_string(),
            });
            sanitized_tokens.push("[redacted-number]".to_string());
            continue;
        }

        if lowered.contains("ignore") && lowered.contains("instructions") {
            flags.push(GuardrailFlag {
                kind: "prompt_injection".to_string(),
                severity: "high".to_string(),
                excerpt: token.to_string(),
            });
        }

        if TOXIC_TERMS.iter().any(|term| lowered.contains(term)) {
            flags.push(GuardrailFlag {
                kind: "toxicity".to_string(),
                severity: "high".to_string(),
                excerpt: token.to_string(),
            });
        }

        sanitized_tokens.push(token.to_string());
    }

    let blocked = flags.iter().any(|flag| {
        flag.severity == "high" && flag.kind != "pii_email" && flag.kind != "pii_phone"
    });
    let status = if blocked {
        "blocked"
    } else if flags.is_empty() {
        "passed"
    } else {
        "redacted"
    };

    GuardrailVerdict {
        status: status.to_string(),
        redacted_text: sanitized_tokens.join(" "),
        blocked,
        flags,
    }
}

fn looks_like_email(token: &str) -> bool {
    token.contains('@')
        && token.split('@').count() == 2
        && token
            .split('@')
            .last()
            .map(|value| value.contains('.'))
            .unwrap_or(false)
}
