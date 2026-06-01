use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::models::{
    AgentRun, CategoryItem, CategoryUpsertRequest, Citation, CreateDocumentRequest,
    DashboardSummary, DeletedResource, DocumentDetail, DocumentListItem, DocumentVersion, FaqItem,
    FaqUpsertRequest, FavoriteDocumentItem, FavoriteState, QaAnswer, QuestionHistoryItem,
    ReadRecordItem, RoleItem, TagItem, TagUpsertRequest, UpdateDocumentRequest,
    UserCreateRequest, UserItem, UserUpdateRequest,
};
use crate::security::{
    hash_password, password_needs_rehash, verify_password, ADMIN_PASSWORD, ADMIN_USERNAME,
    EDITOR_PASSWORD, EDITOR_USERNAME,
};
use crate::store::{AppStore, StoreMutationError};

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
struct StoreData {
    documents: Vec<DocumentDetail>,
    managed_categories: Vec<CategoryItem>,
    managed_tags: Vec<TagItem>,
    managed_roles: Vec<RoleDefinition>,
    users: Vec<UserItem>,
    user_credentials: Vec<UserCredential>,
    faq_items: Vec<FaqItem>,
    favorite_documents: Vec<FavoriteRecord>,
    read_records: Vec<ReadRecordEntry>,
    agent_runs: Vec<AgentRun>,
    question_history: Vec<QuestionHistoryItem>,
    next_document_id: u64,
    next_user_id: u64,
    next_version_id: u64,
    next_faq_id: u64,
    next_read_id: u64,
    next_question_id: u64,
    next_answer_id: u64,
    next_run_id: u64,
}

#[derive(Clone, Serialize, Deserialize)]
struct FavoriteRecord {
    user_id: u64,
    document_id: u64,
    favorite_time: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
struct ReadRecordEntry {
    read_id: u64,
    user_id: u64,
    document_id: u64,
    read_time: chrono::DateTime<Utc>,
}

#[derive(Clone, Serialize, Deserialize)]
struct RoleDefinition {
    role_name: String,
    description: String,
}

#[derive(Clone, Serialize, Deserialize)]
struct UserCredential {
    user_id: u64,
    username: String,
    password: String,
}

pub struct MemoryStore {
    inner: Mutex<StoreData>,
    storage_path: PathBuf,
}

impl MemoryStore {
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        let storage_path = path.into();
        if let Some(parent) = storage_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let data = fs::read_to_string(&storage_path)
            .ok()
            .and_then(|content| serde_json::from_str::<StoreData>(&content).ok())
            .map(|mut data| {
                Self::ensure_demo_defaults(&mut data);
                data
            })
            .unwrap_or_else(Self::build_demo_data);

        let store = Self {
            inner: Mutex::new(data),
            storage_path,
        };

        store.persist();
        store
    }

    fn build_demo_data() -> StoreData {
        let now = Utc::now();
        let version = DocumentVersion {
            version_id: 1,
            version_no: "v1.0.0".to_string(),
            title: "数据库权限申请流程".to_string(),
            content: "1. 提交权限申请单。\n2. 直属主管审批。\n3. DBA 审核后开通只读账号。".to_string(),
            summary: "用于说明企业内部数据库权限申请的标准流程。".to_string(),
            change_note: "初始版本".to_string(),
            created_at: now,
        };

        let document = DocumentDetail {
            document_id: 1,
            title: "数据库权限申请流程".to_string(),
            summary: version.summary.clone(),
            content: version.content.clone(),
            category_name: "制度流程".to_string(),
            status: "published".to_string(),
            version_no: version.version_no.clone(),
            is_favorite: false,
            tags: vec!["数据库".to_string(), "权限".to_string(), "流程".to_string()],
            versions: vec![version],
        };

        let user = UserItem {
            user_id: 1,
            username: "admin".to_string(),
            role_name: "系统管理员".to_string(),
            department: "IT".to_string(),
            email: "admin@example.com".to_string(),
        };
        let editor = UserItem {
            user_id: 2,
            username: "editor".to_string(),
            role_name: "知识管理员".to_string(),
            department: "知识运营".to_string(),
            email: "editor@example.com".to_string(),
        };

        let run = AgentRun {
            run_id: 1,
            agent_type: "summary".to_string(),
            trigger_type: "document_publish".to_string(),
            status: "success".to_string(),
            input_text: "数据库权限申请流程".to_string(),
            output_text: "摘要已生成".to_string(),
            started_at: now,
        };

        let faq = FaqItem {
            faq_id: 1,
            document_id: 1,
            question: "数据库权限如何申请？".to_string(),
            answer: "提交权限申请单，经直属主管审批后，由 DBA 审核并开通只读账号。".to_string(),
            created_at: now,
        };

        StoreData {
            documents: vec![document],
            managed_roles: vec![
                RoleDefinition {
                    role_name: "系统管理员".to_string(),
                    description: "平台超级管理员".to_string(),
                },
                RoleDefinition {
                    role_name: "知识管理员".to_string(),
                    description: "知识内容与FAQ维护人员".to_string(),
                },
                RoleDefinition {
                    role_name: "普通用户".to_string(),
                    description: "普通知识使用者".to_string(),
                },
            ],
            managed_categories: vec![CategoryItem {
                category_name: "制度流程".to_string(),
                description: "制度流程相关知识分类".to_string(),
                document_count: 1,
            }],
            managed_tags: vec![
                TagItem {
                    tag_name: "数据库".to_string(),
                    description: "数据库主题标签".to_string(),
                    document_count: 1,
                },
                TagItem {
                    tag_name: "权限".to_string(),
                    description: "权限主题标签".to_string(),
                    document_count: 1,
                },
                TagItem {
                    tag_name: "流程".to_string(),
                    description: "流程主题标签".to_string(),
                    document_count: 1,
                },
            ],
            users: vec![user, editor],
            user_credentials: vec![
                UserCredential {
                    user_id: 1,
                    username: ADMIN_USERNAME.to_string(),
                    password: hash_password(ADMIN_PASSWORD),
                },
                UserCredential {
                    user_id: 2,
                    username: EDITOR_USERNAME.to_string(),
                    password: hash_password(EDITOR_PASSWORD),
                },
            ],
            faq_items: vec![faq],
            favorite_documents: Vec::new(),
            read_records: Vec::new(),
            agent_runs: vec![run],
            question_history: Vec::new(),
            next_document_id: 2,
            next_user_id: 3,
            next_version_id: 2,
            next_faq_id: 2,
            next_read_id: 1,
            next_question_id: 1,
            next_answer_id: 1,
            next_run_id: 2,
        }
    }

