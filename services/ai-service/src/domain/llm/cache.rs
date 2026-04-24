pub fn normalize_text(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character.is_whitespace() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn fingerprint(input: &str) -> Vec<f32> {
    let normalized = normalize_text(input);
    let mut vector = vec![0.0f32; 16];
    let vector_len = vector.len();

    for (index, byte) in normalized.bytes().enumerate() {
        vector[index % vector_len] += byte as f32 / 255.0;
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
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let length = left.len().min(right.len());
    let mut dot_product = 0.0;
    let mut left_magnitude = 0.0;
    let mut right_magnitude = 0.0;

    for index in 0..length {
        dot_product += left[index] * right[index];
        left_magnitude += left[index] * left[index];
        right_magnitude += right[index] * right[index];
    }

    if left_magnitude == 0.0 || right_magnitude == 0.0 {
        return 0.0;
    }

    dot_product / (left_magnitude.sqrt() * right_magnitude.sqrt())
}

pub fn cache_key(kind: &str, input: &str) -> String {
    format!("{kind}:{}", normalize_text(input))
}
