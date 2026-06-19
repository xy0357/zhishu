use axum::{
    extract::{Path, State},
    http::header,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};

use crate::{
    app::AppState,
    models::{
        ApiResponse, CreateDocumentRequest, DocumentDetail, DocumentFileMeta, DocumentListItem,
        DocumentSegment, DocumentVersion, RegisterDocumentFileRequest, UpdateDocumentRequest,
        UploadDocumentFileRequest,
    },
    object_storage,
    security::{require_content_manager, require_user},
    store::StoreMutationError,
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

pub async fn list_document_segments(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<DocumentSegment>>>, StatusCode> {
    let _user = require_user(&headers, &state).await?;
    state
        .store
        .list_document_segments(id)
        .await
        .map(|items| Json(ApiResponse::ok("document segments", items)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn list_document_files(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<DocumentFileMeta>>>, StatusCode> {
    let _user = require_content_manager(&headers, &state).await?;
    Ok(Json(ApiResponse::ok(
        "document files",
        state.store.list_document_files().await,
    )))
}

pub async fn get_document_file(
    Path(file_id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DocumentFileMeta>>, StatusCode> {
    let _user = require_content_manager(&headers, &state).await?;
    state
        .store
        .get_document_file(file_id)
        .await
        .map(|item| Json(ApiResponse::ok("document file detail", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

pub async fn register_document_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<RegisterDocumentFileRequest>,
) -> Result<(StatusCode, Json<ApiResponse<DocumentFileMeta>>), StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    state
        .store
        .register_document_file(user.user_id, payload)
        .await
        .map(|item| (StatusCode::CREATED, Json(ApiResponse::ok("document file registered", item))))
        .map_err(map_store_error)
}

pub async fn upload_document_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<UploadDocumentFileRequest>,
) -> Result<(StatusCode, Json<ApiResponse<DocumentFileMeta>>), StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    let bytes = STANDARD
        .decode(payload.content_base64.as_bytes())
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let object_key = object_storage::build_object_key(&state.config.minio_bucket, &payload.original_name);
    object_storage::store_bytes(&state.config.object_storage_dir, &object_key, &bytes)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state
        .store
        .register_document_file(
            user.user_id,
            RegisterDocumentFileRequest {
                object_key: Some(object_key),
                original_name: payload.original_name,
                mime_type: payload.mime_type,
                file_size: bytes.len() as u64,
                sha256: Some(sha256),
            },
        )
        .await
        .map(|item| (StatusCode::CREATED, Json(ApiResponse::ok("document file uploaded", item))))
        .map_err(map_store_error)
}

pub async fn download_document_file(
    Path(file_id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, StatusCode> {
    let _user = require_content_manager(&headers, &state).await?;
    let file = state
        .store
        .get_document_file(file_id)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;
    let bytes = object_storage::read_bytes(&state.config.object_storage_dir, &file.object_key)
        .map_err(|_| StatusCode::NOT_FOUND)?;

    Ok((
        [
            (header::CONTENT_TYPE, file.mime_type),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{}\"", file.original_name),
            ),
        ],
        bytes,
    ))
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

pub async fn reindex_document(
    Path(id): Path<u64>,
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<DocumentDetail>>, StatusCode> {
    let user = require_content_manager(&headers, &state).await?;
    state
        .store
        .reindex_document(user.user_id, id)
        .await
        .map(|item| Json(ApiResponse::ok("document reindexed", item)))
        .ok_or(StatusCode::NOT_FOUND)
}

fn map_store_error(error: StoreMutationError) -> StatusCode {
    match error {
        StoreMutationError::NotFound => StatusCode::NOT_FOUND,
        StoreMutationError::Conflict => StatusCode::CONFLICT,
    }
}