    fn persist(&self) {
        if let Ok(state) = self.inner.lock() {
            self.persist_locked(&state);
        }
    }

    fn persist_locked(&self, state: &StoreData) {
        if let Ok(content) = serde_json::to_string_pretty(state) {
            let _ = fs::write(&self.storage_path, content);
        }
    }

    fn next_version_no(version_count: usize) -> String {
        format!("v1.0.{}", version_count.saturating_sub(1))
    }

    fn first_line(text: &str) -> String {
        text.lines().next().unwrap_or_default().to_string()
    }

    fn make_run(
        run_id: u64,
        agent_type: &str,
        trigger_type: &str,
        status: &str,
        input_text: String,
        output_text: String,
    ) -> AgentRun {
        AgentRun {
            run_id,
            agent_type: agent_type.to_string(),
            trigger_type: trigger_type.to_string(),
            status: status.to_string(),
            input_text,
            output_text,
            started_at: Utc::now(),
        }
    }

    fn default_faq(detail: &DocumentDetail, faq_id: u64) -> FaqItem {
        FaqItem {
            faq_id,
            document_id: detail.document_id,
            question: format!("{} 的核心流程是什么？", detail.title),
            answer: detail.summary.clone(),
            created_at: Utc::now(),
        }
    }

    fn make_favorite_item(
        _user_id: u64,
        detail: &DocumentDetail,
        favorite_time: chrono::DateTime<Utc>,
    ) -> FavoriteDocumentItem {
        FavoriteDocumentItem {
            document_id: detail.document_id,
            title: detail.title.clone(),
            category_name: detail.category_name.clone(),
            status: detail.status.clone(),
            version_no: detail.version_no.clone(),
            favorite_time,
        }
    }

    fn make_read_record(
        read_id: u64,
        detail: &DocumentDetail,
        read_time: chrono::DateTime<Utc>,
    ) -> ReadRecordItem {
        ReadRecordItem {
            read_id,
            document_id: detail.document_id,
            title: detail.title.clone(),
            category_name: detail.category_name.clone(),
            status: detail.status.clone(),
            version_no: detail.version_no.clone(),
            read_time,
        }
    }

    fn is_favorite_for_user(state: &StoreData, user_id: u64, document_id: u64) -> bool {
        state
            .favorite_documents
            .iter()
            .any(|item| item.user_id == user_id && item.document_id == document_id)
    }

    fn ensure_category_registered(state: &mut StoreData, category_name: &str) {
        if !state
            .managed_categories
            .iter()
            .any(|item| item.category_name == category_name)
        {
            state.managed_categories.push(CategoryItem {
                category_name: category_name.to_string(),
                description: format!("{} 相关知识分类", category_name),
                document_count: 0,
            });
        }
    }

    fn ensure_demo_defaults(state: &mut StoreData) {
        if state.managed_roles.is_empty() {
            state.managed_roles = vec![
                RoleDefinition {
                    role_name: "系统管理员".to_string(),
                    description: "平台超级管理员".to_string(),
                },
                RoleDefinition {
                    role_name: "知识管理员".to_string(),
                    description: "知识内容与FAQ维护人员".to_string(),
                },
                RoleDefinition {
                    role_name: "普通用户".to_string(),
                    description: "普通知识使用者".to_string(),
                },
            ];
        }

        if !state.users.iter().any(|item| item.username == ADMIN_USERNAME) {
            state.users.push(UserItem {
                user_id: 1,
                username: ADMIN_USERNAME.to_string(),
                role_name: "系统管理员".to_string(),
                department: "IT".to_string(),
                email: "admin@example.com".to_string(),
            });
        }

        if !state.users.iter().any(|item| item.username == EDITOR_USERNAME) {
            state.users.push(UserItem {
                user_id: 2,
                username: EDITOR_USERNAME.to_string(),
                role_name: "知识管理员".to_string(),
                department: "知识运营".to_string(),
                email: "editor@example.com".to_string(),
            });
        }

        if !state.user_credentials.iter().any(|item| item.username == ADMIN_USERNAME) {
            state.user_credentials.push(UserCredential {
                user_id: 1,
                username: ADMIN_USERNAME.to_string(),
                password: hash_password(ADMIN_PASSWORD),
            });
        }

        if !state.user_credentials.iter().any(|item| item.username == EDITOR_USERNAME) {
            state.user_credentials.push(UserCredential {
                user_id: 2,
                username: EDITOR_USERNAME.to_string(),
                password: hash_password(EDITOR_PASSWORD),
            });
        }

        for credential in &mut state.user_credentials {
            if password_needs_rehash(&credential.password) {
                credential.password = hash_password(&credential.password);
            }
        }

        if state.next_user_id < 3 {
            state.next_user_id = 3;
        }
    }

    fn ensure_tags_registered(state: &mut StoreData, tags: &[String]) {
        for tag in tags {
            if !state.managed_tags.iter().any(|item| item.tag_name == *tag) {
                state.managed_tags.push(TagItem {
                    tag_name: tag.clone(),
                    description: format!("{} 主题标签", tag),
                    document_count: 0,
                });
            }
        }
    }

