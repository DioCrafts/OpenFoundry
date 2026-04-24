use crate::models::{feature::Coordinate, spatial_index::GeocodeResponse};

const KNOWN_LOCATIONS: [(&str, Coordinate); 7] = [
    (
        "madrid",
        Coordinate {
            lat: 40.4168,
            lon: -3.7038,
        },
    ),
    (
        "barcelona",
        Coordinate {
            lat: 41.3874,
            lon: 2.1686,
        },
    ),
    (
        "paris",
        Coordinate {
            lat: 48.8566,
            lon: 2.3522,
        },
    ),
    (
        "berlin",
        Coordinate {
            lat: 52.52,
            lon: 13.405,
        },
    ),
    (
        "lisbon",
        Coordinate {
            lat: 38.7223,
            lon: -9.1393,
        },
    ),
    (
        "london",
        Coordinate {
            lat: 51.5072,
            lon: -0.1276,
        },
    ),
    (
        "new york",
        Coordinate {
            lat: 40.7128,
            lon: -74.006,
        },
    ),
];

pub fn forward(address: &str) -> GeocodeResponse {
    let normalized = address.trim().to_lowercase();
    if let Some((label, coordinate)) = KNOWN_LOCATIONS
        .iter()
        .find(|(label, _)| normalized.contains(label))
    {
        return GeocodeResponse {
            address: title_case(label),
            coordinate: *coordinate,
            confidence: 0.96,
            source: "reference gazetteer".to_string(),
        };
    }

    let hash = normalized.bytes().fold(0u64, |acc, byte| acc + byte as u64);
    GeocodeResponse {
        address: address.to_string(),
        coordinate: Coordinate {
            lat: 35.0 + (hash % 240) as f64 / 10.0,
            lon: -20.0 + (hash % 400) as f64 / 10.0,
        },
        confidence: 0.68,
        source: "deterministic fallback".to_string(),
    }
}

pub fn reverse(coordinate: Coordinate) -> GeocodeResponse {
    let (label, known_coordinate) = KNOWN_LOCATIONS
        .iter()
        .min_by(|left, right| {
            coordinate
                .distance_km(left.1)
                .partial_cmp(&coordinate.distance_km(right.1))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("known locations cannot be empty");

    GeocodeResponse {
        address: title_case(label),
        coordinate: *known_coordinate,
        confidence: 0.91,
        source: "reverse gazetteer".to_string(),
    }
}

fn title_case(value: &str) -> String {
    value
        .split_whitespace()
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
