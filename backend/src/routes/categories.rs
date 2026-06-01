use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    app::AppState,
    models::{ApiResponse, CategoryItem, CategoryUpsertRequest, DeletedResource},
    security::{require_taxonomy_manager, require_user},
    store::StoreMutationError,
};

pub async fn list_categories(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<CategoryItem>>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("categories", state.store.list_categories().await)))
}

pub async fn create_category(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CategoryUpsertRequest>,
) -> Result<(StatusCode, Json<ApiResponse<CategoryItem>>), StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    let category = state.store.create_category(payload).await;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok("category created", category))))
}

pub async fn update_category(
    Path(name): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CategoryUpsertRequest>,
) -> Result<Json<ApiResponse<CategoryItem>>, StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    state
        .store
        .update_category(name, payload)
        .await
        .map(|item| Json(ApiResponse::ok("category updated", item)))
        .map_err(map_store_error)
}

pub async fn delete_category(
    Path(name): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DeletedResource>>, StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    state
        .store
        .delete_category(name)
        .await
        .map(|item| Json(ApiResponse::ok("category deleted", item)))
        .map_err(map_store_error)
}

fn map_store_error(error: StoreMutationError) -> StatusCode {
    match error {
        StoreMutationError::NotFound => StatusCode::NOT_FOUND,
        StoreMutationError::Conflict => StatusCode::CONFLICT,
    }
}
