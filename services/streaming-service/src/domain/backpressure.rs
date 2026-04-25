use crate::models::{sink::BackpressureSnapshot, topology::BackpressurePolicy};

pub fn derive_backpressure_snapshot(
    policy: &BackpressurePolicy,
    total_backlog: i32,
    busiest_stream_backlog: i32,
    active_streams: usize,
) -> BackpressureSnapshot {
    let queue_capacity = policy.queue_capacity.max(1);
    let queue_depth = total_backlog.max(0).min(queue_capacity);
    let queue_ratio = total_backlog.max(0) as f32 / queue_capacity as f32;
    let in_flight_ratio = total_backlog.max(0) as f32 / policy.max_in_flight.max(1) as f32;
    let hot_stream_ratio =
        busiest_stream_backlog.max(0) as f32 / policy.max_in_flight.max(1) as f32;
    let ratio = queue_ratio
        .max(in_flight_ratio * 0.65)
        .max(hot_stream_ratio * 0.45);

    let status = if ratio >= 0.8 {
        "throttling"
    } else if ratio >= 0.5 {
        "elevated"
    } else {
        "healthy"
    };

    let strategy_adjustment = if policy.throttle_strategy.eq_ignore_ascii_case("drop-tail") {
        0.05_f32
    } else {
        0.0_f32
    };
    let throttle_factor = if status == "throttling" {
        (0.62_f32 - strategy_adjustment).max(0.4_f32)
    } else if status == "elevated" {
        (0.86_f32 - strategy_adjustment).max(0.65_f32)
    } else {
        1.0_f32
    };
    let lag_ms = if total_backlog <= 0 {
        0
    } else {
        ((total_backlog - policy.max_in_flight).max(0) * 22)
            + busiest_stream_backlog.max(1) * 9
            + active_streams.max(1) as i32 * 14
    };

    BackpressureSnapshot {
        queue_depth,
        queue_capacity,
        lag_ms,
        throttle_factor,
        status: status.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::derive_backpressure_snapshot;
    use crate::models::topology::BackpressurePolicy;

    #[test]
    fn reports_healthy_when_backlog_is_small() {
        let snapshot = derive_backpressure_snapshot(&BackpressurePolicy::default(), 24, 12, 2);

        assert_eq!(snapshot.status, "healthy");
        assert_eq!(snapshot.queue_depth, 24);
        assert_eq!(snapshot.throttle_factor, 1.0);
    }

    #[test]
    fn reports_elevated_when_backlog_builds_up() {
        let snapshot = derive_backpressure_snapshot(&BackpressurePolicy::default(), 420, 160, 3);

        assert_eq!(snapshot.status, "elevated");
        assert!(snapshot.lag_ms > 0);
        assert!(snapshot.throttle_factor < 1.0);
    }

    #[test]
    fn reports_throttling_when_queue_nears_capacity() {
        let snapshot = derive_backpressure_snapshot(
            &BackpressurePolicy {
                max_in_flight: 256,
                queue_capacity: 512,
                throttle_strategy: "drop-tail".to_string(),
            },
            640,
            448,
            4,
        );

        assert_eq!(snapshot.status, "throttling");
        assert_eq!(snapshot.queue_depth, 512);
        assert!(snapshot.throttle_factor < 0.7);
    }
}
