use crate::models::conversation::GuardrailVerdict;
use crate::models::provider::LlmProvider;

pub fn cache_hit_rate(entry_count: i64, total_hits: i64) -> f32 {
    if entry_count <= 0 {
        0.0
    } else {
        (total_hits as f32 / entry_count as f32).min(100.0)
    }
}

pub fn risk_score(verdict: &GuardrailVerdict) -> f32 {
    if verdict.blocked {
        1.0
    } else if verdict.flags.is_empty() {
        0.0
    } else {
        (verdict.flags.len() as f32 / 5.0).min(0.95)
    }
}

pub fn safety_score(verdict: &GuardrailVerdict) -> f32 {
    (1.0 - risk_score(verdict)).clamp(0.0, 1.0)
}

pub fn estimated_cost_usd(
    provider: &LlmProvider,
    prompt_tokens: i32,
    completion_tokens: i32,
    cache_hit: bool,
) -> f32 {
    if cache_hit {
        return 0.0;
    }

    let input_cost = (prompt_tokens.max(0) as f32 / 1_000.0)
        * provider.route_rules.input_cost_per_1k_tokens_usd.max(0.0);
    let output_cost = (completion_tokens.max(0) as f32 / 1_000.0)
        * provider.route_rules.output_cost_per_1k_tokens_usd.max(0.0);

    (input_cost + output_cost).max(0.0)
}

pub fn quality_score(reply: &str, rubric_keywords: &[String]) -> f32 {
    let normalized_reply = reply.to_lowercase();
    if rubric_keywords.is_empty() {
        return ((reply.split_whitespace().count() as f32) / 120.0).clamp(0.35, 0.9);
    }

    let hits = rubric_keywords
        .iter()
        .filter(|keyword| normalized_reply.contains(&keyword.to_lowercase()))
        .count();

    (hits as f32 / rubric_keywords.len() as f32).clamp(0.0, 1.0)
}

pub fn normalized_score(value: f32, min: f32, max: f32, lower_is_better: bool) -> f32 {
    if (max - min).abs() < f32::EPSILON {
        return 1.0;
    }

    let normalized = ((value - min) / (max - min)).clamp(0.0, 1.0);
    if lower_is_better {
        1.0 - normalized
    } else {
        normalized
    }
}

pub fn overall_benchmark_score(quality: f32, safety: f32, latency: f32, cost: f32) -> f32 {
    ((quality * 0.45) + (safety * 0.25) + (latency * 0.15) + (cost * 0.15)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::models::provider::{LlmProvider, ProviderHealthState, ProviderRoutingRules};

    use super::{estimated_cost_usd, normalized_score, overall_benchmark_score, quality_score};

    fn sample_provider() -> LlmProvider {
        LlmProvider {
            id: Uuid::nil(),
            name: "Local".to_string(),
            provider_type: "ollama".to_string(),
            model_name: "llama3".to_string(),
            endpoint_url: "http://localhost:11434/api".to_string(),
            api_mode: "chat".to_string(),
            credential_reference: None,
            credential_configured: false,
            enabled: true,
            load_balance_weight: 10,
            max_output_tokens: 2048,
            cost_tier: "local".to_string(),
            tags: vec![],
            route_rules: ProviderRoutingRules {
                input_cost_per_1k_tokens_usd: 0.002,
                output_cost_per_1k_tokens_usd: 0.004,
                ..ProviderRoutingRules::default()
            },
            health_state: ProviderHealthState::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn estimates_cost_from_provider_rates() {
        let provider = sample_provider();
        let cost = estimated_cost_usd(&provider, 500, 250, false);

        assert!((cost - 0.002).abs() < 0.0001);
    }

    #[test]
    fn normalizes_inverse_scores() {
        let score = normalized_score(900.0, 300.0, 900.0, true);

        assert!(score < 0.05);
    }

    #[test]
    fn scores_quality_from_rubric_keywords() {
        let score = quality_score(
            "This answer covers latency, cost, and fallback policy.",
            &[
                "latency".to_string(),
                "fallback".to_string(),
                "private".to_string(),
            ],
        );

        assert!((score - 0.666).abs() < 0.02);
    }

    #[test]
    fn computes_weighted_benchmark_score() {
        let score = overall_benchmark_score(0.9, 1.0, 0.8, 0.7);

        assert!(score > 0.85);
    }
}
