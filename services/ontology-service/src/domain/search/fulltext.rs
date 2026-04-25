use std::collections::HashSet;

pub fn tokenize(input: &str) -> Vec<String> {
    input
        .split(|character: char| {
            !character.is_alphanumeric() && character != '_' && character != '-'
        })
        .filter(|token| !token.is_empty())
        .map(|token| token.to_lowercase())
        .collect()
}

pub fn score(query: &str, title: &str, body: &str) -> f32 {
    let query_tokens = tokenize(query);
    if query_tokens.is_empty() {
        return 0.0;
    }

    let title_tokens = tokenize(title);
    let body_tokens = tokenize(body);
    let query_set = query_tokens.iter().collect::<HashSet<_>>();
    let title_set = title_tokens.iter().collect::<HashSet<_>>();
    let body_set = body_tokens.iter().collect::<HashSet<_>>();

    let title_hits = query_set.intersection(&title_set).count() as f32;
    let body_hits = query_set.intersection(&body_set).count() as f32;
    let coverage = (title_hits * 1.5 + body_hits) / query_set.len().max(1) as f32;

    let lowered_query = query.trim().to_lowercase();
    let lowered_title = title.to_lowercase();
    let lowered_body = body.to_lowercase();

    let exact_title = if !lowered_query.is_empty() && lowered_title.contains(&lowered_query) {
        0.35
    } else {
        0.0
    };
    let exact_body = if !lowered_query.is_empty() && lowered_body.contains(&lowered_query) {
        0.15
    } else {
        0.0
    };

    (coverage / 2.5).min(1.0) + exact_title + exact_body
}

#[cfg(test)]
mod tests {
    use super::score;

    #[test]
    fn prefers_title_hits() {
        let with_title = score("customer health", "Customer Health", "weekly status board");
        let with_body = score(
            "customer health",
            "Overview",
            "customer health weekly board",
        );
        assert!(with_title > with_body);
    }
}
