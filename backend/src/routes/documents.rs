use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::{
    app::AppState,
    models::{
        ApiResponse, CreateDocumentRequest, DocumentDetail, DocumentListItem, DocumentVersion,
        UpdateDocumentRequest,
    },
    security::{require_content_manager, require_user},
};

pub async fn list_documents(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<DocumentListItem>>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok(
        "document list",
        state.store.list_documents(user.user_id).await,
    )))
}

pub async fn get_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DocumentDetail>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    state
        .store
        .get_document(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document detail", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn list_versions(
    Path(id): Path<u64>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<DocumentVersion>>>, StatusCode> {
    state
        .store
        .get_versions(id)
        .await
        .map(|items| Json(ApiResponse::ok("document versions", items)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn create_document(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateDocumentRequest>,
) -> Result<(StatusCode, Json<ApiResponse<DocumentDetail>>), StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    let detail = state.store.create_document(user.user_id, payload).await;
    Ok((StatusCode::CREATED, Json(ApiResponse::ok("document created", detail))))
}

pub async fn update_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UpdateDocumentRequest>,
) -> Result<Json<ApiResponse<DocumentDetail>>, StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    state
        .store
        .update_document(user.user_id, id, payload)
        .await
        .map(|item| Json(ApiResponse::ok("document updated", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn publish_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DocumentDetail>>, StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    state
        .store
        .publish_document(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document published", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn archive_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DocumentDetail>>, StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    state
        .store
        .archive_document(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document archived", item)))
        .ok_or(StatusCode::NOT_FOUND)
}
