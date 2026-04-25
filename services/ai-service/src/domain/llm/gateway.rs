use std::cmp::Ordering;

use uuid::Uuid;

use crate::models::provider::LlmProvider;

pub fn route_providers(
    providers: &[LlmProvider],
    preferred_provider_id: Option<Uuid>,
    use_case: &str,
) -> Vec<LlmProvider> {
    let mut candidates = providers
        .iter()
        .filter(|provider| {
            provider.enabled
                && (provider.route_rules.use_cases.is_empty()
                    || provider
                        .route_rules
                        .use_cases
                        .iter()
                        .any(|candidate| candidate == use_case || candidate == "general"))
        })
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        provider_rank(right)
            .partial_cmp(&provider_rank(left))
            .unwrap_or(Ordering::Equal)
            .then_with(|| right.load_balance_weight.cmp(&left.load_balance_weight))
    });

    if let Some(preferred_provider_id) = preferred_provider_id {
        if let Some(position) = candidates
            .iter()
            .position(|provider| provider.id == preferred_provider_id)
        {
            let provider = candidates.remove(position);
            candidates.insert(0, provider.clone());

            let mut ordered_fallbacks = Vec::new();
            for fallback_id in &provider.route_rules.fallback_provider_ids {
                if let Some(index) = candidates
                    .iter()
                    .position(|candidate| candidate.id == *fallback_id)
                {
                    ordered_fallbacks.push(candidates.remove(index));
                }
            }

            for fallback in ordered_fallbacks.into_iter().rev() {
                candidates.insert(1, fallback);
            }
        }
    }

    candidates
}

pub fn select_provider(candidates: &[LlmProvider], fallback_enabled: bool) -> Option<LlmProvider> {
    if fallback_enabled {
        candidates
            .iter()
            .find(|provider| provider.health_state.status != "offline")
            .cloned()
            .or_else(|| candidates.first().cloned())
    } else {
        candidates.first().cloned()
    }
}

pub fn estimate_tokens(content: &str) -> i32 {
    ((content.split_whitespace().count() as f32) * 1.35).ceil() as i32
}

fn provider_rank(provider: &LlmProvider) -> f32 {
    let health_bonus = match provider.health_state.status.as_str() {
        "healthy" => 100.0,
        "degraded" => 50.0,
        _ => 0.0,
    };

    health_bonus + provider.load_balance_weight as f32 - (provider.health_state.error_rate * 100.0)
}
