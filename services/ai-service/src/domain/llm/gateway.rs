use std::cmp::Ordering;

use uuid::Uuid;

use crate::models::provider::LlmProvider;

pub fn route_providers(
    providers: &[LlmProvider],
    preferred_provider_id: Option<Uuid>,
    use_case: &str,
    required_modalities: &[String],
    require_private_network: bool,
    prefer_private_network: bool,
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
                && supports_required_modalities(provider, required_modalities)
                && (!require_private_network || provider_uses_private_network(provider))
        })
        .cloned()
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        provider_rank(right, prefer_private_network)
            .partial_cmp(&provider_rank(left, prefer_private_network))
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

pub fn provider_uses_private_network(provider: &LlmProvider) -> bool {
    matches!(
        provider.route_rules.network_scope.to_lowercase().as_str(),
        "private" | "hybrid" | "local"
    )
}

fn supports_required_modalities(provider: &LlmProvider, required_modalities: &[String]) -> bool {
    required_modalities.iter().all(|required| {
        provider
            .route_rules
            .supported_modalities
            .iter()
            .any(|supported| supported.eq_ignore_ascii_case(required))
    })
}

fn provider_rank(provider: &LlmProvider, prefer_private_network: bool) -> f32 {
    let health_bonus = match provider.health_state.status.as_str() {
        "healthy" => 100.0,
        "degraded" => 50.0,
        _ => 0.0,
    };

    let private_bonus = if prefer_private_network && provider_uses_private_network(provider) {
        35.0
    } else {
        0.0
    };
    let multimodal_bonus = if provider
        .route_rules
        .supported_modalities
        .iter()
        .any(|modality| modality.eq_ignore_ascii_case("image"))
    {
        5.0
    } else {
        0.0
    };

    health_bonus + provider.load_balance_weight as f32 + private_bonus + multimodal_bonus
        - (provider.health_state.error_rate * 100.0)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use uuid::Uuid;

    use crate::models::provider::{LlmProvider, ProviderHealthState, ProviderRoutingRules};

    use super::{provider_uses_private_network, route_providers};

    fn provider(name: &str, network_scope: &str, modalities: &[&str], weight: i32) -> LlmProvider {
        LlmProvider {
            id: Uuid::now_v7(),
            name: name.to_string(),
            provider_type: "openai".to_string(),
            model_name: "model".to_string(),
            endpoint_url: "https://example.com".to_string(),
            api_mode: "chat_completions".to_string(),
            credential_reference: None,
            credential_configured: false,
            enabled: true,
            load_balance_weight: weight,
            max_output_tokens: 1024,
            cost_tier: "standard".to_string(),
            tags: vec![],
            route_rules: ProviderRoutingRules {
                use_cases: vec!["chat".to_string()],
                network_scope: network_scope.to_string(),
                supported_modalities: modalities.iter().map(|entry| entry.to_string()).collect(),
                ..ProviderRoutingRules::default()
            },
            health_state: ProviderHealthState::default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn filters_on_private_network_requirement() {
        let providers = vec![
            provider("Public", "public", &["text"], 100),
            provider("Local", "local", &["text"], 40),
        ];

        let routed = route_providers(&providers, None, "chat", &["text".to_string()], true, true);

        assert_eq!(routed.len(), 1);
        assert!(provider_uses_private_network(&routed[0]));
    }

    #[test]
    fn filters_on_required_modalities() {
        let providers = vec![
            provider("Text", "public", &["text"], 100),
            provider("Vision", "public", &["text", "image"], 20),
        ];

        let routed = route_providers(
            &providers,
            None,
            "chat",
            &["text".to_string(), "image".to_string()],
            false,
            false,
        );

        assert_eq!(routed.len(), 1);
        assert_eq!(routed[0].name, "Vision");
    }
}
