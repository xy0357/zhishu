use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use sqlx::{MySql, MySqlPool, Row, Transaction};

use crate::{
    models::{
        AgentRun, CategoryItem, CategoryUpsertRequest, Citation, CreateDocumentRequest,
        DashboardSummary, DeletedResource, DocumentDetail, DocumentListItem, DocumentVersion,
        FaqItem, FaqUpsertRequest, FavoriteDocumentItem, FavoriteState, QaAnswer,
        QuestionHistoryItem, ReadRecordItem, RoleItem, TagItem, TagUpsertRequest,
        UpdateDocumentRequest, UserCreateRequest, UserItem, UserUpdateRequest,
    },
    security::{
        hash_password, verify_password, ADMIN_PASSWORD, ADMIN_USERNAME, EDITOR_PASSWORD,
        EDITOR_USERNAME,
    },
    store::{AppStore, StoreMutationError},
};

pub struct MySqlStore {
    pool: MySqlPool,
}

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

impl MySqlStore {
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = MySqlPool::connect(database_url).await?;
        MIGRATOR.run(&pool).await?;
        let store = Self { pool };
        store.ensure_bootstrap_data().await?;
        Ok(store)
    }

    async fn ensure_bootstrap_data(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO roles (role_id, role_name, description)
             VALUES (1, '系统管理员', '系统默认管理员')
             ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO roles (role_id, role_name, description)
             VALUES (2, '知识管理员', '知识维护与运营管理员')
             ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO roles (role_id, role_name, description)
             VALUES (3, '普通用户', '普通知识使用者')
             ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description)",
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO users (user_id, role_id, username, password_hash, email, department)
             VALUES (?, 1, ?, ?, 'admin@example.com', 'IT')
             ON DUPLICATE KEY UPDATE username = VALUES(username), password_hash = VALUES(password_hash), email = VALUES(email), department = VALUES(department)",
        )
        .bind(1_i64)
        .bind(ADMIN_USERNAME)
        .bind(hash_password(ADMIN_PASSWORD))
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "INSERT INTO users (user_id, role_id, username, password_hash, email, department)
             VALUES (?, 2, ?, ?, 'editor@example.com', '知识运营')
             ON DUPLICATE KEY UPDATE username = VALUES(username), password_hash = VALUES(password_hash), email = VALUES(email), department = VALUES(department)",
        )
        .bind(2_i64)
        .bind(EDITOR_USERNAME)
        .bind(hash_password(EDITOR_PASSWORD))
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn ensure_category(
        tx: &mut Transaction<'_, MySql>,
        category_name: &str,
    ) -> Result<i64, sqlx::Error> {
        if let Some(existing) = sqlx::query("SELECT category_id FROM categories WHERE category_name = ?")
            .bind(category_name)
            .fetch_optional(tx.as_mut())
            .await?
        {
            return Ok(existing.get::<i64, _>("category_id"));
        }

        let result = sqlx::query(
            "INSERT INTO categories (category_name, description) VALUES (?, ?)",
        )
        .bind(category_name)
        .bind(format!("{} 相关知识分类", category_name))
        .execute(tx.as_mut())
        .await?;

        Ok(result.last_insert_id() as i64)
    }

    async fn ensure_tag(tx: &mut Transaction<'_, MySql>, tag_name: &str) -> Result<i64, sqlx::Error> {
        if let Some(existing) = sqlx::query("SELECT tag_id FROM tags WHERE tag_name = ?")
            .bind(tag_name)
            .fetch_optional(tx.as_mut())
            .await?
        {
            return Ok(existing.get::<i64, _>("tag_id"));
        }

        let result = sqlx::query("INSERT INTO tags (tag_name, description) VALUES (?, ?)")
            .bind(tag_name)
            .bind(format!("{} 主题标签", tag_name))
            .execute(tx.as_mut())
            .await?;

        Ok(result.last_insert_id() as i64)
    }

    async fn replace_document_tags(
        tx: &mut Transaction<'_, MySql>,
        document_id: i64,
        tags: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM document_tags WHERE document_id = ?")
            .bind(document_id)
            .execute(tx.as_mut())
            .await?;

        for tag in tags {
            let tag_id = Self::ensure_tag(tx, tag).await?;
            sqlx::query("INSERT INTO document_tags (document_id, tag_id) VALUES (?, ?)")
                .bind(document_id)
                .bind(tag_id)
                .execute(tx.as_mut())
                .await?;
        }

        Ok(())
    }

    async fn replace_document_faq(
        tx: &mut Transaction<'_, MySql>,
        document_id: i64,
        title: &str,
        summary: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM faq_items WHERE document_id = ?")
            .bind(document_id)
            .execute(tx.as_mut())
            .await?;

        sqlx::query(
            "INSERT INTO faq_items (document_id, question, answer, status) VALUES (?, ?, ?, 'active')",
        )
        .bind(document_id)
        .bind(format!("{} 的核心流程是什么？", title))
        .bind(summary)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    async fn create_agent_run(
        tx: &mut Transaction<'_, MySql>,
        operator_user_id: u64,
        agent_type: &str,
        trigger_type: &str,
        status: &str,
        input_text: &str,
        output_text: &str,
        document_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO agent_runs (
                agent_type, trigger_type, operator_user_id, document_id, status, input_text, output_text, started_at, finished_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(agent_type)
        .bind(trigger_type)
        .bind(operator_user_id as i64)
        .bind(document_id)
        .bind(status)
        .bind(input_text)
        .bind(output_text)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    async fn load_tags(&self, document_id: i64) -> Result<Vec<String>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT t.tag_name
             FROM document_tags dt
             JOIN tags t ON dt.tag_id = t.tag_id
             WHERE dt.document_id = ?
             ORDER BY t.tag_name",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|row| row.get::<String, _>("tag_name")).collect())
    }

    async fn load_versions(&self, document_id: i64) -> Result<Vec<DocumentVersion>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT version_id, version_no, title, content, summary, change_note, created_at
             FROM document_versions
             WHERE document_id = ?
             ORDER BY version_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| DocumentVersion {
                version_id: row.get::<i64, _>("version_id") as u64,
                version_no: row.get::<String, _>("version_no"),
                title: row.get::<String, _>("title"),
                content: row.get::<String, _>("content"),
                summary: row
                    .try_get::<Option<String>, _>("summary")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                change_note: row.get::<String, _>("change_note"),
                created_at: Self::mysql_dt_to_utc(row.get("created_at")),
            })
            .collect())
    }

    async fn load_document_detail_for_user(
        &self,
        user_id: u64,
        document_id: i64,
    ) -> Result<Option<DocumentDetail>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT d.document_id, d.title, d.summary, dv.content, c.category_name, d.status, d.current_version_no,
                    CASE WHEN fr.favorite_id IS NULL THEN 0 ELSE 1 END AS is_favorite
             FROM documents d
             JOIN categories c ON d.category_id = c.category_id
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             LEFT JOIN favorite_records fr ON fr.document_id = d.document_id AND fr.user_id = ?
             WHERE d.document_id = ?",
        )
        .bind(user_id as i64)
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };

        let tags = self.load_tags(document_id).await?;
        let versions = self.load_versions(document_id).await?;

        Ok(Some(DocumentDetail {
            document_id: row.get::<i64, _>("document_id") as u64,
            title: row.get::<String, _>("title"),
            summary: row
                .try_get::<Option<String>, _>("summary")
                .ok()
                .flatten()
                .unwrap_or_default(),
            content: row
                .try_get::<Option<String>, _>("content")
                .ok()
                .flatten()
                .unwrap_or_default(),
            category_name: row.get::<String, _>("category_name"),
            status: row.get::<String, _>("status"),
            version_no: row.get::<String, _>("current_version_no"),
            is_favorite: row.get::<i8, _>("is_favorite") == 1,
            tags,
            versions,
        }))
    }

    async fn load_user_by_username(&self, username: &str) -> Result<Option<UserItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT u.user_id, u.username, r.role_name, u.department, u.email
             FROM users u
             JOIN roles r ON u.role_id = r.role_id
             WHERE u.username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| UserItem {
            user_id: row.get::<i64, _>("user_id") as u64,
            username: row.get::<String, _>("username"),
            role_name: row.get::<String, _>("role_name"),
            department: row
                .try_get::<Option<String>, _>("department")
                .ok()
                .flatten()
                .unwrap_or_default(),
            email: row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .unwrap_or_default(),
        }))
    }

    async fn load_user_by_id(&self, user_id: u64) -> Result<Option<UserItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT u.user_id, u.username, r.role_name, u.department, u.email
             FROM users u
             JOIN roles r ON u.role_id = r.role_id
             WHERE u.user_id = ?",
        )
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| UserItem {
            user_id: row.get::<i64, _>("user_id") as u64,
            username: row.get::<String, _>("username"),
            role_name: row.get::<String, _>("role_name"),
            department: row
                .try_get::<Option<String>, _>("department")
                .ok()
                .flatten()
                .unwrap_or_default(),
            email: row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .unwrap_or_default(),
        }))
    }

    async fn load_category_item(&self, category_name: &str) -> Result<Option<CategoryItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT c.category_name, c.description, COUNT(d.document_id) AS document_count
             FROM categories c
             LEFT JOIN documents d ON d.category_id = c.category_id
             WHERE c.category_name = ?
             GROUP BY c.category_id, c.category_name, c.description",
        )
        .bind(category_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| CategoryItem {
            category_name: row.get::<String, _>("category_name"),
            description: row
                .try_get::<Option<String>, _>("description")
                .ok()
                .flatten()
                .unwrap_or_default(),
            document_count: row.get::<i64, _>("document_count") as usize,
        }))
    }

    async fn load_tag_item(&self, tag_name: &str) -> Result<Option<TagItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT t.tag_name, t.description, COUNT(dt.document_id) AS document_count
             FROM tags t
             LEFT JOIN document_tags dt ON dt.tag_id = t.tag_id
             WHERE t.tag_name = ?
             GROUP BY t.tag_id, t.tag_name, t.description",
        )
        .bind(tag_name)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| TagItem {
            tag_name: row.get::<String, _>("tag_name"),
            description: row
                .try_get::<Option<String>, _>("description")
                .ok()
                .flatten()
                .unwrap_or_default(),
            document_count: row.get::<i64, _>("document_count") as usize,
        }))
    }

    async fn load_faq_item(&self, faq_id: i64) -> Result<Option<FaqItem>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT faq_id, document_id, question, answer, created_at
             FROM faq_items
             WHERE faq_id = ?",
        )
        .bind(faq_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| FaqItem {
            faq_id: row.get::<i64, _>("faq_id") as u64,
            document_id: row.get::<i64, _>("document_id") as u64,
            question: row.get::<String, _>("question"),
            answer: row.get::<String, _>("answer"),
            created_at: Self::mysql_dt_to_utc(row.get("created_at")),
        }))
    }

    fn mysql_dt_to_utc(value: sqlx::types::chrono::NaiveDateTime) -> DateTime<Utc> {
        DateTime::<Utc>::from_naive_utc_and_offset(value, Utc)
    }
}

