pub fn chunk_text(content: &str, max_chars: usize) -> Vec<(i32, String)> {
    let mut chunks = Vec::new();
    let mut buffer = String::new();
    let mut position = 0;

    for paragraph in content.split("\n\n") {
        let trimmed = paragraph.trim();
        if trimmed.is_empty() {
            continue;
        }

        if buffer.len() + trimmed.len() + 2 > max_chars && !buffer.is_empty() {
            chunks.push((position, buffer.trim().to_string()));
            position += 1;
            buffer.clear();
        }

        if trimmed.len() > max_chars {
            for sentence in trimmed.split('.') {
                let sentence = sentence.trim();
                if sentence.is_empty() {
                    continue;
                }

                if buffer.len() + sentence.len() + 2 > max_chars && !buffer.is_empty() {
                    chunks.push((position, buffer.trim().to_string()));
                    position += 1;
                    buffer.clear();
                }

                buffer.push_str(sentence);
                buffer.push_str(". ");
            }
        } else {
            buffer.push_str(trimmed);
            buffer.push_str("\n\n");
        }
    }

    if !buffer.trim().is_empty() {
        chunks.push((position, buffer.trim().to_string()));
    }

    chunks
}
