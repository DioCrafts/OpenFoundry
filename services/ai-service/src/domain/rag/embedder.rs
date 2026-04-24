pub fn embed_text(content: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; 12];
    let vector_len = vector.len();

    for (index, token) in content
        .to_lowercase()
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .enumerate()
    {
        let token_value = token.bytes().fold(0u32, |accumulator, byte| {
            accumulator.wrapping_add(byte as u32)
        });
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