#[async_trait]
impl AppStore for MySqlStore {
    async fn authenticate(&self, username: String, password: String) -> Option<UserItem> {
        let row = sqlx::query(
            "SELECT u.user_id, u.username, u.password_hash, r.role_name, u.department, u.email
             FROM users u
             JOIN roles r ON u.role_id = r.role_id
             WHERE u.username = ?",
        )
        .bind(&username)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten()?;
        let stored_password = row.get::<String, _>("password_hash");
        if !verify_password(&password, &stored_password) {
            return None;
        }
        Some(UserItem {
            user_id: row.get::<i64, _>("user_id") as u64,
            username: row.get::<String, _>("username"),
            role_name: row.get::<String, _>("role_name"),
            department: row
                .try_get::<Option<String>, _>("department")
                .ok()
                .flatten()
                .unwrap_or_default(),
            email: row
                .try_get::<Option<String>, _>("email")
                .ok()
                .flatten()
                .unwrap_or_default(),
        })
    }

    async fn get_user_by_username(&self, username: String) -> Option<UserItem> {
        self.load_user_by_username(&username).await.ok().flatten()
    }

    async fn dashboard_summary(&self) -> DashboardSummary {
        let total_documents = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        let published_documents = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM documents WHERE status = 'published'",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);
        let total_questions = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM questions")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);
        let total_agent_runs = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM agent_runs")
            .fetch_one(&self.pool)
            .await
            .unwrap_or(0);

        DashboardSummary {
            total_documents: total_documents as usize,
            published_documents: published_documents as usize,
            total_questions: total_questions as usize,
            total_agent_runs: total_agent_runs as usize,
        }
    }

