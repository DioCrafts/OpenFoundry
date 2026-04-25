use chrono::Utc;
use uuid::Uuid;

use crate::{AppState, models::agent::ConnectorAgent};

pub async fn resolve_agent_url(
    state: &AppState,
    connection_config: &serde_json::Value,
) -> Result<Option<String>, String> {
    if let Some(url) = connection_config
        .get("agent_url")
        .and_then(serde_json::Value::as_str)
    {
        let trimmed = url.trim();
        if !trimmed.is_empty() {
            return Ok(Some(trimmed.to_string()));
        }
    }

    let Some(agent_id) = connection_config
        .get("agent_id")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
    else {
        return Ok(None);
    };

    let agent_id = Uuid::parse_str(agent_id).map_err(|error| error.to_string())?;
    let Some(agent) =
        sqlx::query_as::<_, ConnectorAgent>("SELECT * FROM connector_agents WHERE id = $1")
            .bind(agent_id)
            .fetch_optional(&state.db)
            .await
            .map_err(|error| error.to_string())?
    else {
        return Err(format!("connector agent '{agent_id}' not found"));
    };

    if agent.status != "online" {
        return Err(format!(
            "connector agent '{}' is not online (status: {})",
            agent.name, agent.status
        ));
    }

    let stale_cutoff = Utc::now() - state.agent_stale_after;
    if let Some(last_heartbeat_at) = agent.last_heartbeat_at
        && last_heartbeat_at < stale_cutoff
    {
        return Err(format!(
            "connector agent '{}' heartbeat is stale",
            agent.name
        ));
    }

    Ok(Some(agent.agent_url))
}
