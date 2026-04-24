use crate::models::{sink::BackpressureSnapshot, topology::BackpressurePolicy};

pub fn simulate_backpressure(
    policy: &BackpressurePolicy,
    stream_count: usize,
    has_join: bool,
) -> BackpressureSnapshot {
    let queue_capacity = policy.queue_capacity.max(128);
    let mut queue_depth = 48 + stream_count as i32 * 22;
    if has_join {
        queue_depth += 72;
    }
    queue_depth = queue_depth.min(queue_capacity - 4).max(8);

    let lag_ms = 95 + stream_count as i32 * 38 + if has_join { 140 } else { 0 };
    let ratio = queue_depth as f32 / queue_capacity as f32;
    let status = if ratio >= 0.78 {
        "throttling"
    } else if ratio >= 0.52 {
        "elevated"
    } else {
        "healthy"
    };
    let throttle_factor = if status == "throttling" {
        0.72
    } else if status == "elevated" {
        0.88
    } else {
        1.0
    };

    BackpressureSnapshot {
        queue_depth,
        queue_capacity,
        lag_ms,
        throttle_factor,
        status: status.to_string(),
    }
}
