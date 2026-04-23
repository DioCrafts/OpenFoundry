/// Well-known NATS subject prefixes for each service domain.
pub mod subjects {
    pub const AUTH: &str = "of.auth";
    pub const DATASETS: &str = "of.datasets";
    pub const PIPELINES: &str = "of.pipelines";
    pub const ONTOLOGY: &str = "of.ontology";
    pub const QUERIES: &str = "of.queries";
    pub const AUDIT: &str = "of.audit";
    pub const NOTIFICATIONS: &str = "of.notifications";
}

/// JetStream stream names.
pub mod streams {
    pub const EVENTS: &str = "OF_EVENTS";
    pub const AUDIT: &str = "OF_AUDIT";
    pub const NOTIFICATIONS: &str = "OF_NOTIFICATIONS";
}

