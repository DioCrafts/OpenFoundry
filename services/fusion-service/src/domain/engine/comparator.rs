use std::cmp::{max, min};

pub fn normalize_text(input: &str) -> String {
    input
        .to_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect()
}

pub fn normalize_phone(input: &str) -> String {
    input
        .chars()
        .filter(|character| character.is_ascii_digit())
        .collect()
}

pub fn levenshtein_similarity(left: &str, right: &str) -> f32 {
    let normalized_left = normalize_text(left);
    let normalized_right = normalize_text(right);

    if normalized_left.is_empty() && normalized_right.is_empty() {
        return 1.0;
    }

    let left_chars = normalized_left.chars().collect::<Vec<_>>();
    let right_chars = normalized_right.chars().collect::<Vec<_>>();

    let mut previous = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut current = vec![0; right_chars.len() + 1];

    for (left_index, left_character) in left_chars.iter().enumerate() {
        current[0] = left_index + 1;
        for (right_index, right_character) in right_chars.iter().enumerate() {
            let cost = usize::from(left_character != right_character);
            current[right_index + 1] = min(
                min(current[right_index] + 1, previous[right_index + 1] + 1),
                previous[right_index] + cost,
            );
        }
        previous.clone_from(&current);
    }

    let distance = previous[right_chars.len()];
    let max_len = max(left_chars.len(), right_chars.len()).max(1) as f32;
    (1.0 - distance as f32 / max_len).clamp(0.0, 1.0)
}

pub fn jaro_winkler_similarity(left: &str, right: &str) -> f32 {
    let left = normalize_text(left);
    let right = normalize_text(right);

    if left == right {
        return 1.0;
    }
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }

    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let match_distance = max(left_chars.len(), right_chars.len()) / 2;
    let match_distance = match_distance.saturating_sub(1);

    let mut left_matches = vec![false; left_chars.len()];
    let mut right_matches = vec![false; right_chars.len()];
    let mut matches = 0;

    for (left_index, left_character) in left_chars.iter().enumerate() {
        let start = left_index.saturating_sub(match_distance);
        let end = min(left_index + match_distance + 1, right_chars.len());

        for right_index in start..end {
            if right_matches[right_index] || *left_character != right_chars[right_index] {
                continue;
            }

            left_matches[left_index] = true;
            right_matches[right_index] = true;
            matches += 1;
            break;
        }
    }

    if matches == 0 {
        return 0.0;
    }

    let mut transpositions = 0;
    let mut right_cursor = 0;
    for (left_index, matched) in left_matches.iter().enumerate() {
        if !matched {
            continue;
        }

        while right_cursor < right_matches.len() && !right_matches[right_cursor] {
            right_cursor += 1;
        }

        if right_cursor < right_chars.len() && left_chars[left_index] != right_chars[right_cursor] {
            transpositions += 1;
        }
        right_cursor += 1;
    }

    let matches = matches as f32;
    let jaro = (matches / left_chars.len() as f32
        + matches / right_chars.len() as f32
        + (matches - transpositions as f32 / 2.0) / matches)
        / 3.0;

    let common_prefix = left_chars
        .iter()
        .zip(right_chars.iter())
        .take_while(|(left_char, right_char)| left_char == right_char)
        .take(4)
        .count() as f32;

    (jaro + common_prefix * 0.1 * (1.0 - jaro)).clamp(0.0, 1.0)
}

pub fn soundex(input: &str) -> String {
    let normalized = normalize_text(input);
    let mut characters = normalized.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };

    let mut output = String::from(first.to_ascii_uppercase());
    let mut previous_code = map_soundex(first);

    for character in characters {
        let code = map_soundex(character);
        if code != '0' && code != previous_code {
            output.push(code);
        }
        previous_code = code;
        if output.len() == 4 {
            break;
        }
    }

    while output.len() < 4 {
        output.push('0');
    }

    output
}

fn map_soundex(character: char) -> char {
    match character {
        'b' | 'f' | 'p' | 'v' => '1',
        'c' | 'g' | 'j' | 'k' | 'q' | 's' | 'x' | 'z' => '2',
        'd' | 't' => '3',
        'l' => '4',
        'm' | 'n' => '5',
        'r' => '6',
        _ => '0',
    }
}

pub fn metaphone(input: &str) -> String {
    let normalized = normalize_text(input);
    if normalized.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    let mut previous = '\0';

    for (index, character) in normalized.chars().enumerate() {
        if index > 0 && "aeiou".contains(character) {
            continue;
        }

        let mapped = match character {
            'b' | 'f' | 'p' | 'v' => 'B',
            'c' | 'g' | 'j' | 'k' | 'q' => 'K',
            's' | 'x' | 'z' => 'S',
            'd' | 't' => 'T',
            'l' => 'L',
            'm' | 'n' => 'N',
            'r' => 'R',
            'h' | 'w' => continue,
            _ => character.to_ascii_uppercase(),
        };

        if mapped != previous {
            output.push(mapped);
        }
        previous = mapped;
    }

    output
}

pub fn compare_values(comparator: &str, left: &str, right: &str) -> f32 {
    match comparator {
        "exact" => f32::from(normalize_text(left) == normalize_text(right)),
        "email_exact" => f32::from(left.trim().eq_ignore_ascii_case(right.trim())),
        "phone_exact" => f32::from(normalize_phone(left) == normalize_phone(right)),
        "levenshtein" => levenshtein_similarity(left, right),
        "jaro_winkler" => jaro_winkler_similarity(left, right),
        "soundex" => f32::from(soundex(left) == soundex(right)),
        "metaphone" => f32::from(metaphone(left) == metaphone(right)),
        "phonetic" => {
            let soundex_score = f32::from(soundex(left) == soundex(right));
            let metaphone_score = f32::from(metaphone(left) == metaphone(right));
            ((soundex_score + metaphone_score) / 2.0).clamp(0.0, 1.0)
        }
        _ => levenshtein_similarity(left, right)
            .max(jaro_winkler_similarity(left, right))
            .clamp(0.0, 1.0),
    }
}
