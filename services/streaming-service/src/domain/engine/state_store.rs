use chrono::Utc;

use crate::models::{sink::StateStoreSnapshot, topology::TopologyDefinition};

pub fn simulate_state_store(topology: &TopologyDefinition, key_count: i32) -> StateStoreSnapshot {
    StateStoreSnapshot {
        backend: topology.state_backend.clone(),
        namespace: topology.name.to_lowercase().replace(' ', "-"),
        key_count,
        disk_usage_mb: 96 + key_count / 4,
        checkpoint_count: 7,
        last_checkpoint_at: Utc::now(),
    }
}
