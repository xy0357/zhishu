use axum::{extract::{Path, State}, http::{HeaderMap, StatusCode}, Json};

use crate::{
    app::AppState,
    models::{ApiResponse, RoleItem, UserCreateRequest, UserItem, UserUpdateRequest},
    security::require_admin,
    store::StoreMutationError,
};

pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<UserItem>>>, StatusCode> {
    let _user = require_admin(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("user list", state.store.list_users().await)))
}

pub async fn list_roles(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<RoleItem>>>, StatusCode> {
    let _user = require_admin(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("role list", state.store.list_roles().await)))
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UserCreateRequest>,
) -> Result<(StatusCode, Json<ApiResponse<UserItem>>), StatusCode> {
    let _user = require_admin(&headers, &state).await?;
    state
        .store
        .create_user(payload)
        .await
        .map(|item| (StatusCode::CREATED, Json(ApiResponse::ok("user created", item))))
        .map_err(map_store_error)
}

pub async fn update_user(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UserUpdateRequest>,
) -> Result<Json<ApiResponse<UserItem>>, StatusCode> {
    let _user = require_admin(&headers, &state).await?;
    state
        .store
        .update_user(id, payload)
        .await
        .map(|item| Json(ApiResponse::ok("user updated", item)))
        .map_err(map_store_error)
}

fn map_store_error(error: StoreMutationError) -> StatusCode {
    match error {
        StoreMutationError::NotFound => StatusCode::NOT_FOUND,
        StoreMutationError::Conflict => StatusCode::CONFLICT,
    }
}
