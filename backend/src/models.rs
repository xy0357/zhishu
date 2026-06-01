use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: T,
}

impl<T> ApiResponse<T> {
    pub fn ok(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub total_documents: usize,
    pub published_documents: usize,
    pub total_questions: usize,
    pub total_agent_runs: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DocumentListItem {
    pub document_id: u64,
    pub title: String,
    pub summary: String,
    pub category_name: String,
    pub status: String,
    pub version_no: String,
    #[serde(default)]
    pub is_favorite: bool,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DocumentVersion {
    pub version_id: u64,
    pub version_no: String,
    pub title: String,
    pub content: String,
    pub summary: String,
    pub change_note: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DocumentDetail {
    pub document_id: u64,
    pub title: String,
    pub summary: String,
    pub content: String,
    pub category_name: String,
    pub status: String,
    pub version_no: String,
    #[serde(default)]
    pub is_favorite: bool,
    pub tags: Vec<String>,
    pub versions: Vec<DocumentVersion>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub category_name: String,
    pub tags: Vec<String>,
    pub change_note: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateDocumentRequest {
    pub title: String,
    pub summary: String,
    pub content: String,
    pub category_name: String,
    pub tags: Vec<String>,
    pub change_note: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Citation {
    pub cite_order: u32,
    pub document_title: String,
    pub version_no: String,
    pub snippet_text: String,
    pub score: f32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QaAnswer {
    pub answer_id: u64,
    pub answer_text: String,
    pub confidence_score: f32,
    pub citations: Vec<Citation>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AskQuestionRequest {
    pub question_text: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AgentRun {
    pub run_id: u64,
    pub agent_type: String,
    pub trigger_type: String,
    pub status: String,
    pub input_text: String,
    pub output_text: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct QuestionHistoryItem {
    pub question_id: u64,
    pub question_text: String,
    pub answer_preview: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CategoryItem {
    pub category_name: String,
    pub description: String,
    pub document_count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CategoryUpsertRequest {
    pub category_name: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TagItem {
    pub tag_name: String,
    pub description: String,
    pub document_count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TagUpsertRequest {
    pub tag_name: String,
    pub description: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FaqItem {
    pub faq_id: u64,
    pub document_id: u64,
    pub question: String,
    pub answer: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FaqUpsertRequest {
    pub question: String,
    pub answer: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserItem {
    pub user_id: u64,
    pub username: String,
    pub role_name: String,
    pub department: String,
    pub email: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoleItem {
    pub role_name: String,
    pub description: String,
    pub user_count: usize,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserCreateRequest {
    pub username: String,
    pub role_name: String,
    pub department: String,
    pub email: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserUpdateRequest {
    pub username: String,
    pub role_name: String,
    pub department: String,
    pub email: String,
    pub password: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub token_type: String,
    pub user: UserItem,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FavoriteDocumentItem {
    pub document_id: u64,
    pub title: String,
    pub category_name: String,
    pub status: String,
    pub version_no: String,
    pub favorite_time: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ReadRecordItem {
    pub read_id: u64,
    pub document_id: u64,
    pub title: String,
    pub category_name: String,
    pub status: String,
    pub version_no: String,
    pub read_time: DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct FavoriteState {
    pub document_id: u64,
    pub is_favorite: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DeletedResource {
    pub resource_type: String,
    pub resource_key: String,
}
