use chrono::{Duration, Utc};

use crate::models::{sink::CepMatch, topology::CepDefinition};

#[allow(dead_code)]
pub fn simulate_cep_matches(cep_definition: Option<&CepDefinition>) -> Vec<CepMatch> {
    let Some(definition) = cep_definition else {
        return Vec::new();
    };

    let now = Utc::now();
    vec![
        CepMatch {
            pattern_name: definition.pattern_name.clone(),
            matched_sequence: definition.sequence.clone(),
            confidence: 0.92,
            detected_at: now - Duration::seconds(9),
        },
        CepMatch {
            pattern_name: definition.pattern_name.clone(),
            matched_sequence: definition.sequence.clone(),
            confidence: 0.88,
            detected_at: now - Duration::seconds(2),
        },
    ]
}