    pub fn dashboard_summary(&self) -> DashboardSummary {
        let state = self.inner.lock().expect("memory store lock");
        DashboardSummary {
            total_documents: state.documents.len(),
            published_documents: state
                .documents
                .iter()
                .filter(|item| item.status == "published")
                .count(),
            total_questions: state.question_history.len(),
            total_agent_runs: state.agent_runs.len(),
        }
    }

    pub fn list_documents(&self, user_id: u64) -> Vec<DocumentListItem> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .documents
            .iter()
            .map(|item| DocumentListItem {
                document_id: item.document_id,
                title: item.title.clone(),
                summary: item.summary.clone(),
                category_name: item.category_name.clone(),
                status: item.status.clone(),
                version_no: item.version_no.clone(),
                is_favorite: Self::is_favorite_for_user(&state, user_id, item.document_id),
                updated_at: item
                    .versions
                    .last()
                    .map(|version| version.created_at)
                    .unwrap_or_else(Utc::now),
            })
            .collect()
    }

    pub fn get_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let state = self.inner.lock().expect("memory store lock");
        state.documents.iter().find(|item| item.document_id == id).cloned().map(|mut item| {
            item.is_favorite = Self::is_favorite_for_user(&state, user_id, id);
            item
        })
    }

    pub fn get_versions(&self, id: u64) -> Option<Vec<DocumentVersion>> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .documents
            .iter()
            .find(|item| item.document_id == id)
            .map(|item| item.versions.clone())
    }

    pub fn create_document(&self, _user_id: u64, payload: CreateDocumentRequest) -> DocumentDetail {
        let mut state = self.inner.lock().expect("memory store lock");
        Self::ensure_category_registered(&mut state, &payload.category_name);
        Self::ensure_tags_registered(&mut state, &payload.tags);
        let now = Utc::now();
        let version = DocumentVersion {
            version_id: state.next_version_id,
            version_no: "v1.0.0".to_string(),
            title: payload.title.clone(),
            content: payload.content.clone(),
            summary: payload.summary.clone(),
            change_note: payload.change_note,
            created_at: now,
        };
        state.next_version_id += 1;

        let detail = DocumentDetail {
            document_id: state.next_document_id,
            title: payload.title,
            summary: payload.summary,
            content: payload.content,
            category_name: payload.category_name,
            status: "draft".to_string(),
            version_no: "v1.0.0".to_string(),
            is_favorite: false,
            tags: payload.tags,
            versions: vec![version],
        };
        state.next_document_id += 1;
        state.documents.push(detail.clone());
        let faq_id = state.next_faq_id;
        state.next_faq_id += 1;
        state.faq_items.push(Self::default_faq(&detail, faq_id));

        let run_id = state.next_run_id;
        state.next_run_id += 1;
        state.agent_runs.push(Self::make_run(
            run_id,
            "summary",
            "manual",
            "success",
            detail.title.clone(),
            "新文档已写入并生成初始摘要".to_string(),
        ));

        self.persist_locked(&state);
        detail
    }

    pub fn update_document(&self, user_id: u64, id: u64, payload: UpdateDocumentRequest) -> Option<DocumentDetail> {
        let mut state = self.inner.lock().expect("memory store lock");
        Self::ensure_category_registered(&mut state, &payload.category_name);
        Self::ensure_tags_registered(&mut state, &payload.tags);
        let run_id = state.next_run_id;
        state.next_run_id += 1;
        let version_id = state.next_version_id;
        state.next_version_id += 1;

        let updated_clone = {
            let updated = state.documents.iter_mut().find(|item| item.document_id == id)?;
            let version_no = Self::next_version_no(updated.versions.len() + 1);
            let version = DocumentVersion {
                version_id,
                version_no: version_no.clone(),
                title: payload.title.clone(),
                content: payload.content.clone(),
                summary: payload.summary.clone(),
                change_note: payload.change_note,
                created_at: Utc::now(),
            };

            updated.title = payload.title;
            updated.summary = payload.summary;
            updated.content = payload.content;
            updated.category_name = payload.category_name;
            updated.tags = payload.tags;
            updated.version_no = version_no;
            updated.versions.push(version);
            updated.clone()
        };

        state.faq_items.retain(|item| item.document_id != id);
        let faq_id = state.next_faq_id;
        state.next_faq_id += 1;
        state.faq_items.push(Self::default_faq(&updated_clone, faq_id));

        state.agent_runs.push(Self::make_run(
            run_id,
            "summary",
            "manual",
            "success",
            updated_clone.title.clone(),
            "文档已更新并生成新版本".to_string(),
        ));

        self.persist_locked(&state);
        drop(state);
        self.get_document(user_id, updated_clone.document_id)
    }

    pub fn publish_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let mut state = self.inner.lock().expect("memory store lock");
        let run_id = state.next_run_id;
        state.next_run_id += 1;

        let updated_clone = {
            let updated = state.documents.iter_mut().find(|item| item.document_id == id)?;
            updated.status = "published".to_string();
            updated.clone()
        };

        state.agent_runs.push(Self::make_run(
            run_id,
            "audit",
            "document_publish",
            "success",
            updated_clone.title.clone(),
            "文档发布成功".to_string(),
        ));

        self.persist_locked(&state);
        drop(state);
        self.get_document(user_id, updated_clone.document_id)
    }

    pub fn archive_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let mut state = self.inner.lock().expect("memory store lock");
        let run_id = state.next_run_id;
        state.next_run_id += 1;

        let archived_clone = {
            let archived = state.documents.iter_mut().find(|item| item.document_id == id)?;
            archived.status = "archived".to_string();
            archived.clone()
        };

        state.agent_runs.push(Self::make_run(
            run_id,
            "audit",
            "manual",
            "success",
            archived_clone.title.clone(),
            "文档已归档".to_string(),
        ));

        self.persist_locked(&state);
        drop(state);
        self.get_document(user_id, archived_clone.document_id)
    }

    pub fn ask_question(&self, user_id: u64, question_text: String) -> QaAnswer {
        let mut state = self.inner.lock().expect("memory store lock");
        let question_id = state.next_question_id;
        state.next_question_id += 1;

        let (doc_title, version_no, snippet_text) = state
            .documents
            .iter()
            .find(|doc| doc.status != "archived")
            .map(|doc| {
                (
                    doc.title.clone(),
                    doc.version_no.clone(),
                    Self::first_line(&doc.content),
                )
            })
            .unwrap_or_else(|| ("暂无文档".to_string(), "v0".to_string(), "暂无证据".to_string()));

        let answer_text = format!(
            "根据当前知识库，关于“{}”的建议处理方式是：先走标准申请流程，再由对应管理员审核开通。",
            question_text
        );

        let answer = QaAnswer {
            answer_id: state.next_answer_id,
            answer_text: answer_text.clone(),
            confidence_score: 0.88,
            citations: vec![Citation {
                cite_order: 1,
                document_title: doc_title,
                version_no,
                snippet_text,
                score: 0.92,
            }],
            created_at: Utc::now(),
        };
        state.next_answer_id += 1;
        state.question_history.push(QuestionHistoryItem {
            question_id,
            question_text: question_text.clone(),
            answer_preview: format!("U{} {}", user_id, answer_text.chars().take(42).collect::<String>()),
            created_at: answer.created_at,
        });

        let run_id = state.next_run_id;
        state.next_run_id += 1;
        state.agent_runs.push(Self::make_run(
            run_id,
            "answer",
            "question_submit",
            "success",
            question_text,
            answer.answer_text.clone(),
        ));

        self.persist_locked(&state);
        answer
    }

    pub fn list_question_history(&self, user_id: u64) -> Vec<QuestionHistoryItem> {
        let state = self.inner.lock().expect("memory store lock");
        let prefix = format!("U{} ", user_id);
        state
            .question_history
            .iter()
            .rev()
            .filter(|item| item.answer_preview.starts_with(&prefix))
            .map(|item| {
                let mut cloned = item.clone();
                cloned.answer_preview = cloned.answer_preview.replacen(&prefix, "", 1);
                cloned
            })
            .collect()
    }

    pub fn list_agent_runs(&self) -> Vec<AgentRun> {
        let state = self.inner.lock().expect("memory store lock");
        state.agent_runs.iter().rev().cloned().collect()
    }

    pub fn list_categories(&self) -> Vec<CategoryItem> {
        let state = self.inner.lock().expect("memory store lock");
        let mut counts = BTreeMap::<String, usize>::new();
        for doc in &state.documents {
            *counts.entry(doc.category_name.clone()).or_default() += 1;
        }

        let mut descriptions = BTreeMap::<String, String>::new();
        for item in &state.managed_categories {
            descriptions.insert(item.category_name.clone(), item.description.clone());
        }
        for name in counts.keys() {
            descriptions
                .entry(name.clone())
                .or_insert_with(|| format!("{} 相关知识分类", name));
        }

        descriptions
            .into_iter()
            .map(|(category_name, description)| CategoryItem {
                document_count: counts.get(&category_name).copied().unwrap_or(0),
                description,
                category_name,
            })
            .collect()
    }

    pub fn list_tags(&self) -> Vec<TagItem> {
        let state = self.inner.lock().expect("memory store lock");
        let mut counts = BTreeMap::<String, usize>::new();
        for doc in &state.documents {
            for tag in &doc.tags {
                *counts.entry(tag.clone()).or_default() += 1;
            }
        }

        let mut descriptions = BTreeMap::<String, String>::new();
        for item in &state.managed_tags {
            descriptions.insert(item.tag_name.clone(), item.description.clone());
        }
        for name in counts.keys() {
            descriptions
                .entry(name.clone())
                .or_insert_with(|| format!("{} 主题标签", name));
        }

        descriptions
            .into_iter()
            .map(|(tag_name, description)| TagItem {
                document_count: counts.get(&tag_name).copied().unwrap_or(0),
                description,
                tag_name,
            })
            .collect()
    }

    pub fn list_faq_items(&self, document_id: u64) -> Vec<FaqItem> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .faq_items
            .iter()
            .filter(|item| item.document_id == document_id)
            .cloned()
            .collect()
    }

    pub fn create_category(&self, payload: CategoryUpsertRequest) -> CategoryItem {
        let mut state = self.inner.lock().expect("memory store lock");
        if let Some(existing) = state
            .managed_categories
            .iter_mut()
            .find(|item| item.category_name == payload.category_name)
        {
            existing.description = payload.description;
        } else {
            state.managed_categories.push(CategoryItem {
                category_name: payload.category_name.clone(),
                description: payload.description.clone(),
                document_count: 0,
            });
        }
        self.persist_locked(&state);
        drop(state);
        self.list_categories()
            .into_iter()
            .find(|item| item.category_name == payload.category_name)
            .expect("created category")
    }

    pub fn update_category(
        &self,
        current_name: String,
        payload: CategoryUpsertRequest,
    ) -> Result<CategoryItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        let Some(index) = state
            .managed_categories
            .iter()
            .position(|item| item.category_name == current_name)
        else {
            return Err(StoreMutationError::NotFound);
        };

        if current_name != payload.category_name
            && state
                .managed_categories
                .iter()
                .any(|item| item.category_name == payload.category_name)
        {
            return Err(StoreMutationError::Conflict);
        }

        state.managed_categories[index].category_name = payload.category_name.clone();
        state.managed_categories[index].description = payload.description.clone();

        for document in &mut state.documents {
            if document.category_name == current_name {
                document.category_name = payload.category_name.clone();
            }
        }

        self.persist_locked(&state);
        drop(state);
        self.list_categories()
            .into_iter()
            .find(|item| item.category_name == payload.category_name)
            .ok_or(StoreMutationError::NotFound)
    }

    pub fn delete_category(
        &self,
        category_name: String,
    ) -> Result<DeletedResource, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        if state.documents.iter().any(|item| item.category_name == category_name) {
            return Err(StoreMutationError::Conflict);
        }

        let before = state.managed_categories.len();
        state
            .managed_categories
            .retain(|item| item.category_name != category_name);
        if before == state.managed_categories.len() {
            return Err(StoreMutationError::NotFound);
        }

        self.persist_locked(&state);
        Ok(DeletedResource {
            resource_type: "category".to_string(),
            resource_key: category_name,
        })
    }

    pub fn create_tag(&self, payload: TagUpsertRequest) -> TagItem {
        let mut state = self.inner.lock().expect("memory store lock");
        if let Some(existing) = state
            .managed_tags
            .iter_mut()
            .find(|item| item.tag_name == payload.tag_name)
        {
            existing.description = payload.description;
        } else {
            state.managed_tags.push(TagItem {
                tag_name: payload.tag_name.clone(),
                description: payload.description.clone(),
                document_count: 0,
            });
        }
        self.persist_locked(&state);
        drop(state);
        self.list_tags()
            .into_iter()
            .find(|item| item.tag_name == payload.tag_name)
            .expect("created tag")
    }

    pub fn update_tag(
        &self,
        current_name: String,
        payload: TagUpsertRequest,
    ) -> Result<TagItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        let Some(index) = state
            .managed_tags
            .iter()
            .position(|item| item.tag_name == current_name)
        else {
            return Err(StoreMutationError::NotFound);
        };

        if current_name != payload.tag_name
            && state
                .managed_tags
                .iter()
                .any(|item| item.tag_name == payload.tag_name)
        {
            return Err(StoreMutationError::Conflict);
        }

        state.managed_tags[index].tag_name = payload.tag_name.clone();
        state.managed_tags[index].description = payload.description.clone();

        for document in &mut state.documents {
            for tag in &mut document.tags {
                if *tag == current_name {
                    *tag = payload.tag_name.clone();
                }
            }
        }

        self.persist_locked(&state);
        drop(state);
        self.list_tags()
            .into_iter()
            .find(|item| item.tag_name == payload.tag_name)
            .ok_or(StoreMutationError::NotFound)
    }

    pub fn delete_tag(&self, tag_name: String) -> Result<DeletedResource, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        let before = state.managed_tags.len();
        state.managed_tags.retain(|item| item.tag_name != tag_name);
        if before == state.managed_tags.len() {
            return Err(StoreMutationError::NotFound);
        }

        for document in &mut state.documents {
            document.tags.retain(|item| item != &tag_name);
        }

        self.persist_locked(&state);
        Ok(DeletedResource {
            resource_type: "tag".to_string(),
            resource_key: tag_name,
        })
    }

    pub fn create_faq(
        &self,
        document_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        if !state.documents.iter().any(|item| item.document_id == document_id) {
            return Err(StoreMutationError::NotFound);
        }

        let faq = FaqItem {
            faq_id: state.next_faq_id,
            document_id,
            question: payload.question,
            answer: payload.answer,
            created_at: Utc::now(),
        };
        state.next_faq_id += 1;
        state.faq_items.push(faq.clone());
        self.persist_locked(&state);
        Ok(faq)
    }

    pub fn update_faq(
        &self,
        faq_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        let Some(existing) = state.faq_items.iter_mut().find(|item| item.faq_id == faq_id) else {
            return Err(StoreMutationError::NotFound);
        };

        existing.question = payload.question;
        existing.answer = payload.answer;
        let updated = existing.clone();
        self.persist_locked(&state);
        Ok(updated)
    }

    pub fn delete_faq(&self, faq_id: u64) -> Result<DeletedResource, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        let before = state.faq_items.len();
        state.faq_items.retain(|item| item.faq_id != faq_id);
        if before == state.faq_items.len() {
            return Err(StoreMutationError::NotFound);
        }

        self.persist_locked(&state);
        Ok(DeletedResource {
            resource_type: "faq".to_string(),
            resource_key: faq_id.to_string(),
        })
    }

    pub fn list_users(&self) -> Vec<UserItem> {
        let state = self.inner.lock().expect("memory store lock");
        state.users.clone()
    }

    pub fn list_roles(&self) -> Vec<RoleItem> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .managed_roles
            .iter()
            .map(|role| RoleItem {
                role_name: role.role_name.clone(),
                description: role.description.clone(),
                user_count: state
                    .users
                    .iter()
                    .filter(|user| user.role_name == role.role_name)
                    .count(),
            })
            .collect()
    }

    pub fn create_user(
        &self,
        payload: UserCreateRequest,
    ) -> Result<UserItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        if state.users.iter().any(|item| item.username == payload.username) {
            return Err(StoreMutationError::Conflict);
        }
        if !state
            .managed_roles
            .iter()
            .any(|role| role.role_name == payload.role_name)
        {
            return Err(StoreMutationError::NotFound);
        }

        let user = UserItem {
            user_id: state.next_user_id,
            username: payload.username.clone(),
            role_name: payload.role_name,
            department: payload.department,
            email: payload.email,
        };
        state.next_user_id += 1;
        state.users.push(user.clone());
        state.user_credentials.push(UserCredential {
            user_id: user.user_id,
            username: user.username.clone(),
            password: hash_password(&payload.password),
        });
        self.persist_locked(&state);
        Ok(user)
    }

    pub fn update_user(
        &self,
        user_id: u64,
        payload: UserUpdateRequest,
    ) -> Result<UserItem, StoreMutationError> {
        let mut state = self.inner.lock().expect("memory store lock");
        if user_id == 1 && payload.username != ADMIN_USERNAME {
            return Err(StoreMutationError::Conflict);
        }
        if !state
            .managed_roles
            .iter()
            .any(|role| role.role_name == payload.role_name)
        {
            return Err(StoreMutationError::NotFound);
        }

        if state
            .users
            .iter()
            .any(|item| item.user_id != user_id && item.username == payload.username)
        {
            return Err(StoreMutationError::Conflict);
        }

        let Some(user) = state.users.iter_mut().find(|item| item.user_id == user_id) else {
            return Err(StoreMutationError::NotFound);
        };

        user.username = payload.username.clone();
        user.role_name = payload.role_name;
        user.department = payload.department;
        user.email = payload.email;
        let updated = user.clone();

        if let Some(credential) = state
            .user_credentials
            .iter_mut()
            .find(|item| item.user_id == user_id)
        {
            credential.username = payload.username;
            if let Some(password) = payload.password.filter(|value| !value.is_empty()) {
                credential.password = hash_password(&password);
            }
        }

        self.persist_locked(&state);
        Ok(updated)
    }

    pub fn list_favorite_documents(&self, user_id: u64) -> Vec<FavoriteDocumentItem> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .favorite_documents
            .iter()
            .rev()
            .filter(|item| item.user_id == user_id)
            .filter_map(|item| {
                let detail = state.documents.iter().find(|doc| doc.document_id == item.document_id)?;
                Some(Self::make_favorite_item(user_id, detail, item.favorite_time))
            })
            .collect()
    }

    pub fn list_recent_reads(&self, user_id: u64) -> Vec<ReadRecordItem> {
        let state = self.inner.lock().expect("memory store lock");
        state
            .read_records
            .iter()
            .rev()
            .filter(|item| item.user_id == user_id)
            .take(12)
            .filter_map(|item| {
                let detail = state.documents.iter().find(|doc| doc.document_id == item.document_id)?;
                Some(Self::make_read_record(item.read_id, detail, item.read_time))
            })
            .collect()
    }

    pub fn record_document_read(&self, user_id: u64, id: u64) -> Option<ReadRecordItem> {
        let mut state = self.inner.lock().expect("memory store lock");
        let detail = state.documents.iter().find(|item| item.document_id == id)?.clone();
        let read_id = state.next_read_id;
        state.next_read_id += 1;
        let read_time = Utc::now();
        state.read_records.push(ReadRecordEntry {
            read_id,
            user_id,
            document_id: id,
            read_time,
        });
        self.persist_locked(&state);
        Some(Self::make_read_record(read_id, &detail, read_time))
    }

    pub fn toggle_favorite_document(&self, user_id: u64, id: u64) -> Option<FavoriteState> {
        let mut state = self.inner.lock().expect("memory store lock");
        let _detail = state.documents.iter().find(|item| item.document_id == id)?.clone();
        let is_already_favorite = state
            .favorite_documents
            .iter()
            .any(|item| item.user_id == user_id && item.document_id == id);

        if is_already_favorite {
            state
                .favorite_documents
                .retain(|item| !(item.user_id == user_id && item.document_id == id));
        } else {
            state.favorite_documents.push(FavoriteRecord {
                user_id,
                document_id: id,
                favorite_time: Utc::now(),
            });
        }

        let new_state = !is_already_favorite;
        self.persist_locked(&state);
        Some(FavoriteState {
            document_id: id,
            is_favorite: new_state,
        })
    }

    pub fn authenticate(&self, username: String, password: String) -> Option<UserItem> {
        let state = self.inner.lock().expect("memory store lock");
        let credential = state
            .user_credentials
            .iter()
            .find(|item| item.username == username)?;
        if !verify_password(&password, &credential.password) {
            return None;
        }
        state.users.iter().find(|item| item.username == username).cloned()
    }

    pub fn get_user_by_username(&self, username: String) -> Option<UserItem> {
        let state = self.inner.lock().expect("memory store lock");
        state.users.iter().find(|item| item.username == username).cloned()
    }

    #[allow(dead_code)]
    pub fn storage_path(&self) -> &Path {
        &self.storage_path
    }
}

