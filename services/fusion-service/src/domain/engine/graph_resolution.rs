use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use uuid::Uuid;

use crate::models::cluster::{EntityRecord, MatchEvidence, ResolvedCluster, ReviewQueueItem};

pub struct GraphResolutionResult {
    pub clusters: Vec<ResolvedCluster>,
    pub review_items: Vec<ReviewQueueItem>,
}

pub fn resolve_clusters(
    job_id: Uuid,
    records: &[EntityRecord],
    evidences: &[MatchEvidence],
    review_threshold: f32,
    auto_merge_threshold: f32,
) -> GraphResolutionResult {
    let mut union_find = UnionFind::new(records.len());
    let record_positions = records
        .iter()
        .enumerate()
        .map(|(index, record)| (record.record_id.clone(), index))
        .collect::<BTreeMap<_, _>>();

    for evidence in evidences {
        if evidence.final_score < review_threshold {
            continue;
        }

        let Some(left_index) = record_positions.get(&evidence.left_record_id) else {
            continue;
        };
        let Some(right_index) = record_positions.get(&evidence.right_record_id) else {
            continue;
        };

        union_find.union(*left_index, *right_index);
    }

    let mut grouped = BTreeMap::<usize, Vec<EntityRecord>>::new();
    for (index, record) in records.iter().enumerate() {
        grouped
            .entry(union_find.find(index))
            .or_default()
            .push(record.clone());
    }

    let mut clusters = Vec::new();
    let mut review_items = Vec::new();
    let now = Utc::now();

    for records in grouped.into_values() {
        let cluster_id = Uuid::now_v7();
        let record_ids = records
            .iter()
            .map(|record| record.record_id.clone())
            .collect::<BTreeSet<_>>();

        let cluster_evidence = evidences
            .iter()
            .filter(|evidence| {
                record_ids.contains(&evidence.left_record_id)
                    && record_ids.contains(&evidence.right_record_id)
                    && evidence.final_score >= review_threshold
            })
            .cloned()
            .collect::<Vec<_>>();

        let requires_review = cluster_evidence.iter().any(|evidence| {
            evidence.final_score >= review_threshold && evidence.final_score < auto_merge_threshold
        });

        let confidence_score = if cluster_evidence.is_empty() {
            let total_confidence = records.iter().map(|record| record.confidence).sum::<f32>();
            (total_confidence / records.len().max(1) as f32).clamp(0.0, 1.0)
        } else {
            let total_score = cluster_evidence
                .iter()
                .map(|evidence| evidence.final_score)
                .sum::<f32>();
            (total_score / cluster_evidence.len().max(1) as f32).clamp(0.0, 1.0)
        };

        let status = if cluster_evidence.is_empty() {
            "singleton"
        } else if requires_review {
            "pending_review"
        } else {
            "resolved"
        };

        let cluster_key = records
            .iter()
            .map(|record| record.record_id.clone())
            .collect::<Vec<_>>()
            .join("|");

        let cluster = ResolvedCluster {
            id: cluster_id,
            job_id,
            cluster_key,
            status: status.to_string(),
            records: records.clone(),
            evidence: cluster_evidence.clone(),
            confidence_score,
            requires_review,
            suggested_golden_record_id: None,
            created_at: now,
            updated_at: now,
        };

        if requires_review {
            let rationale = cluster_evidence
                .iter()
                .take(3)
                .map(|evidence| evidence.explanation.clone())
                .collect::<Vec<_>>();

            review_items.push(ReviewQueueItem {
                id: Uuid::now_v7(),
                cluster_id,
                status: "pending".to_string(),
                severity: if confidence_score < auto_merge_threshold {
                    "high"
                } else {
                    "medium"
                }
                .to_string(),
                recommended_action: "manual_review".to_string(),
                rationale,
                assigned_to: None,
                reviewed_by: None,
                notes: String::new(),
                created_at: now,
                updated_at: now,
            });
        }

        clusters.push(cluster);
    }

    clusters.sort_by(|left, right| right.records.len().cmp(&left.records.len()));
    GraphResolutionResult {
        clusters,
        review_items,
    }
}

struct UnionFind {
    parents: Vec<usize>,
    ranks: Vec<usize>,
}

impl UnionFind {
    fn new(size: usize) -> Self {
        Self {
            parents: (0..size).collect(),
            ranks: vec![0; size],
        }
    }

    fn find(&mut self, index: usize) -> usize {
        if self.parents[index] != index {
            let root = self.find(self.parents[index]);
            self.parents[index] = root;
        }
        self.parents[index]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root == right_root {
            return;
        }

        if self.ranks[left_root] < self.ranks[right_root] {
            self.parents[left_root] = right_root;
        } else if self.ranks[left_root] > self.ranks[right_root] {
            self.parents[right_root] = left_root;
        } else {
            self.parents[right_root] = left_root;
            self.ranks[left_root] += 1;
        }
    }
}
