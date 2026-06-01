use async_trait::async_trait;
use std::sync::Arc;

use crate::models::{
    AgentRun, CategoryItem, CategoryUpsertRequest, CreateDocumentRequest,
    DashboardSummary, DeletedResource, DocumentDetail, DocumentListItem, DocumentVersion, FaqItem,
    FaqUpsertRequest, FavoriteDocumentItem, FavoriteState, QaAnswer, QuestionHistoryItem,
    ReadRecordItem, RoleItem, TagItem, TagUpsertRequest, UpdateDocumentRequest, UserCreateRequest,
    UserItem, UserUpdateRequest,
};

pub mod memory;
pub mod mysql;

pub type DynStore = Arc<dyn AppStore>;

#[derive(Debug)]
pub enum StoreMutationError {
    NotFound,
    Conflict,
}

#[async_trait]
pub trait AppStore: Send + Sync {
    async fn dashboard_summary(&self) -> DashboardSummary;
    async fn authenticate(&self, username: String, password: String) -> Option<UserItem>;
    async fn get_user_by_username(&self, username: String) -> Option<UserItem>;
    async fn list_documents(&self, user_id: u64) -> Vec<DocumentListItem>;
    async fn get_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail>;
    async fn get_versions(&self, id: u64) -> Option<Vec<DocumentVersion>>;
    async fn create_document(&self, user_id: u64, payload: CreateDocumentRequest) -> DocumentDetail;
    async fn update_document(
        &self,
        user_id: u64,
        id: u64,
        payload: UpdateDocumentRequest,
    ) -> Option<DocumentDetail>;
    async fn publish_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail>;
    async fn archive_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail>;
    async fn ask_question(&self, user_id: u64, question_text: String) -> QaAnswer;
    async fn list_question_history(&self, user_id: u64) -> Vec<QuestionHistoryItem>;
    async fn list_agent_runs(&self) -> Vec<AgentRun>;
    async fn list_categories(&self) -> Vec<CategoryItem>;
    async fn create_category(&self, payload: CategoryUpsertRequest) -> CategoryItem;
    async fn update_category(
        &self,
        current_name: String,
        payload: CategoryUpsertRequest,
    ) -> Result<CategoryItem, StoreMutationError>;
    async fn delete_category(&self, category_name: String)
        -> Result<DeletedResource, StoreMutationError>;
    async fn list_tags(&self) -> Vec<TagItem>;
    async fn create_tag(&self, payload: TagUpsertRequest) -> TagItem;
    async fn update_tag(
        &self,
        current_name: String,
        payload: TagUpsertRequest,
    ) -> Result<TagItem, StoreMutationError>;
    async fn delete_tag(&self, tag_name: String) -> Result<DeletedResource, StoreMutationError>;
    async fn list_faq_items(&self, document_id: u64) -> Vec<FaqItem>;
    async fn create_faq(
        &self,
        document_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError>;
    async fn update_faq(
        &self,
        faq_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError>;
    async fn delete_faq(&self, faq_id: u64) -> Result<DeletedResource, StoreMutationError>;
    async fn list_roles(&self) -> Vec<RoleItem>;
    async fn list_users(&self) -> Vec<UserItem>;
    async fn create_user(&self, payload: UserCreateRequest) -> Result<UserItem, StoreMutationError>;
    async fn update_user(
        &self,
        user_id: u64,
        payload: UserUpdateRequest,
    ) -> Result<UserItem, StoreMutationError>;
    async fn list_favorite_documents(&self, user_id: u64) -> Vec<FavoriteDocumentItem>;
    async fn list_recent_reads(&self, user_id: u64) -> Vec<ReadRecordItem>;
    async fn record_document_read(&self, user_id: u64, id: u64) -> Option<ReadRecordItem>;
    async fn toggle_favorite_document(&self, user_id: u64, id: u64) -> Option<FavoriteState>;
}
