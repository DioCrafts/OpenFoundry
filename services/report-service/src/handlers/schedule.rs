use axum::{Json, extract::State};
use chrono::Utc;

use crate::{
    AppState,
    domain::cron,
    handlers::{ServiceResult, db_error, load_all_reports, load_execution_history},
    models::snapshot::ScheduleBoard,
};

pub async fn get_schedule_board(State(state): State<AppState>) -> ServiceResult<ScheduleBoard> {
    let reports = load_all_reports(&state.db)
        .await
        .map_err(|cause| db_error(&cause))?;
    let recent_executions = load_execution_history(&state.db, None, 5)
        .await
        .map_err(|cause| db_error(&cause))?;
    let board = cron::build_schedule_board(&reports, recent_executions, Utc::now());
    Ok(Json(board))
}
