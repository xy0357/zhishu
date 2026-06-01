use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};

use crate::{
    app::AppState,
    models::{ApiResponse, DashboardSummary},
    security::require_user,
};

pub async fn dashboard_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DashboardSummary>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    let data = state.store.dashboard_summary().await;
    Ok(Json(ApiResponse::ok("dashboard summary", data)))
}