#[async_trait]
impl AppStore for MemoryStore {
    async fn authenticate(&self, username: String, password: String) -> Option<UserItem> {
        Self::authenticate(self, username, password)
    }

    async fn get_user_by_username(&self, username: String) -> Option<UserItem> {
        Self::get_user_by_username(self, username)
    }

    async fn dashboard_summary(&self) -> DashboardSummary {
        Self::dashboard_summary(self)
    }

    async fn list_documents(&self, user_id: u64) -> Vec<DocumentListItem> {
        Self::list_documents(self, user_id)
    }

    async fn get_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        Self::get_document(self, user_id, id)
    }

    async fn get_versions(&self, id: u64) -> Option<Vec<DocumentVersion>> {
        Self::get_versions(self, id)
    }

    async fn create_document(&self, user_id: u64, payload: CreateDocumentRequest) -> DocumentDetail {
        Self::create_document(self, user_id, payload)
    }

    async fn update_document(&self, user_id: u64, id: u64, payload: UpdateDocumentRequest) -> Option<DocumentDetail> {
        Self::update_document(self, user_id, id, payload)
    }

    async fn publish_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        Self::publish_document(self, user_id, id)
    }

    async fn archive_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        Self::archive_document(self, user_id, id)
    }

    async fn ask_question(&self, user_id: u64, question_text: String) -> QaAnswer {
        Self::ask_question(self, user_id, question_text)
    }

    async fn list_question_history(&self, user_id: u64) -> Vec<QuestionHistoryItem> {
        Self::list_question_history(self, user_id)
    }

    async fn list_agent_runs(&self) -> Vec<AgentRun> {
        Self::list_agent_runs(self)
    }

    async fn list_categories(&self) -> Vec<CategoryItem> {
        Self::list_categories(self)
    }

    async fn create_category(&self, payload: CategoryUpsertRequest) -> CategoryItem {
        Self::create_category(self, payload)
    }

    async fn update_category(
        &self,
        current_name: String,
        payload: CategoryUpsertRequest,
    ) -> Result<CategoryItem, StoreMutationError> {
        Self::update_category(self, current_name, payload)
    }

    async fn delete_category(
        &self,
        category_name: String,
    ) -> Result<DeletedResource, StoreMutationError> {
        Self::delete_category(self, category_name)
    }

    async fn list_tags(&self) -> Vec<TagItem> {
        Self::list_tags(self)
    }

    async fn create_tag(&self, payload: TagUpsertRequest) -> TagItem {
        Self::create_tag(self, payload)
    }

    async fn update_tag(
        &self,
        current_name: String,
        payload: TagUpsertRequest,
    ) -> Result<TagItem, StoreMutationError> {
        Self::update_tag(self, current_name, payload)
    }

    async fn delete_tag(&self, tag_name: String) -> Result<DeletedResource, StoreMutationError> {
        Self::delete_tag(self, tag_name)
    }

    async fn list_faq_items(&self, document_id: u64) -> Vec<FaqItem> {
        Self::list_faq_items(self, document_id)
    }

    async fn create_faq(
        &self,
        document_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        Self::create_faq(self, document_id, payload)
    }

    async fn update_faq(
        &self,
        faq_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        Self::update_faq(self, faq_id, payload)
    }

    async fn delete_faq(&self, faq_id: u64) -> Result<DeletedResource, StoreMutationError> {
        Self::delete_faq(self, faq_id)
    }

    async fn list_roles(&self) -> Vec<RoleItem> {
        Self::list_roles(self)
    }

    async fn list_users(&self) -> Vec<UserItem> {
        Self::list_users(self)
    }

    async fn create_user(&self, payload: UserCreateRequest) -> Result<UserItem, StoreMutationError> {
        Self::create_user(self, payload)
    }

    async fn update_user(
        &self,
        user_id: u64,
        payload: UserUpdateRequest,
    ) -> Result<UserItem, StoreMutationError> {
        Self::update_user(self, user_id, payload)
    }

    async fn list_favorite_documents(&self, user_id: u64) -> Vec<FavoriteDocumentItem> {
        Self::list_favorite_documents(self, user_id)
    }

    async fn list_recent_reads(&self, user_id: u64) -> Vec<ReadRecordItem> {
        Self::list_recent_reads(self, user_id)
    }

    async fn record_document_read(&self, user_id: u64, id: u64) -> Option<ReadRecordItem> {
        Self::record_document_read(self, user_id, id)
    }

    async fn toggle_favorite_document(&self, user_id: u64, id: u64) -> Option<FavoriteState> {
        Self::toggle_favorite_document(self, user_id, id)
    }
}

