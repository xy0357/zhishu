use axum::{extract::State, http::{HeaderMap, StatusCode}, Json};

use crate::{
    app::AppState,
    models::{ApiResponse, AskQuestionRequest, QaAnswer, QuestionHistoryItem},
    security::require_user,
};

pub async fn ask_question(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AskQuestionRequest>,
) -> Result<Json<ApiResponse<QaAnswer>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    let answer = state.store.ask_question(user.user_id, payload.question_text).await;
    Ok(Json(ApiResponse::ok("qa answered", answer)))
}

pub async fn list_question_history(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ApiResponse<Vec<QuestionHistoryItem>>>, StatusCode> {
    let user = require_user(&headers, &state).await?;
    Ok(Json(ApiResponse::ok(
        "question history",
        state.store.list_question_history(user.user_id).await,
    )))
}