    async fn list_documents(&self, user_id: u64) -> Vec<DocumentListItem> {
        let rows = sqlx::query(
            "SELECT d.document_id, d.title, d.summary, c.category_name, d.status, d.current_version_no, d.updated_at,
                    CASE WHEN fr.favorite_id IS NULL THEN 0 ELSE 1 END AS is_favorite
             FROM documents d
             JOIN categories c ON d.category_id = c.category_id
             LEFT JOIN favorite_records fr ON fr.document_id = d.document_id AND fr.user_id = ?
             ORDER BY d.updated_at DESC, d.document_id DESC",
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| DocumentListItem {
                document_id: row.get::<i64, _>("document_id") as u64,
                title: row.get::<String, _>("title"),
                summary: row
                    .try_get::<Option<String>, _>("summary")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                category_name: row.get::<String, _>("category_name"),
                status: row.get::<String, _>("status"),
                version_no: row.get::<String, _>("current_version_no"),
                is_favorite: row.get::<i8, _>("is_favorite") == 1,
                updated_at: Self::mysql_dt_to_utc(row.get("updated_at")),
            })
            .collect()
    }

    async fn get_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        self.load_document_detail_for_user(user_id, id as i64).await.ok().flatten()
    }

    async fn get_versions(&self, id: u64) -> Option<Vec<DocumentVersion>> {
        self.load_versions(id as i64).await.ok()
    }

    async fn create_document(&self, user_id: u64, payload: CreateDocumentRequest) -> DocumentDetail {
        let mut tx = self.pool.begin().await.expect("begin create document tx");
        let category_id = Self::ensure_category(&mut tx, &payload.category_name)
            .await
            .expect("ensure category");

        let insert_document = sqlx::query(
            "INSERT INTO documents (
                category_id, creator_id, current_version_no, title, summary, status, created_at, updated_at
             ) VALUES (?, ?, 'v1.0.0', ?, ?, 'draft', UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(category_id)
        .bind(user_id as i64)
        .bind(&payload.title)
        .bind(&payload.summary)
        .execute(tx.as_mut())
        .await
        .expect("insert document");
        let document_id = insert_document.last_insert_id() as i64;

        let insert_version = sqlx::query(
            "INSERT INTO document_versions (
                document_id, version_no, title, content, summary, change_note, is_published_snapshot, created_by, created_at
             ) VALUES (?, 'v1.0.0', ?, ?, ?, ?, 0, ?, UTC_TIMESTAMP())",
        )
        .bind(document_id)
        .bind(&payload.title)
        .bind(&payload.content)
        .bind(&payload.summary)
        .bind(&payload.change_note)
        .bind(user_id as i64)
        .execute(tx.as_mut())
        .await
        .expect("insert document version");
        let version_id = insert_version.last_insert_id() as i64;

        sqlx::query(
            "UPDATE documents
             SET current_version_id = ?, updated_at = UTC_TIMESTAMP()
             WHERE document_id = ?",
        )
        .bind(version_id)
        .bind(document_id)
        .execute(tx.as_mut())
        .await
        .expect("update current version");

        Self::replace_document_tags(&mut tx, document_id, &payload.tags)
            .await
            .expect("replace tags");
        Self::replace_document_faq(&mut tx, document_id, &payload.title, &payload.summary)
            .await
            .expect("replace faq");
        Self::create_agent_run(
            &mut tx,
            user_id,
            "summary",
            "manual",
            "success",
            &payload.title,
            "新文档已写入并生成初始摘要",
            Some(document_id),
        )
        .await
        .expect("create agent run");

        tx.commit().await.expect("commit create document");

        self.load_document_detail_for_user(user_id, document_id)
            .await
            .expect("reload created document")
            .expect("created document exists")
    }

    async fn update_document(&self, user_id: u64, id: u64, payload: UpdateDocumentRequest) -> Option<DocumentDetail> {
        let document_id = id as i64;
        let mut tx = self.pool.begin().await.ok()?;
        let category_id = Self::ensure_category(&mut tx, &payload.category_name).await.ok()?;

        let version_count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM document_versions WHERE document_id = ?",
        )
        .bind(document_id)
        .fetch_one(tx.as_mut())
        .await
        .ok()? as usize;

        let version_no = format!("v1.0.{}", version_count);

        let version_result = sqlx::query(
            "INSERT INTO document_versions (
                document_id, version_no, title, content, summary, change_note, is_published_snapshot, created_by, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, 0, ?, UTC_TIMESTAMP())",
        )
        .bind(document_id)
        .bind(&version_no)
        .bind(&payload.title)
        .bind(&payload.content)
        .bind(&payload.summary)
        .bind(&payload.change_note)
        .bind(user_id as i64)
        .execute(tx.as_mut())
        .await
        .ok()?;
        let version_id = version_result.last_insert_id() as i64;

        sqlx::query(
            "UPDATE documents
             SET category_id = ?, current_version_id = ?, current_version_no = ?, title = ?, summary = ?, updated_at = UTC_TIMESTAMP()
             WHERE document_id = ?",
        )
        .bind(category_id)
        .bind(version_id)
        .bind(&version_no)
        .bind(&payload.title)
        .bind(&payload.summary)
        .bind(document_id)
        .execute(tx.as_mut())
        .await
        .ok()?;

        Self::replace_document_tags(&mut tx, document_id, &payload.tags).await.ok()?;
        Self::replace_document_faq(&mut tx, document_id, &payload.title, &payload.summary)
            .await
            .ok()?;
        Self::create_agent_run(
            &mut tx,
            user_id,
            "summary",
            "manual",
            "success",
            &payload.title,
            "文档已更新并生成新版本",
            Some(document_id),
        )
        .await
        .ok()?;

        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }

    async fn publish_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let document_id = id as i64;
        let mut tx = self.pool.begin().await.ok()?;
        let row = sqlx::query("SELECT title, current_version_id FROM documents WHERE document_id = ?")
            .bind(document_id)
            .fetch_optional(tx.as_mut())
            .await
            .ok()??;
        let title = row.get::<String, _>("title");
        let current_version_id = row.try_get::<Option<i64>, _>("current_version_id").ok().flatten();

        sqlx::query("UPDATE documents SET status = 'published', published_at = UTC_TIMESTAMP(), updated_at = UTC_TIMESTAMP() WHERE document_id = ?")
            .bind(document_id)
            .execute(tx.as_mut())
            .await
            .ok()?;

        if let Some(version_id) = current_version_id {
            sqlx::query("UPDATE document_versions SET is_published_snapshot = 1 WHERE version_id = ?")
                .bind(version_id)
                .execute(tx.as_mut())
                .await
                .ok()?;
        }

        Self::create_agent_run(
            &mut tx,
            user_id,
            "audit",
            "document_publish",
            "success",
            &title,
            "文档发布成功",
            Some(document_id),
        )
        .await
        .ok()?;

        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }

    async fn archive_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let document_id = id as i64;
        let mut tx = self.pool.begin().await.ok()?;
        let row = sqlx::query("SELECT title FROM documents WHERE document_id = ?")
            .bind(document_id)
            .fetch_optional(tx.as_mut())
            .await
            .ok()??;
        let title = row.get::<String, _>("title");

        sqlx::query("UPDATE documents SET status = 'archived', updated_at = UTC_TIMESTAMP() WHERE document_id = ?")
            .bind(document_id)
            .execute(tx.as_mut())
            .await
            .ok()?;

        Self::create_agent_run(
            &mut tx,
            user_id,
            "audit",
            "manual",
            "success",
            &title,
            "文档已归档",
            Some(document_id),
        )
        .await
        .ok()?;

        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }

    async fn ask_question(&self, user_id: u64, question_text: String) -> QaAnswer {
        let mut tx = self.pool.begin().await.expect("begin ask question tx");
        let doc = sqlx::query(
            "SELECT d.document_id, d.title, d.current_version_id, d.current_version_no, dv.content
             FROM documents d
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             WHERE d.status <> 'archived'
             ORDER BY d.document_id
             LIMIT 1",
        )
        .fetch_optional(tx.as_mut())
        .await
        .expect("load context document");

        let answer_text = format!(
            "根据当前知识库，关于“{}”的建议处理方式是：先走标准申请流程，再由对应管理员审核开通。",
            question_text
        );

        let insert_question = sqlx::query(
            "INSERT INTO questions (user_id, question_text, status, created_at) VALUES (?, ?, 'answered', UTC_TIMESTAMP())",
        )
        .bind(user_id as i64)
        .bind(&question_text)
        .execute(tx.as_mut())
        .await
        .expect("insert question");
        let question_id = insert_question.last_insert_id() as i64;

        let insert_answer = sqlx::query(
            "INSERT INTO answers (question_id, answer_text, confidence_score, model_name, status, latency_ms, created_at)
             VALUES (?, ?, 0.88, 'demo-rag-agent', 'success', 120, UTC_TIMESTAMP())",
        )
        .bind(question_id)
        .bind(&answer_text)
        .execute(tx.as_mut())
        .await
        .expect("insert answer");
        let answer_id = insert_answer.last_insert_id() as i64;

        let mut citations = Vec::new();
        if let Some(doc) = doc {
            let document_id = doc.get::<i64, _>("document_id");
            let document_title = doc.get::<String, _>("title");
            let version_id = doc.try_get::<Option<i64>, _>("current_version_id").ok().flatten().unwrap_or(0);
            let version_no = doc
                .try_get::<Option<String>, _>("current_version_no")
                .ok()
                .flatten()
                .unwrap_or_else(|| "v0".to_string());
            let content = doc
                .try_get::<Option<String>, _>("content")
                .ok()
                .flatten()
                .unwrap_or_default();
            let snippet_text = content.lines().next().unwrap_or_default().to_string();

            sqlx::query(
                "INSERT INTO answer_citations (
                    answer_id, document_id, version_id, segment_id, cite_order, score, snippet_text
                 ) VALUES (?, ?, ?, NULL, 1, 0.92, ?)",
            )
            .bind(answer_id)
            .bind(document_id)
            .bind(version_id)
            .bind(&snippet_text)
            .execute(tx.as_mut())
            .await
            .expect("insert citation");

            citations.push(Citation {
                cite_order: 1,
                document_title,
                version_no,
                snippet_text,
                score: 0.92,
            });
        }

        sqlx::query(
            "INSERT INTO agent_runs (
                agent_type, trigger_type, operator_user_id, question_id, answer_id, status, input_text, output_text, started_at, finished_at
             ) VALUES ('answer', 'question_submit', ?, ?, ?, 'success', ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(user_id as i64)
        .bind(question_id)
        .bind(answer_id)
        .bind(&question_text)
        .bind(&answer_text)
        .execute(tx.as_mut())
        .await
        .expect("insert agent run");

        tx.commit().await.expect("commit ask question");

        QaAnswer {
            answer_id: answer_id as u64,
            answer_text,
            confidence_score: 0.88,
            citations,
            created_at: Utc::now(),
        }
    }

    async fn list_question_history(&self, user_id: u64) -> Vec<QuestionHistoryItem> {
        let rows = sqlx::query(
            "SELECT q.question_id, q.question_text, q.created_at, a.answer_text
             FROM questions q
             LEFT JOIN answers a ON a.question_id = q.question_id
             WHERE q.user_id = ?
             ORDER BY q.question_id DESC",
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| {
                let answer_text = row
                    .try_get::<Option<String>, _>("answer_text")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                QuestionHistoryItem {
                    question_id: row.get::<i64, _>("question_id") as u64,
                    question_text: row.get::<String, _>("question_text"),
                    answer_preview: answer_text.chars().take(48).collect(),
                    created_at: Self::mysql_dt_to_utc(row.get("created_at")),
                }
            })
            .collect()
    }

    async fn list_agent_runs(&self) -> Vec<AgentRun> {
        let rows = sqlx::query(
            "SELECT run_id, agent_type, trigger_type, status, input_text, output_text, started_at
             FROM agent_runs
             ORDER BY run_id DESC",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| AgentRun {
                run_id: row.get::<i64, _>("run_id") as u64,
                agent_type: row.get::<String, _>("agent_type"),
                trigger_type: row.get::<String, _>("trigger_type"),
                status: row.get::<String, _>("status"),
                input_text: row
                    .try_get::<Option<String>, _>("input_text")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                output_text: row
                    .try_get::<Option<String>, _>("output_text")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                started_at: Self::mysql_dt_to_utc(row.get("started_at")),
            })
            .collect()
    }

    async fn list_categories(&self) -> Vec<CategoryItem> {
        let rows = sqlx::query(
            "SELECT c.category_name, c.description, COUNT(d.document_id) AS document_count
             FROM categories c
             LEFT JOIN documents d ON d.category_id = c.category_id
             GROUP BY c.category_id, c.category_name, c.description
             ORDER BY c.category_name",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| CategoryItem {
                category_name: row.get::<String, _>("category_name"),
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                document_count: row.get::<i64, _>("document_count") as usize,
            })
            .collect()
    }

    async fn create_category(&self, payload: CategoryUpsertRequest) -> CategoryItem {
        sqlx::query(
            "INSERT INTO categories (category_name, description)
             VALUES (?, ?)
             ON DUPLICATE KEY UPDATE description = VALUES(description)",
        )
        .bind(&payload.category_name)
        .bind(&payload.description)
        .execute(&self.pool)
        .await
        .expect("create category");

        self.load_category_item(&payload.category_name)
            .await
            .expect("load created category")
            .expect("created category exists")
    }

    async fn update_category(
        &self,
        current_name: String,
        payload: CategoryUpsertRequest,
    ) -> Result<CategoryItem, StoreMutationError> {
        let existing_id = sqlx::query_scalar::<_, i64>(
            "SELECT category_id FROM categories WHERE category_name = ?",
        )
        .bind(&current_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?
        .ok_or(StoreMutationError::NotFound)?;

        if current_name != payload.category_name {
            let duplicate_id = sqlx::query_scalar::<_, i64>(
                "SELECT category_id FROM categories WHERE category_name = ?",
            )
            .bind(&payload.category_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
            if duplicate_id.is_some() {
                return Err(StoreMutationError::Conflict);
            }
        }

        sqlx::query(
            "UPDATE categories SET category_name = ?, description = ? WHERE category_id = ?",
        )
        .bind(&payload.category_name)
        .bind(&payload.description)
        .bind(existing_id)
        .execute(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;

        self.load_category_item(&payload.category_name)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn delete_category(
        &self,
        category_name: String,
    ) -> Result<DeletedResource, StoreMutationError> {
        let row = sqlx::query(
            "SELECT c.category_id, COUNT(d.document_id) AS document_count
             FROM categories c
             LEFT JOIN documents d ON d.category_id = c.category_id
             WHERE c.category_name = ?
             GROUP BY c.category_id",
        )
        .bind(&category_name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?
        .ok_or(StoreMutationError::NotFound)?;

        if row.get::<i64, _>("document_count") > 0 {
            return Err(StoreMutationError::Conflict);
        }

        sqlx::query("DELETE FROM categories WHERE category_id = ?")
            .bind(row.get::<i64, _>("category_id"))
            .execute(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;

        Ok(DeletedResource {
            resource_type: "category".to_string(),
            resource_key: category_name,
        })
    }

    async fn list_tags(&self) -> Vec<TagItem> {
        let rows = sqlx::query(
            "SELECT t.tag_name, t.description, COUNT(dt.document_id) AS document_count
             FROM tags t
             LEFT JOIN document_tags dt ON dt.tag_id = t.tag_id
             GROUP BY t.tag_id, t.tag_name, t.description
             ORDER BY t.tag_name",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| TagItem {
                tag_name: row.get::<String, _>("tag_name"),
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                document_count: row.get::<i64, _>("document_count") as usize,
            })
            .collect()
    }

    async fn create_tag(&self, payload: TagUpsertRequest) -> TagItem {
        sqlx::query(
            "INSERT INTO tags (tag_name, description)
             VALUES (?, ?)
             ON DUPLICATE KEY UPDATE description = VALUES(description)",
        )
        .bind(&payload.tag_name)
        .bind(&payload.description)
        .execute(&self.pool)
        .await
        .expect("create tag");

        self.load_tag_item(&payload.tag_name)
            .await
            .expect("load created tag")
            .expect("created tag exists")
    }

    async fn update_tag(
        &self,
        current_name: String,
        payload: TagUpsertRequest,
    ) -> Result<TagItem, StoreMutationError> {
        let existing_id = sqlx::query_scalar::<_, i64>("SELECT tag_id FROM tags WHERE tag_name = ?")
            .bind(&current_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)?;

        if current_name != payload.tag_name {
            let duplicate_id =
                sqlx::query_scalar::<_, i64>("SELECT tag_id FROM tags WHERE tag_name = ?")
                    .bind(&payload.tag_name)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|_| StoreMutationError::Conflict)?;
            if duplicate_id.is_some() {
                return Err(StoreMutationError::Conflict);
            }
        }

        sqlx::query("UPDATE tags SET tag_name = ?, description = ? WHERE tag_id = ?")
            .bind(&payload.tag_name)
            .bind(&payload.description)
            .bind(existing_id)
            .execute(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;

        self.load_tag_item(&payload.tag_name)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn delete_tag(&self, tag_name: String) -> Result<DeletedResource, StoreMutationError> {
        let tag_id = sqlx::query_scalar::<_, i64>("SELECT tag_id FROM tags WHERE tag_name = ?")
            .bind(&tag_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)?;

        let mut tx = self.pool.begin().await.map_err(|_| StoreMutationError::Conflict)?;
        sqlx::query("DELETE FROM document_tags WHERE tag_id = ?")
            .bind(tag_id)
            .execute(tx.as_mut())
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        sqlx::query("DELETE FROM tags WHERE tag_id = ?")
            .bind(tag_id)
            .execute(tx.as_mut())
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        tx.commit().await.map_err(|_| StoreMutationError::Conflict)?;

        Ok(DeletedResource {
            resource_type: "tag".to_string(),
            resource_key: tag_name,
        })
    }

    async fn list_faq_items(&self, document_id: u64) -> Vec<FaqItem> {
        let rows = sqlx::query(
            "SELECT faq_id, document_id, question, answer, created_at
             FROM faq_items
             WHERE document_id = ?
             ORDER BY faq_id DESC",
        )
        .bind(document_id as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| FaqItem {
                faq_id: row.get::<i64, _>("faq_id") as u64,
                document_id: row.get::<i64, _>("document_id") as u64,
                question: row.get::<String, _>("question"),
                answer: row.get::<String, _>("answer"),
                created_at: Self::mysql_dt_to_utc(row.get("created_at")),
            })
            .collect()
    }

    async fn create_faq(
        &self,
        document_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents WHERE document_id = ?")
            .bind(document_id as i64)
            .fetch_one(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        if exists == 0 {
            return Err(StoreMutationError::NotFound);
        }

        let result = sqlx::query(
            "INSERT INTO faq_items (document_id, question, answer, status)
             VALUES (?, ?, ?, 'active')",
        )
        .bind(document_id as i64)
        .bind(&payload.question)
        .bind(&payload.answer)
        .execute(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;

        self.load_faq_item(result.last_insert_id() as i64)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn update_faq(
        &self,
        faq_id: u64,
        payload: FaqUpsertRequest,
    ) -> Result<FaqItem, StoreMutationError> {
        let affected = sqlx::query("UPDATE faq_items SET question = ?, answer = ? WHERE faq_id = ?")
            .bind(&payload.question)
            .bind(&payload.answer)
            .bind(faq_id as i64)
            .execute(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        if affected.rows_affected() == 0 {
            return Err(StoreMutationError::NotFound);
        }

        self.load_faq_item(faq_id as i64)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn delete_faq(&self, faq_id: u64) -> Result<DeletedResource, StoreMutationError> {
        let affected = sqlx::query("DELETE FROM faq_items WHERE faq_id = ?")
            .bind(faq_id as i64)
            .execute(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        if affected.rows_affected() == 0 {
            return Err(StoreMutationError::NotFound);
        }

        Ok(DeletedResource {
            resource_type: "faq".to_string(),
            resource_key: faq_id.to_string(),
        })
    }

    async fn list_roles(&self) -> Vec<RoleItem> {
        let rows = sqlx::query(
            "SELECT r.role_name, r.description, COUNT(u.user_id) AS user_count
             FROM roles r
             LEFT JOIN users u ON u.role_id = r.role_id
             GROUP BY r.role_id, r.role_name, r.description
             ORDER BY r.role_id",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| RoleItem {
                role_name: row.get::<String, _>("role_name"),
                description: row
                    .try_get::<Option<String>, _>("description")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                user_count: row.get::<i64, _>("user_count") as usize,
            })
            .collect()
    }

    async fn list_users(&self) -> Vec<UserItem> {
        let rows = sqlx::query(
            "SELECT u.user_id, u.username, r.role_name, u.department, u.email
             FROM users u
             JOIN roles r ON u.role_id = r.role_id
             ORDER BY u.user_id",
        )
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| UserItem {
                user_id: row.get::<i64, _>("user_id") as u64,
                username: row.get::<String, _>("username"),
                role_name: row.get::<String, _>("role_name"),
                department: row
                    .try_get::<Option<String>, _>("department")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
                email: row
                    .try_get::<Option<String>, _>("email")
                    .ok()
                    .flatten()
                    .unwrap_or_default(),
            })
            .collect()
    }

    async fn create_user(&self, payload: UserCreateRequest) -> Result<UserItem, StoreMutationError> {
        let role_id = sqlx::query_scalar::<_, i64>("SELECT role_id FROM roles WHERE role_name = ?")
            .bind(&payload.role_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)?;

        let duplicate = sqlx::query_scalar::<_, i64>("SELECT user_id FROM users WHERE username = ?")
            .bind(&payload.username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        if duplicate.is_some() {
            return Err(StoreMutationError::Conflict);
        }

        let result = sqlx::query(
            "INSERT INTO users (role_id, username, password_hash, email, department)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(role_id)
        .bind(&payload.username)
        .bind(hash_password(&payload.password))
        .bind(&payload.email)
        .bind(&payload.department)
        .execute(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;

        self.load_user_by_id(result.last_insert_id())
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn update_user(
        &self,
        user_id: u64,
        payload: UserUpdateRequest,
    ) -> Result<UserItem, StoreMutationError> {
        if user_id == 1 && payload.username != "admin" {
            return Err(StoreMutationError::Conflict);
        }

        let role_id = sqlx::query_scalar::<_, i64>("SELECT role_id FROM roles WHERE role_name = ?")
            .bind(&payload.role_name)
            .fetch_optional(&self.pool)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)?;

        let duplicate = sqlx::query_scalar::<_, i64>(
            "SELECT user_id FROM users WHERE username = ? AND user_id <> ?",
        )
        .bind(&payload.username)
        .bind(user_id as i64)
        .fetch_optional(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;
        if duplicate.is_some() {
            return Err(StoreMutationError::Conflict);
        }

        let password_value = if let Some(password) = payload.password.clone().filter(|value| !value.is_empty()) {
            hash_password(&password)
        } else {
            sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE user_id = ?")
                .bind(user_id as i64)
                .fetch_optional(&self.pool)
                .await
                .map_err(|_| StoreMutationError::Conflict)?
                .ok_or(StoreMutationError::NotFound)?
        };

        let affected = sqlx::query(
            "UPDATE users
             SET role_id = ?, username = ?, password_hash = ?, email = ?, department = ?
             WHERE user_id = ?",
        )
        .bind(role_id)
        .bind(&payload.username)
        .bind(password_value)
        .bind(&payload.email)
        .bind(&payload.department)
        .bind(user_id as i64)
        .execute(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;
        if affected.rows_affected() == 0 {
            return Err(StoreMutationError::NotFound);
        }

        self.load_user_by_id(user_id)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn list_favorite_documents(&self, user_id: u64) -> Vec<FavoriteDocumentItem> {
        let rows = sqlx::query(
            "SELECT d.document_id, d.title, c.category_name, d.status, d.current_version_no, f.favorite_time
             FROM favorite_records f
             JOIN documents d ON d.document_id = f.document_id
             JOIN categories c ON c.category_id = d.category_id
             WHERE f.user_id = ?
             ORDER BY f.favorite_time DESC",
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| FavoriteDocumentItem {
                document_id: row.get::<i64, _>("document_id") as u64,
                title: row.get::<String, _>("title"),
                category_name: row.get::<String, _>("category_name"),
                status: row.get::<String, _>("status"),
                version_no: row.get::<String, _>("current_version_no"),
                favorite_time: Self::mysql_dt_to_utc(row.get("favorite_time")),
            })
            .collect()
    }

    async fn list_recent_reads(&self, user_id: u64) -> Vec<ReadRecordItem> {
        let rows = sqlx::query(
            "SELECT rr.read_id, d.document_id, d.title, c.category_name, d.status, d.current_version_no, rr.read_time
             FROM read_records rr
             JOIN documents d ON d.document_id = rr.document_id
             JOIN categories c ON c.category_id = d.category_id
             WHERE rr.user_id = ?
             ORDER BY rr.read_id DESC
             LIMIT 12",
        )
        .bind(user_id as i64)
        .fetch_all(&self.pool)
        .await
        .unwrap_or_default();

        rows
            .into_iter()
            .map(|row| ReadRecordItem {
                read_id: row.get::<i64, _>("read_id") as u64,
                document_id: row.get::<i64, _>("document_id") as u64,
                title: row.get::<String, _>("title"),
                category_name: row.get::<String, _>("category_name"),
                status: row.get::<String, _>("status"),
                version_no: row.get::<String, _>("current_version_no"),
                read_time: Self::mysql_dt_to_utc(row.get("read_time")),
            })
            .collect()
    }

    async fn record_document_read(&self, user_id: u64, id: u64) -> Option<ReadRecordItem> {
        let document_id = id as i64;
        let row = sqlx::query(
            "SELECT d.document_id, d.title, c.category_name, d.status, d.current_version_no
             FROM documents d
             JOIN categories c ON c.category_id = d.category_id
             WHERE d.document_id = ?",
        )
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .ok()??;

        let inserted = sqlx::query(
            "INSERT INTO read_records (user_id, document_id, read_time) VALUES (?, ?, UTC_TIMESTAMP())",
        )
        .bind(user_id as i64)
        .bind(document_id)
        .execute(&self.pool)
        .await
        .ok()?;

        Some(ReadRecordItem {
            read_id: inserted.last_insert_id() as u64,
            document_id: row.get::<i64, _>("document_id") as u64,
            title: row.get::<String, _>("title"),
            category_name: row.get::<String, _>("category_name"),
            status: row.get::<String, _>("status"),
            version_no: row.get::<String, _>("current_version_no"),
            read_time: Utc::now(),
        })
    }

    async fn toggle_favorite_document(&self, user_id: u64, id: u64) -> Option<FavoriteState> {
        let document_id = id as i64;
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM documents WHERE document_id = ?")
            .bind(document_id)
            .fetch_one(&self.pool)
            .await
            .ok()?;

        if exists == 0 {
            return None;
        }

        let favorite_id = sqlx::query_scalar::<_, i64>(
            "SELECT favorite_id FROM favorite_records WHERE user_id = ? AND document_id = ?",
        )
        .bind(user_id as i64)
        .bind(document_id)
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        let is_favorite = if favorite_id.is_some() {
            sqlx::query("DELETE FROM favorite_records WHERE user_id = ? AND document_id = ?")
                .bind(user_id as i64)
                .bind(document_id)
                .execute(&self.pool)
                .await
                .ok()?;
            false
        } else {
            sqlx::query(
                "INSERT INTO favorite_records (user_id, document_id, favorite_time) VALUES (?, ?, UTC_TIMESTAMP())",
            )
            .bind(user_id as i64)
            .bind(document_id)
            .execute(&self.pool)
            .await
            .ok()?;
            true
        };

        Some(FavoriteState {
            document_id: id,
            is_favorite,
        })
    }
}
