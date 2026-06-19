use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};

use crate::{
    app::AppState,
    models::{ApiResponse, DashboardSummary},
    redis_cache,
    security::require_user,
};

pub async fn dashboard_summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DashboardSummary>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    let cache_key = format!("zhishu:dashboard:summary:{}", state.config.storage_backend);
    let data = if let Some(cached) =
        redis_cache::get_json::<DashboardSummary>(&state.config.redis_url, &cache_key).await
    {
        cached
    } else {
        let fresh = state.store.dashboard_summary().await;
        let _ = redis_cache::set_json(&state.config.redis_url, &cache_key, 15, &fresh).await;
        fresh
    };
    Ok(Json(ApiResponse::ok("dashboard summary", data)))
}
