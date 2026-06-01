use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    app::AppState,
    models::{ApiResponse, DeletedResource, TagItem, TagUpsertRequest},
    security::{require_taxonomy_manager, require_user},
    store::StoreMutationError,
};

pub async fn list_tags(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<TagItem>>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("tags", state.store.list_tags().await)))
}

pub async fn create_tag(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TagUpsertRequest>,
) -> Result<(StatusCode, Json<ApiResponse<TagItem>>), StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    let tag = state.store.create_tag(payload).await;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok("tag created", tag))))
}

pub async fn update_tag(
    Path(name): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<TagUpsertRequest>,
) -> Result<Json<ApiResponse<TagItem>>, StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    state
        .store
        .update_tag(name, payload)
        .await
        .map(|item| Json(ApiResponse::ok("tag updated", item)))
        .map_err(map_store_error)
}

pub async fn delete_tag(
    Path(name): Path<String>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DeletedResource>>, StatusCode> {
    let _ = require_taxonomy_manager(&headers, &state).await?;
    state
        .store
        .delete_tag(name)
        .await
        .map(|item| Json(ApiResponse::ok("tag deleted", item)))
        .map_err(map_store_error)
}

fn map_store_error(error: StoreMutationError) -> StatusCode {
    match error {
        StoreMutationError::NotFound => StatusCode::NOT_FOUND,
        StoreMutationError::Conflict => StatusCode::CONFLICT,
    }
}
