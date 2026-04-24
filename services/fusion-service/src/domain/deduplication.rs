use serde_json::json;

use crate::models::{cluster::EntityRecord, job::ResolutionJobConfig};

pub fn synthesize_entity_records(
    entity_type: &str,
    config: &ResolutionJobConfig,
) -> Vec<EntityRecord> {
    if entity_type.eq_ignore_ascii_case("organization") {
        synthesize_organization_records(config)
    } else {
        synthesize_person_records(config)
    }
}

fn synthesize_person_records(config: &ResolutionJobConfig) -> Vec<EntityRecord> {
    let profiles = [
        (
            ["John Smith", "Jon Smyth", "J. Smith"],
            [
                "john.smith@acme.com",
                "jon.smyth@acme.com",
                "john.smith+support@acme.com",
            ],
            ["+1 415 555 0100", "+1 (415) 555-0100", "4155550100"],
            "San Francisco",
            "Acme Logistics",
            "100 Market St",
        ),
        (
            ["Ana Perez", "Anna Peres", "A. Perez"],
            [
                "ana.perez@nova.io",
                "anna.peres@nova.io",
                "ana.perez+ops@nova.io",
            ],
            ["+34 91 555 0101", "+34 915550101", "91-555-0101"],
            "Madrid",
            "Nova Energy",
            "Calle Gran Via 44",
        ),
        (
            ["Mark Johnson", "Marc Jonson", "M. Johnson"],
            [
                "mark.johnson@northwind.co",
                "marc.jonson@northwind.co",
                "mark.johnson+help@northwind.co",
            ],
            ["+1 646 555 0102", "+1-646-555-0102", "6465550102"],
            "New York",
            "Northwind Trading",
            "215 Madison Ave",
        ),
        (
            ["Mei Chen", "May Chen", "M. Chen"],
            [
                "mei.chen@harbor.ai",
                "may.chen@harbor.ai",
                "mei.chen+crm@harbor.ai",
            ],
            ["+65 6555 0103", "+65-6555-0103", "65550103"],
            "Singapore",
            "Harbor AI",
            "12 Anson Rd",
        ),
    ];

    build_records(config, &profiles, "person")
}

fn synthesize_organization_records(config: &ResolutionJobConfig) -> Vec<EntityRecord> {
    let profiles = [
        (
            ["Acme Logistics", "ACME Logstics", "Acme Log."],
            [
                "ops@acme-logistics.com",
                "support@acme-logistics.com",
                "hello@acme-logistics.com",
            ],
            ["+1 415 555 1000", "+1 (415) 555-1000", "4155551000"],
            "San Francisco",
            "Acme Logistics",
            "100 Market St",
        ),
        (
            ["Nova Energy", "Nova Energi", "Nova Eng."],
            [
                "info@novaenergy.io",
                "ops@novaenergy.io",
                "hello@novaenergy.io",
            ],
            ["+34 91 555 1001", "+34 915551001", "915551001"],
            "Madrid",
            "Nova Energy",
            "Calle Gran Via 44",
        ),
        (
            ["Northwind Trading", "North Wind Trading", "Northwind Trdg"],
            [
                "ops@northwind.co",
                "support@northwind.co",
                "hello@northwind.co",
            ],
            ["+1 646 555 1002", "+1-646-555-1002", "6465551002"],
            "New York",
            "Northwind Trading",
            "215 Madison Ave",
        ),
    ];

    build_records(config, &profiles, "organization")
}

fn build_records(
    config: &ResolutionJobConfig,
    profiles: &[([&str; 3], [&str; 3], [&str; 3], &str, &str, &str)],
    record_type: &str,
) -> Vec<EntityRecord> {
    let source_labels = if config.source_labels.is_empty() {
        vec!["crm".to_string(), "erp".to_string(), "support".to_string()]
    } else {
        config.source_labels.clone()
    };

    let mut records = Vec::new();
    let target_count = config.record_count.max(9) as usize;

    'outer: for cycle in 0..4 {
        for (profile_index, profile) in profiles.iter().enumerate() {
            for (source_index, source) in source_labels.iter().enumerate() {
                let name = profile.0[source_index % profile.0.len()].to_string();
                let email = profile.1[source_index % profile.1.len()].to_string();
                let phone = profile.2[source_index % profile.2.len()].to_string();
                let external_id = format!("{}-{}-{}", source, profile_index + 1, cycle + 1);

                records.push(EntityRecord {
                    record_id: format!("{source}:{record_type}:{external_id}"),
                    source: source.clone(),
                    external_id: external_id.clone(),
                    display_name: name.clone(),
                    confidence: (0.82 - cycle as f32 * 0.03).clamp(0.45, 0.95),
                    attributes: json!({
                        "name": name,
                        "email": email,
                        "phone": phone,
                        "city": profile.3,
                        "company": profile.4,
                        "address": profile.5,
                        "source_rank": source_index + 1,
                    }),
                });

                if records.len() >= target_count {
                    break 'outer;
                }
            }
        }
    }

    records
}