#[cfg(test)]
mod tests {
    use super::MemoryStore;
    use crate::models::{
        CategoryUpsertRequest, CreateDocumentRequest, FaqUpsertRequest, TagUpsertRequest,
        UpdateDocumentRequest, UserCreateRequest, UserUpdateRequest,
    };
    use std::{env, fs, path::PathBuf};
    use uuid::Uuid;

    fn temp_store_path(test_name: &str) -> PathBuf {
        env::temp_dir().join(format!("zhishu-memory-store-{}-{}.json", test_name, Uuid::new_v4()))
    }

    fn build_create_request() -> CreateDocumentRequest {
        CreateDocumentRequest {
            title: "VPN 开通流程".to_string(),
            summary: "用于说明外网访问权限申请流程。".to_string(),
            content: "1. 提交 VPN 申请。\n2. 直属主管审批。\n3. 运维开通访问权限。".to_string(),
            category_name: "运维流程".to_string(),
            tags: vec!["VPN".to_string(), "运维".to_string()],
            change_note: "初始化版本".to_string(),
        }
    }

    #[test]
    fn create_update_publish_archive_should_form_closed_loop() {
        let path = temp_store_path("closed-loop");
        let store = MemoryStore::from_path(&path);

        let created = store.create_document(1, build_create_request());
        assert_eq!(created.document_id, 2);
        assert_eq!(created.status, "draft");
        assert_eq!(created.version_no, "v1.0.0");
        assert_eq!(store.list_faq_items(created.document_id).len(), 1);

        let updated = store
            .update_document(
                1,
                created.document_id,
                UpdateDocumentRequest {
                    title: "VPN 开通流程（修订）".to_string(),
                    summary: "补充了审批后的通知步骤。".to_string(),
                    content: "1. 提交 VPN 申请。\n2. 直属主管审批。\n3. 运维开通访问权限。\n4. 邮件通知申请人验证。".to_string(),
                    category_name: "运维流程".to_string(),
                    tags: vec!["VPN".to_string(), "远程办公".to_string()],
                    change_note: "补充验证步骤".to_string(),
                },
            )
            .expect("updated document");
        assert_eq!(updated.version_no, "v1.0.1");
        assert_eq!(updated.versions.len(), 2);
        assert_eq!(updated.tags, vec!["VPN".to_string(), "远程办公".to_string()]);

        let published = store
            .publish_document(1, created.document_id)
            .expect("published document");
        assert_eq!(published.status, "published");

        let archived = store
            .archive_document(1, created.document_id)
            .expect("archived document");
        assert_eq!(archived.status, "archived");

        let agent_runs = store.list_agent_runs();
        assert!(agent_runs.len() >= 4);
        assert_eq!(agent_runs[0].output_text, "文档已归档");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn ask_question_should_append_history_and_citation() {
        let path = temp_store_path("qa");
        let store = MemoryStore::from_path(&path);

        let answer = store.ask_question(1, "如何申请数据库权限？".to_string());
        assert_eq!(answer.answer_id, 1);
        assert_eq!(answer.citations.len(), 1);
        assert_eq!(answer.citations[0].document_title, "数据库权限申请流程");

        let history = store.list_question_history(1);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].question_text, "如何申请数据库权限？");

