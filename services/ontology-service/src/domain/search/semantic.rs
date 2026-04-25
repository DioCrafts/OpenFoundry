pub fn embed_text(content: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; 16];
    let vector_len = vector.len();

    for (index, token) in content
        .to_lowercase()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .enumerate()
    {
        let token_value = token
            .bytes()
            .fold(0u32, |accumulator, byte| accumulator.wrapping_add(byte as u32));
        vector[index % vector_len] += (token_value % 997) as f32 / 997.0;
    }

    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in &mut vector {
            *value /= magnitude;
        }
    }

    vector
}

pub fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    if left.len() != right.len() || left.is_empty() {
        return 0.0;
    }

    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum::<f32>()
        .clamp(-1.0, 1.0)
}

pub fn score(query: &str, text: &str) -> f32 {
    let query_embedding = embed_text(query);
    let text_embedding = embed_text(text);
    cosine_similarity(&query_embedding, &text_embedding)
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn similar_text_scores_higher_than_unrelated_text() {
        let related = score("payment risk review", "payment risk review workflow");
        let unrelated = score("payment risk review", "mountain weather and sailing");
        assert!(related > unrelated);
    }
}
