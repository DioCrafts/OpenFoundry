use chrono::{DateTime, Duration, Timelike, Utc};

use crate::models::{
    report::ReportDefinition,
    schedule::{ReportSchedule, ScheduleCadence, ScheduledRun},
    snapshot::{ReportExecution, ScheduleBoard},
};

pub fn normalize_schedule(mut schedule: ReportSchedule, now: DateTime<Utc>) -> ReportSchedule {
    schedule.next_run_at = next_run_at(&schedule, now);
    schedule
}

pub fn next_run_at(schedule: &ReportSchedule, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    if !schedule.enabled {
        return None;
    }

    match schedule.cadence {
        ScheduleCadence::Manual => None,
        ScheduleCadence::Cron => {
            Some(now + Duration::minutes(schedule.interval_minutes.unwrap_or(30)))
        }
        ScheduleCadence::Daily => Some(round_to_hour(now + Duration::days(1), 9)),
        ScheduleCadence::Weekly => Some(round_to_hour(now + Duration::days(7), 9)),
        ScheduleCadence::Monthly => Some(round_to_hour(now + Duration::days(30), 9)),
    }
}

pub fn build_schedule_board(
    reports: &[ReportDefinition],
    recent_executions: Vec<ReportExecution>,
    now: DateTime<Utc>,
) -> ScheduleBoard {
    let mut upcoming = reports
        .iter()
        .filter_map(|report| {
            report
                .schedule
                .next_run_at
                .or_else(|| next_run_at(&report.schedule, now))
                .map(|next_run_at| ScheduledRun {
                    report_id: report.id,
                    report_name: report.name.clone(),
                    generator_kind: report.generator_kind,
                    next_run_at,
                    recipient_count: report.recipients.len(),
                    cadence: report.schedule.cadence,
                })
        })
        .collect::<Vec<_>>();

    upcoming.sort_by_key(|run| run.next_run_at);

    ScheduleBoard {
        active_schedules: reports
            .iter()
            .filter(|report| report.schedule.enabled)
            .count(),
        paused_reports: reports
            .iter()
            .filter(|report| !report.schedule.enabled)
            .count(),
        upcoming: upcoming.into_iter().take(8).collect(),
        recent_executions,
    }
}

fn round_to_hour(timestamp: DateTime<Utc>, hour: u32) -> DateTime<Utc> {
    timestamp
        .with_hour(hour)
        .and_then(|value| value.with_minute(0))
        .and_then(|value| value.with_second(0))
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(timestamp)
}
