use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorContractCatalog {
    pub connectors: Vec<ConnectorContractProfile>,
    pub certification_summary: ConnectorCertificationSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorContractProfile {
    pub connector_type: String,
    pub display_name: String,
    pub template_family: String,
    pub auth: ConnectorAuthProfile,
    pub testing: ConnectorTestingProfile,
    pub sync: ConnectorSyncProfile,
    pub observability: ConnectorObservabilityProfile,
    pub builder: ConnectorBuilderProfile,
    pub certification: ConnectorCertificationProfile,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorAuthProfile {
    pub strategy: String,
    pub secret_fields: Vec<String>,
    pub supports_oauth: bool,
    pub supports_private_network_agent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorTestingProfile {
    pub supports_connection_testing: bool,
    pub supports_discovery: bool,
    pub supports_schema_introspection: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorSyncProfile {
    pub modes: Vec<String>,
    pub supports_incremental: bool,
    pub supports_cdc: bool,
    pub supports_zero_copy: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorObservabilityProfile {
    pub retries: bool,
    pub status_tracking: bool,
    pub source_signatures: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorBuilderProfile {
    pub scaffold_kind: String,
    pub reusable_components: Vec<String>,
    pub example_targets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCertificationProfile {
    pub level: String,
    pub runtime_depth: String,
    pub auth: String,
    pub observability: String,
    pub schema_evolution: String,
    pub performance_posture: String,
    pub failure_handling: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectorCertificationSummary {
    pub certified_connectors: usize,
    pub advanced_connectors: usize,
    pub connectors_needing_hardening: usize,
    pub template_families: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionCapabilityResponse {
    pub connection_id: uuid::Uuid,
    pub connector_type: String,
    pub status: String,
    pub contract: ConnectorContractProfile,
}

pub fn connector_profiles() -> Vec<ConnectorContractProfile> {
    vec![
        sql_profile("postgresql", "PostgreSQL", false),
        sql_profile("mysql", "MySQL", false),
        warehouse_profile("snowflake", "Snowflake"),
        warehouse_profile("bigquery", "BigQuery"),
        object_store_profile("s3", "Amazon S3"),
        event_bus_profile("kafka", "Kafka"),
        saas_profile("salesforce", "Salesforce"),
        erp_profile("sap", "SAP"),
    ]
}

pub fn certification_summary() -> ConnectorCertificationSummary {
    let profiles = connector_profiles();
    let certified_connectors = profiles
        .iter()
        .filter(|profile| profile.certification.level == "certified")
        .count();
    let advanced_connectors = profiles
        .iter()
        .filter(|profile| profile.certification.level == "advanced")
        .count();
    let connectors_needing_hardening = profiles
        .len()
        .saturating_sub(certified_connectors + advanced_connectors);
    let mut template_families = profiles
        .iter()
        .map(|profile| profile.template_family.clone())
        .collect::<Vec<_>>();
    template_families.sort();
    template_families.dedup();

    ConnectorCertificationSummary {
        certified_connectors,
        advanced_connectors,
        connectors_needing_hardening,
        template_families,
    }
}

pub fn connector_profile(connector_type: &str) -> Option<ConnectorContractProfile> {
    connector_profiles()
        .into_iter()
        .find(|profile| profile.connector_type == connector_type)
}

fn sql_profile(
    connector_type: &str,
    display_name: &str,
    supports_cdc: bool,
) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "sql_tabular".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "username_password".to_string(),
            secret_fields: vec!["user".to_string(), "password".to_string()],
            supports_oauth: false,
            supports_private_network_agent: false,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec![
                "batch".to_string(),
                "incremental".to_string(),
                "zero_copy".to_string(),
            ],
            supports_incremental: true,
            supports_cdc,
            supports_zero_copy: true,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "sql_connector".to_string(),
            reusable_components: vec![
                "connection_testing".to_string(),
                "schema_introspection".to_string(),
                "virtual_tables".to_string(),
                "bulk_registration".to_string(),
            ],
            example_targets: vec!["postgresql".to_string(), "mysql".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: if supports_cdc {
                "certified".to_string()
            } else {
                "advanced".to_string()
            },
            runtime_depth: if supports_cdc {
                "batch_incremental_cdc_zero_copy".to_string()
            } else {
                "batch_incremental_zero_copy".to_string()
            },
            auth: "certified".to_string(),
            observability: "advanced".to_string(),
            schema_evolution: "advanced".to_string(),
            performance_posture: "advanced".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec![
            "Shared SQL connector contract with validation, discovery and virtual tables."
                .to_string(),
        ],
    }
}

fn warehouse_profile(connector_type: &str, display_name: &str) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "warehouse_zero_copy".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "service_account_or_key_pair".to_string(),
            secret_fields: vec!["credentials".to_string(), "private_key".to_string()],
            supports_oauth: false,
            supports_private_network_agent: true,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec![
                "batch".to_string(),
                "incremental".to_string(),
                "zero_copy".to_string(),
            ],
            supports_incremental: true,
            supports_cdc: false,
            supports_zero_copy: true,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "warehouse_connector".to_string(),
            reusable_components: vec![
                "connection_testing".to_string(),
                "discovery".to_string(),
                "schema_introspection".to_string(),
                "zero_copy_query".to_string(),
            ],
            example_targets: vec!["snowflake".to_string(), "bigquery".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: "advanced".to_string(),
            runtime_depth: "batch_incremental_zero_copy".to_string(),
            auth: "advanced".to_string(),
            observability: "advanced".to_string(),
            schema_evolution: "advanced".to_string(),
            performance_posture: "advanced".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec!["Optimized for discovery plus virtual-table query flows.".to_string()],
    }
}

fn object_store_profile(connector_type: &str, display_name: &str) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "object_storage".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "access_key".to_string(),
            secret_fields: vec!["access_key".to_string(), "secret_key".to_string()],
            supports_oauth: false,
            supports_private_network_agent: true,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec!["batch".to_string(), "incremental".to_string()],
            supports_incremental: true,
            supports_cdc: false,
            supports_zero_copy: false,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "object_storage_connector".to_string(),
            reusable_components: vec![
                "path_discovery".to_string(),
                "schema_sampling".to_string(),
                "artifact_sync".to_string(),
            ],
            example_targets: vec!["s3".to_string(), "parquet".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: "advanced".to_string(),
            runtime_depth: "batch_incremental".to_string(),
            auth: "advanced".to_string(),
            observability: "advanced".to_string(),
            schema_evolution: "baseline".to_string(),
            performance_posture: "advanced".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec![
            "Shared object storage template for path discovery and artifact sync.".to_string(),
        ],
    }
}

fn event_bus_profile(connector_type: &str, display_name: &str) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "event_bus".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "brokers_and_credentials".to_string(),
            secret_fields: vec!["username".to_string(), "password".to_string()],
            supports_oauth: false,
            supports_private_network_agent: true,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec![
                "streaming".to_string(),
                "incremental".to_string(),
                "zero_copy".to_string(),
            ],
            supports_incremental: true,
            supports_cdc: true,
            supports_zero_copy: true,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "event_bus_connector".to_string(),
            reusable_components: vec![
                "topic_discovery".to_string(),
                "schema_projection".to_string(),
                "stream_materialization".to_string(),
            ],
            example_targets: vec!["kafka".to_string(), "kinesis".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: "certified".to_string(),
            runtime_depth: "streaming_incremental_zero_copy".to_string(),
            auth: "advanced".to_string(),
            observability: "certified".to_string(),
            schema_evolution: "advanced".to_string(),
            performance_posture: "advanced".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec![
            "Shared event-bus template with topic discovery, zero-copy preview and sync."
                .to_string(),
        ],
    }
}

fn saas_profile(connector_type: &str, display_name: &str) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "saas_api".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "oauth_or_api_key".to_string(),
            secret_fields: vec!["client_id".to_string(), "client_secret".to_string()],
            supports_oauth: true,
            supports_private_network_agent: true,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec!["batch".to_string(), "incremental".to_string()],
            supports_incremental: true,
            supports_cdc: false,
            supports_zero_copy: false,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "saas_connector".to_string(),
            reusable_components: vec![
                "oauth_bootstrap".to_string(),
                "discovery".to_string(),
                "normalized_sync".to_string(),
            ],
            example_targets: vec!["salesforce".to_string(), "servicenow".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: "advanced".to_string(),
            runtime_depth: "batch_incremental".to_string(),
            auth: "certified".to_string(),
            observability: "advanced".to_string(),
            schema_evolution: "baseline".to_string(),
            performance_posture: "baseline".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec![
            "Shared SaaS adapter contract with discovery and normalized sync flows.".to_string(),
        ],
    }
}

fn erp_profile(connector_type: &str, display_name: &str) -> ConnectorContractProfile {
    ConnectorContractProfile {
        connector_type: connector_type.to_string(),
        display_name: display_name.to_string(),
        template_family: "erp_api".to_string(),
        auth: ConnectorAuthProfile {
            strategy: "service_account_or_basic".to_string(),
            secret_fields: vec![
                "username".to_string(),
                "password".to_string(),
                "api_key".to_string(),
            ],
            supports_oauth: false,
            supports_private_network_agent: true,
        },
        testing: ConnectorTestingProfile {
            supports_connection_testing: true,
            supports_discovery: true,
            supports_schema_introspection: true,
        },
        sync: ConnectorSyncProfile {
            modes: vec!["batch".to_string(), "incremental".to_string()],
            supports_incremental: true,
            supports_cdc: false,
            supports_zero_copy: false,
        },
        observability: ConnectorObservabilityProfile {
            retries: true,
            status_tracking: true,
            source_signatures: true,
        },
        builder: ConnectorBuilderProfile {
            scaffold_kind: "erp_connector".to_string(),
            reusable_components: vec![
                "entity_discovery".to_string(),
                "hyperauto_blueprints".to_string(),
                "ontology_scaffolding".to_string(),
            ],
            example_targets: vec!["sap".to_string(), "salesforce".to_string()],
        },
        certification: ConnectorCertificationProfile {
            level: if connector_type == "sap" {
                "advanced".to_string()
            } else {
                "baseline".to_string()
            },
            runtime_depth: "batch_incremental_hyperauto".to_string(),
            auth: "advanced".to_string(),
            observability: "advanced".to_string(),
            schema_evolution: "advanced".to_string(),
            performance_posture: "baseline".to_string(),
            failure_handling: "advanced".to_string(),
        },
        notes: vec![
            "Feeds HyperAuto generation and ontology scaffolding from ERP entities.".to_string(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::{certification_summary, connector_profile, connector_profiles};

    #[test]
    fn exposes_phase_one_high_value_connectors() {
        let profiles = connector_profiles();
        assert!(profiles.len() >= 8);
        for connector in [
            "postgresql",
            "mysql",
            "snowflake",
            "bigquery",
            "s3",
            "kafka",
            "salesforce",
            "sap",
        ] {
            assert!(
                connector_profile(connector).is_some(),
                "missing {connector}"
            );
        }
    }

    #[test]
    fn computes_connector_certification_summary() {
        let summary = certification_summary();
        assert!(summary.certified_connectors >= 1);
        assert!(
            summary
                .template_families
                .iter()
                .any(|family| family == "sql_tabular")
        );
    }
}
