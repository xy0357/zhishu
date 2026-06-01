use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    app::AppState,
    models::{ApiResponse, DeletedResource, FaqItem, FaqUpsertRequest},
    security::{require_content_manager, require_user},
    store::StoreMutationError,
};

pub async fn list_document_faq(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<FaqItem>>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok("document faqs", state.store.list_faq_items(id).await)))
}

pub async fn create_document_faq(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FaqUpsertRequest>,
) -> Result<(StatusCode, Json<ApiResponse<FaqItem>>), StatusCode> {
    let _ = require_content_manager(&headers, &state).await?;
    state
        .store
        .create_faq(id, payload)
        .await
        .map(|item| (StatusCode::CREATED, Json(ApiResponse::ok("faq created", item))))
        .map_err(map_store_error)
}

pub async fn update_faq(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<FaqUpsertRequest>,
) -> Result<Json<ApiResponse<FaqItem>>, StatusCode> {
    let _ = require_content_manager(&headers, &state).await?;
    state
        .store
        .update_faq(id, payload)
        .await
        .map(|item| Json(ApiResponse::ok("faq updated", item)))
        .map_err(map_store_error)
}

pub async fn delete_faq(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DeletedResource>>, StatusCode> {
    let _ = require_content_manager(&headers, &state).await?;
    state
        .store
        .delete_faq(id)
        .await
        .map(|item| Json(ApiResponse::ok("faq deleted", item)))
        .map_err(map_store_error)
}

fn map_store_error(error: StoreMutationError) -> StatusCode {
    match error {
        StoreMutationError::NotFound => StatusCode::NOT_FOUND,
        StoreMutationError::Conflict => StatusCode::CONFLICT,
    }
}
