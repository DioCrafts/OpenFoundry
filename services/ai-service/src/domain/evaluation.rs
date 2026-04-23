use crate::models::conversation::GuardrailVerdict;

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
