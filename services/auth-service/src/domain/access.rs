pub const VALID_MARKINGS: &[&str] = &["public", "confidential", "pii"];

pub fn validate_marking(marking: &str) -> Result<(), String> {
    if VALID_MARKINGS.contains(&marking) {
        Ok(())
    } else {
        Err(format!(
            "invalid marking '{marking}', valid markings: {VALID_MARKINGS:?}"
        ))
    }
}

pub fn marking_rank(marking: &str) -> Option<u8> {
    match marking {
        "public" => Some(0),
        "confidential" => Some(1),
        "pii" => Some(2),
        _ => None,
    }
}

pub fn normalize_markings(values: &[String]) -> Result<Vec<String>, String> {
    let mut markings = values
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    markings.sort();
    markings.dedup();

    for marking in &markings {
        validate_marking(marking)?;
    }

    Ok(markings)
}

pub fn max_marking(values: &[String]) -> Option<String> {
    values
        .iter()
        .filter_map(|candidate| marking_rank(candidate).map(|rank| (candidate, rank)))
        .max_by_key(|(_, rank)| *rank)
        .map(|(candidate, _)| candidate.clone())
}

pub fn markings_for_clearance(clearance: Option<&str>) -> Vec<String> {
    let rank = clearance.and_then(marking_rank).unwrap_or(0);

    VALID_MARKINGS
        .iter()
        .copied()
        .filter(|marking| marking_rank(marking).unwrap_or(0) <= rank)
        .map(str::to_string)
        .collect()
}
