use std::collections::{BTreeMap, BTreeSet};

use crate::models::{cluster::EntityRecord, match_rule::BlockingStrategyConfig};

use super::comparator::normalize_text;

#[derive(Debug, Clone)]
pub struct CandidatePair {
    pub left: EntityRecord,
    pub right: EntityRecord,
    pub blocking_key: String,
}

pub fn build_candidate_pairs(
    records: &[EntityRecord],
    strategy: &BlockingStrategyConfig,
) -> Vec<CandidatePair> {
    match strategy.strategy_type.as_str() {
        "sorted-neighborhood" => sorted_neighborhood_pairs(records, strategy),
        "lsh" => lsh_pairs(records, strategy),
        _ => key_based_pairs(records, strategy),
    }
}

fn key_based_pairs(
    records: &[EntityRecord],
    strategy: &BlockingStrategyConfig,
) -> Vec<CandidatePair> {
    let mut groups = BTreeMap::<String, Vec<&EntityRecord>>::new();
    for record in records {
        groups
            .entry(blocking_key(record, &strategy.key_fields))
            .or_default()
            .push(record);
    }

    let mut emitted = BTreeSet::new();
    let mut pairs = Vec::new();

    for (key, group) in groups {
        for left_index in 0..group.len() {
            for right_index in left_index + 1..group.len() {
                emit_pair(
                    &mut pairs,
                    &mut emitted,
                    &key,
                    group[left_index],
                    group[right_index],
                );
            }
        }
    }

    pairs
}

fn sorted_neighborhood_pairs(
    records: &[EntityRecord],
    strategy: &BlockingStrategyConfig,
) -> Vec<CandidatePair> {
    let mut sorted = records
        .iter()
        .map(|record| (blocking_key(record, &strategy.key_fields), record))
        .collect::<Vec<_>>();
    sorted.sort_by(|left, right| left.0.cmp(&right.0));

    let window_size = strategy.window_size.max(2) as usize;
    let mut emitted = BTreeSet::new();
    let mut pairs = Vec::new();

    for left_index in 0..sorted.len() {
        let upper = (left_index + window_size).min(sorted.len());
        for right_index in left_index + 1..upper {
            let key = format!("{}|{}", sorted[left_index].0, sorted[right_index].0);
            emit_pair(
                &mut pairs,
                &mut emitted,
                &key,
                sorted[left_index].1,
                sorted[right_index].1,
            );
        }
    }

    pairs
}

fn lsh_pairs(records: &[EntityRecord], strategy: &BlockingStrategyConfig) -> Vec<CandidatePair> {
    let bucket_count = strategy.bucket_count.max(4) as u64;
    let mut buckets = BTreeMap::<String, Vec<&EntityRecord>>::new();

    for record in records {
        for bucket in record_buckets(record, bucket_count) {
            buckets.entry(bucket).or_default().push(record);
        }
    }

    let mut emitted = BTreeSet::new();
    let mut pairs = Vec::new();

    for (bucket, group) in buckets {
        for left_index in 0..group.len() {
            for right_index in left_index + 1..group.len() {
                emit_pair(
                    &mut pairs,
                    &mut emitted,
                    &bucket,
                    group[left_index],
                    group[right_index],
                );
            }
        }
    }

    pairs
}

fn record_buckets(record: &EntityRecord, bucket_count: u64) -> Vec<String> {
    let mut buckets = BTreeSet::new();
    let seed = normalize_text(&record.display_name);

    for token in seed.split_whitespace() {
        if token.is_empty() {
            continue;
        }

        let hash = token.bytes().fold(0_u64, |accumulator, byte| {
            accumulator.wrapping_mul(31).wrapping_add(byte as u64)
        });
        buckets.insert(format!("bucket-{}", hash % bucket_count));
    }

    if buckets.is_empty() {
        buckets.insert("bucket-0".to_string());
    }

    buckets.into_iter().collect()
}

fn emit_pair(
    pairs: &mut Vec<CandidatePair>,
    emitted: &mut BTreeSet<String>,
    blocking_key: &str,
    left: &EntityRecord,
    right: &EntityRecord,
) {
    let pair_key = pair_key(&left.record_id, &right.record_id);
    if !emitted.insert(pair_key) {
        return;
    }

    pairs.push(CandidatePair {
        left: left.clone(),
        right: right.clone(),
        blocking_key: blocking_key.to_string(),
    });
}

fn pair_key(left_id: &str, right_id: &str) -> String {
    if left_id <= right_id {
        format!("{left_id}::{right_id}")
    } else {
        format!("{right_id}::{left_id}")
    }
}

fn blocking_key(record: &EntityRecord, key_fields: &[String]) -> String {
    let mut parts = key_fields
        .iter()
        .filter_map(|field| extract_field(record, field))
        .map(|value| normalize_text(&value))
        .filter(|value| !value.is_empty())
        .map(|value| value.chars().take(6).collect::<String>())
        .collect::<Vec<_>>();

    if parts.is_empty() {
        parts.push(
            normalize_text(&record.display_name)
                .chars()
                .take(6)
                .collect(),
        );
    }

    parts.join("|")
}

fn extract_field(record: &EntityRecord, field: &str) -> Option<String> {
    match field {
        "display_name" | "name" => Some(record.display_name.clone()),
        _ => record
            .attributes
            .get(field)
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned),
    }
}
