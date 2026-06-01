use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    app::AppState,
    models::{ApiResponse, FavoriteDocumentItem, FavoriteState, ReadRecordItem},
    security::require_user,
};

pub async fn list_favorites(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<FavoriteDocumentItem>>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok(
        "favorite documents",
        state.store.list_favorite_documents(user.user_id).await,
    )))
}

pub async fn list_recent_reads(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<ReadRecordItem>>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok(
        "recent read records",
        state.store.list_recent_reads(user.user_id).await,
    )))
}

pub async fn record_document_read(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<ReadRecordItem>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    state
        .store
        .record_document_read(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document read recorded", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn toggle_favorite_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<FavoriteState>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    state
        .store
        .toggle_favorite_document(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document favorite toggled", item)))
        .ok_or(StatusCode::NOT_FOUND)
}
