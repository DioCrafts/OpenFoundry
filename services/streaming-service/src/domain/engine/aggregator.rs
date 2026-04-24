use chrono::{Duration, Utc};

use crate::models::{
    sink::WindowAggregate, topology::TopologyDefinition, window::WindowDefinition,
};

pub fn simulate_window_aggregates(
    window: Option<&WindowDefinition>,
    topology: &TopologyDefinition,
) -> Vec<WindowAggregate> {
    let now = Utc::now();
    let duration_seconds = window.map(|value| value.duration_seconds).unwrap_or(300) as i64;
    let slide_seconds = window
        .map(|value| value.slide_seconds)
        .unwrap_or(duration_seconds as i32) as i64;
    let window_name = window
        .map(|value| value.name.clone())
        .unwrap_or_else(|| format!("{} inline window", topology.name));
    let window_type = window
        .map(|value| value.window_type.clone())
        .unwrap_or_else(|| "tumbling".to_string());

    vec![
        WindowAggregate {
            window_name: window_name.clone(),
            window_type: window_type.clone(),
            bucket_start: now - Duration::seconds(duration_seconds + slide_seconds),
            bucket_end: now - Duration::seconds(slide_seconds),
            group_key: "customer_id:cust-22".to_string(),
            measure_name: "events_per_window".to_string(),
            value: 28.0,
        },
        WindowAggregate {
            window_name,
            window_type,
            bucket_start: now - Duration::seconds(duration_seconds),
            bucket_end: now,
            group_key: "risk_band:medium".to_string(),
            measure_name: "total_amount".to_string(),
            value: 1730.5,
        },
    ]
}
