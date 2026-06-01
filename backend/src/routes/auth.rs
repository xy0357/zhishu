use axum::{extract::{State}, http::HeaderMap, Json};

use crate::{
    app::AppState,
    models::{ApiResponse, LoginRequest},
    security::{build_auth_session, require_user, unauthorized_response},
};

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> axum::response::Response {
    match state
        .store
        .authenticate(payload.username, payload.password)
        .await
    {
        Some(user) => Json(ApiResponse::ok("login success", build_auth_session(&state.config, user)))
            .into_response(),
        None => unauthorized_response().into_response(),
    }
}

pub async fn current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> axum::response::Response {
    match require_user(&headers, &state).await {
        Ok(user) => Json(ApiResponse::ok("current user", user)).into_response(),
        Err(_) => unauthorized_response().into_response(),
    }
}

use axum::response::IntoResponse;
