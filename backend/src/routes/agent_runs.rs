use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};

use crate::{
    app::AppState,
    models::{AgentRun, ApiResponse},
    security::require_admin,
};

pub async fn list_agent_runs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<AgentRun>>>, StatusCode> {
    let _user = require_admin(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("agent runs", state.store.list_agent_runs().await)))
}
