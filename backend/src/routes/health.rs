use axum::{extract::State, Json};

use crate::{app::AppState, models::ApiResponse};

pub async fn health_check(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(
        "ok",
        serde_json::json!({
            "service": state.config.app_name,
            "status": "healthy"
        }),
    ))
}