        let agent_runs = store.list_agent_runs();
        assert_eq!(agent_runs[0].agent_type, "answer");

        let _ = fs::remove_file(path);
    }

    #[test]
    fn favorite_and_read_behaviors_should_update_document_state() {
        let path = temp_store_path("behavior");
        let store = MemoryStore::from_path(&path);

        let favorite = store
            .toggle_favorite_document(1, 1)
            .expect("favorite state returned");
        assert!(favorite.is_favorite);
        assert_eq!(store.list_favorite_documents(1).len(), 1);
        assert!(store.get_document(1, 1).expect("document").is_favorite);

        let read = store.record_document_read(1, 1).expect("read record");
        assert_eq!(read.document_id, 1);
        assert_eq!(store.list_recent_reads(1).len(), 1);

        let unfavorite = store
            .toggle_favorite_document(1, 1)
            .expect("favorite state returned");
        assert!(!unfavorite.is_favorite);
        assert!(store.list_favorite_documents(1).is_empty());
        assert!(!store.get_document(1, 1).expect("document").is_favorite);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn management_actions_should_update_catalogs_and_faqs() {
        let path = temp_store_path("management");
        let store = MemoryStore::from_path(&path);

        let category = store.create_category(CategoryUpsertRequest {
            category_name: "安全规范".to_string(),
            description: "安全制度与基线".to_string(),
        });
        assert_eq!(category.document_count, 0);

        let renamed_category = store
            .update_category(
                "安全规范".to_string(),
                CategoryUpsertRequest {
                    category_name: "安全制度".to_string(),
                    description: "安全制度与流程".to_string(),
                },
            )
            .expect("category updated");
        assert_eq!(renamed_category.category_name, "安全制度");

        let tag = store.create_tag(TagUpsertRequest {
            tag_name: "审批".to_string(),
            description: "审批相关标签".to_string(),
        });
        assert_eq!(tag.document_count, 0);

        let renamed_tag = store
            .update_tag(
                "审批".to_string(),
                TagUpsertRequest {
                    tag_name: "审核".to_string(),
                    description: "审核相关标签".to_string(),
                },
            )
            .expect("tag updated");
        assert_eq!(renamed_tag.tag_name, "审核");

        let faq = store
            .create_faq(
                1,
                FaqUpsertRequest {
                    question: "审批完成后谁来开通？".to_string(),
                    answer: "由 DBA 完成权限开通。".to_string(),
                },
            )
            .expect("faq created");
        assert_eq!(store.list_faq_items(1).len(), 2);

        let updated_faq = store
            .update_faq(
                faq.faq_id,
                FaqUpsertRequest {
                    question: "审批完成后由谁开通？".to_string(),
                    answer: "由 DBA 或平台管理员完成开通。".to_string(),
                },
            )
            .expect("faq updated");
        assert_eq!(updated_faq.question, "审批完成后由谁开通？");

        store.delete_faq(faq.faq_id).expect("faq deleted");
        assert_eq!(store.list_faq_items(1).len(), 1);

        store.delete_tag("审核".to_string()).expect("tag deleted");
        assert!(store.list_tags().iter().all(|item| item.tag_name != "审核"));

        store
            .delete_category("安全制度".to_string())
            .expect("category deleted");
        assert!(store
            .list_categories()
            .iter()
            .all(|item| item.category_name != "安全制度"));

        let _ = fs::remove_file(path);
    }

    #[test]
    fn user_management_should_create_and_update_users() {
        let path = temp_store_path("users");
        let store = MemoryStore::from_path(&path);

        let created = store
            .create_user(UserCreateRequest {
                username: "viewer".to_string(),
                role_name: "普通用户".to_string(),
                department: "财务".to_string(),
                email: "viewer@example.com".to_string(),
                password: "Viewer@123456".to_string(),
            })
            .expect("user created");
        assert_eq!(created.user_id, 3);
        assert_eq!(store.list_users().len(), 3);
        assert!(store
            .authenticate("viewer".to_string(), "Viewer@123456".to_string())
            .is_some());

        let updated = store
            .update_user(
                created.user_id,
                UserUpdateRequest {
                    username: "viewer".to_string(),
                    role_name: "知识管理员".to_string(),
                    department: "知识运营".to_string(),
                    email: "viewer-updated@example.com".to_string(),
                    password: Some("Viewer@654321".to_string()),
                },
            )
            .expect("user updated");
        assert_eq!(updated.role_name, "知识管理员");
        assert_eq!(updated.department, "知识运营");
        assert!(store
            .authenticate("viewer".to_string(), "Viewer@654321".to_string())
            .is_some());
        assert_eq!(store.list_roles().len(), 3);

        let _ = fs::remove_file(path);
    }

    #[test]
    fn persisted_store_should_survive_restart() {
        let path = temp_store_path("persist");
        let store = MemoryStore::from_path(&path);
        let created = store.create_document(1, build_create_request());
        assert_eq!(created.document_id, 2);
        drop(store);

        let reloaded = MemoryStore::from_path(&path);
        let documents = reloaded.list_documents(1);
        assert_eq!(documents.len(), 2);
        assert!(documents.iter().any(|item| item.title == "VPN 开通流程"));

        let detail = reloaded.get_document(1, created.document_id).expect("reloaded document");
        assert_eq!(detail.version_no, "v1.0.0");

        let _ = fs::remove_file(path);
    }
}
