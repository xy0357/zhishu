use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::migrate::Migrator;
use sqlx::{MySql, MySqlPool, Row, Transaction};

use crate::{
    llm::generate_answer,
    models::{
        AgentRun, CategoryItem, CategoryUpsertRequest, Citation, CreateDocumentRequest,
        DashboardSummary, DeletedResource, DocumentDetail, DocumentFileMeta, DocumentListItem,
        DocumentSegment, DocumentVersion, FaqItem, FaqUpsertRequest, FavoriteDocumentItem,
        FavoriteState, QaAnswer, QuestionHistoryItem, ReadRecordItem, RegisterDocumentFileRequest,
        RoleItem, TagItem, TagUpsertRequest, UpdateDocumentRequest, UserCreateRequest, UserItem,
        UserUpdateRequest,
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
        store.backfill_document_segments().await?;
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

        self.ensure_bootstrap_documents().await?;
        Ok(())
    }
    async fn backfill_document_segments(&self) -> Result<(), sqlx::Error> {
        let rows = sqlx::query(
            "SELECT d.document_id, d.current_version_id, dv.content
             FROM documents d
             JOIN document_versions dv ON dv.version_id = d.current_version_id
             WHERE d.status = 'published'
               AND d.current_version_id IS NOT NULL
               AND NOT EXISTS (
                   SELECT 1
                   FROM document_segments ds
                   WHERE ds.document_id = d.document_id
                     AND ds.version_id = d.current_version_id
               )",
        )
        .fetch_all(&self.pool)
        .await?;

        if rows.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;
        for row in rows {
            let document_id = row.get::<i64, _>("document_id");
            let version_id = row.get::<i64, _>("current_version_id");
            let content = row
                .try_get::<Option<String>, _>("content")
                .ok()
                .flatten()
                .unwrap_or_default();
            Self::replace_document_segments(&mut tx, document_id, version_id, &content).await?;
        }
        tx.commit().await?;
        Ok(())
    }


    fn bootstrap_documents() -> Vec<(&'static str, &'static str, &'static str, &'static str, Vec<&'static str>, &'static str, &'static str)> {
        vec![
            (
                "生产库只读权限申请流程",
                "规范研发、数据分析和运营人员申请生产数据库只读权限的审批流程与最小权限原则。",
                "1. 申请人需在 IT 服务台提交《生产库权限申请单》，写明系统名称、库表范围、使用场景和截止日期。\n2. 直属主管审批业务必要性，数据负责人确认字段范围，DBA 复核账号权限级别。\n3. 默认只开通只读账号，权限有效期最长 90 天，到期需重新申请。\n4. 涉及用户隐私字段时，必须先完成脱敏方案确认，再发放查询权限。",
                "制度流程",
                vec!["数据库", "权限", "生产环境", "审批"],
                "生产库只读权限申请要经过哪些审批？",
                "需要依次经过直属主管、数据负责人和 DBA 审批，默认只开通只读权限，且权限有效期最长 90 天。"
            ),
            (
                "数据导出与脱敏规范",
                "约束内部数据导出的审批边界、脱敏要求和留痕要求。",
                "1. 导出客户、订单、财务等敏感数据前，必须标注用途、接收人和保存期限。\n2. 手机号、身份证号、邮箱等个人信息默认按脱敏规则导出，未经批准不得提供明文。\n3. 导出结果必须存放到公司指定加密目录，不得通过个人聊天工具外发。\n4. 高敏数据导出需要安全负责人追加审批，并在导出后 24 小时内登记审计记录。",
                "数据安全",
                vec!["数据安全", "脱敏", "审计"],
                "哪些数据导出场景必须做脱敏？",
                "凡是涉及手机号、身份证号、邮箱等个人信息的导出，默认必须做脱敏；高敏数据还需要安全负责人追加审批。"
            ),
            (
                "发版变更与回滚SOP",
                "统一应用系统发版前检查、灰度验证和回滚要求。",
                "1. 发版前必须完成测试报告、数据库变更脚本评审和回滚方案确认。\n2. 生产发版统一安排在变更窗口执行，操作人和复核人不能为同一人。\n3. 若核心指标异常或错误率连续 5 分钟超阈值，应立即执行回滚并通知值班群。\n4. 发版结束后 30 分钟内补齐变更单、影响范围和结论记录。",
                "运维规范",
                vec!["发版", "回滚", "变更管理"],
                "什么时候需要立即回滚版本？",
                "当核心指标异常或错误率连续 5 分钟超出阈值时，应立即执行回滚，并同步通知值班群。"
            ),
            (
                "新员工入职账号开通清单",
                "梳理员工入职首日必须开通的系统账号、权限归属和责任人。",
                "1. HR 在入职前两个工作日提交入职工单，包含部门、岗位、直属主管和办公地点。\n2. IT 在入职当天 12 点前开通企业邮箱、OA、即时通讯和 VPN 账号。\n3. 研发岗位如需代码仓库、流水线和数据库权限，必须由直属主管按岗位模板发起二次申请。\n4. 所有账号需绑定企业 MFA，多次登录失败由服务台统一重置。",
                "人事与行政",
                vec!["入职", "账号", "MFA"],
                "新员工首日默认会开通哪些账号？",
                "默认开通企业邮箱、OA、即时通讯和 VPN；研发相关的代码仓库、流水线和数据库权限需要直属主管二次申请。"
            ),
            (
                "VPN远程接入管理办法",
                "说明远程办公场景下 VPN 账号申请、双因素认证和例外审批要求。",
                "1. 员工申请 VPN 时需填写远程办公原因、使用周期和终端设备信息。\n2. VPN 账号必须绑定企业 MFA，禁止多人共用同一账号。\n3. 海外访问、外包人员访问和生产环境跳板访问属于高风险场景，需要安全经理审批。\n4. 连续 30 天未使用的 VPN 账号将自动停用，恢复使用需重新提交申请。",
                "运维规范",
                vec!["VPN", "远程办公", "安全"],
                "哪些 VPN 场景需要额外审批？",
                "海外访问、外包人员访问以及生产环境跳板访问都属于高风险场景，需要安全经理额外审批。"
            ),
        ]
    }
    async fn ensure_bootstrap_documents(&self) -> Result<(), sqlx::Error> {
        for (title, summary, content, category_name, tags, faq_question, faq_answer) in Self::bootstrap_documents() {
            let mut tx = self.pool.begin().await?;
            let category_id = Self::ensure_category(&mut tx, category_name).await?;

            let existing = sqlx::query(
                "SELECT document_id, current_version_id FROM documents WHERE title = ? LIMIT 1",
            )
            .bind(title)
            .fetch_optional(tx.as_mut())
            .await?;

            let document_id = if let Some(row) = existing {
                let document_id = row.get::<i64, _>("document_id");
                let current_version_id = row.try_get::<Option<i64>, _>("current_version_id").ok().flatten();

                sqlx::query(
                    "UPDATE documents
                     SET category_id = ?, title = ?, summary = ?, status = 'published', current_version_no = 'v1.0.0',
                         published_at = COALESCE(published_at, UTC_TIMESTAMP()), updated_at = UTC_TIMESTAMP()
                     WHERE document_id = ?",
                )
                .bind(category_id)
                .bind(title)
                .bind(summary)
                .bind(document_id)
                .execute(tx.as_mut())
                .await?;

                let version_id = if let Some(version_id) = current_version_id {
                    let segments_are_referenced = Self::bootstrap_segments_are_referenced(&mut tx, document_id, version_id).await?;

                    if !segments_are_referenced {
                        sqlx::query(
                            "UPDATE document_versions
                             SET version_no = 'v1.0.0', title = ?, content = ?, summary = ?, change_note = ?,
                                 is_published_snapshot = 1, created_by = 1
                             WHERE version_id = ?",
                        )
                        .bind(title)
                        .bind(content)
                        .bind(summary)
                        .bind("初始化演示文档")
                        .bind(version_id)
                        .execute(tx.as_mut())
                        .await?;
                    }
                    version_id
                } else {
                    let insert_version = sqlx::query(
                        "INSERT INTO document_versions (
                            document_id, version_no, title, content, summary, change_note, is_published_snapshot, created_by, created_at
                         ) VALUES (?, 'v1.0.0', ?, ?, ?, ?, 1, 1, UTC_TIMESTAMP())",
                    )
                    .bind(document_id)
                    .bind(title)
                    .bind(content)
                    .bind(summary)
                    .bind("初始化演示文档")
                    .execute(tx.as_mut())
                    .await?;
                    insert_version.last_insert_id() as i64
                };

                sqlx::query(
                    "UPDATE documents SET current_version_id = ?, updated_at = UTC_TIMESTAMP() WHERE document_id = ?",
                )
                .bind(version_id)
                .bind(document_id)
                .execute(tx.as_mut())
                .await?;

                if !Self::bootstrap_segments_are_referenced(&mut tx, document_id, version_id).await? {
                    Self::replace_document_segments(&mut tx, document_id, version_id, content).await?;
                }
                document_id
            } else {
                let insert_document = sqlx::query(
                    "INSERT INTO documents (
                        category_id, creator_id, current_version_no, title, summary, status, published_at, created_at, updated_at
                     ) VALUES (?, 1, 'v1.0.0', ?, ?, 'published', UTC_TIMESTAMP(), UTC_TIMESTAMP(), UTC_TIMESTAMP())",
                )
                .bind(category_id)
                .bind(title)
                .bind(summary)
                .execute(tx.as_mut())
                .await?;
                let document_id = insert_document.last_insert_id() as i64;

                let insert_version = sqlx::query(
                    "INSERT INTO document_versions (
                        document_id, version_no, title, content, summary, change_note, is_published_snapshot, created_by, created_at
                     ) VALUES (?, 'v1.0.0', ?, ?, ?, ?, 1, 1, UTC_TIMESTAMP())",
                )
                .bind(document_id)
                .bind(title)
                .bind(content)
                .bind(summary)
                .bind("初始化演示文档")
                .execute(tx.as_mut())
                .await?;
                let version_id = insert_version.last_insert_id() as i64;

                sqlx::query(
                    "UPDATE documents SET current_version_id = ?, updated_at = UTC_TIMESTAMP() WHERE document_id = ?",
                )
                .bind(version_id)
                .bind(document_id)
                .execute(tx.as_mut())
                .await?;

                Self::replace_document_segments(&mut tx, document_id, version_id, content).await?;
                document_id
            };

            let version_id = sqlx::query_scalar::<_, i64>(
                "SELECT current_version_id FROM documents WHERE document_id = ?",
            )
            .bind(document_id)
            .fetch_one(tx.as_mut())
            .await?;

            Self::replace_document_tags(
                &mut tx,
                document_id,
                &tags.iter().map(|item| item.to_string()).collect::<Vec<_>>(),
            )
            .await?;
            Self::replace_document_faq_item(&mut tx, document_id, faq_question, faq_answer).await?;

            sqlx::query(
                "DELETE FROM agent_runs
                 WHERE document_id = ? AND trigger_type = 'document_publish' AND meta_json = ?",
            )
            .bind(document_id)
            .bind(r#"{"source":"bootstrap"}"#)
            .execute(tx.as_mut())
            .await?;

            Self::create_agent_run(
                &mut tx,
                1,
                "summary",
                "document_publish",
                Some(document_id),
                Some(version_id),
                None,
                None,
                "success",
                title,
                "演示知识已初始化",
                Some(r#"{"source":"bootstrap"}"#),
            )
            .await?;

            tx.commit().await?;
        }

        Ok(())
    }

    async fn bootstrap_segments_are_referenced(
        tx: &mut Transaction<'_, MySql>,
        document_id: i64,
        version_id: i64,
    ) -> Result<bool, sqlx::Error> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)
             FROM answer_citations ac
             JOIN document_segments ds ON ds.segment_id = ac.segment_id
             WHERE ds.document_id = ?
               AND ds.version_id = ?",
        )
        .bind(document_id)
        .bind(version_id)
        .fetch_one(tx.as_mut())
        .await?;

        Ok(count > 0)
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

    async fn replace_document_faq_item(
        tx: &mut Transaction<'_, MySql>,
        document_id: i64,
        question: &str,
        answer: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM faq_items WHERE document_id = ?")
            .bind(document_id)
            .execute(tx.as_mut())
            .await?;

        sqlx::query(
            "INSERT INTO faq_items (document_id, question, answer, status) VALUES (?, ?, ?, 'active')",
        )
        .bind(document_id)
        .bind(question)
        .bind(answer)
        .execute(tx.as_mut())
        .await?;

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
        document_id: Option<i64>,
        version_id: Option<i64>,
        question_id: Option<i64>,
        answer_id: Option<i64>,
        status: &str,
        input_text: &str,
        output_text: &str,
        meta_json: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO agent_runs (
                agent_type, trigger_type, operator_user_id, document_id, version_id, question_id, answer_id, status, input_text, output_text, meta_json, started_at, finished_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(agent_type)
        .bind(trigger_type)
        .bind(operator_user_id as i64)
        .bind(document_id)
        .bind(version_id)
        .bind(question_id)
        .bind(answer_id)
        .bind(status)
        .bind(input_text)
        .bind(output_text)
        .bind(meta_json)
        .execute(tx.as_mut())
        .await?;

        Ok(())
    }

    fn build_segments(content: &str) -> Vec<(u32, String, u32)> {
        content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .enumerate()
            .map(|(index, line)| ((index + 1) as u32, line.to_string(), line.chars().count() as u32))
            .collect()
    }

    fn first_line(text: &str) -> String {
        text.lines()
            .map(str::trim)
            .find(|line| !line.is_empty())
            .unwrap_or("内容为空")
            .to_string()
    }

    fn search_score(question: &str, text: &str) -> usize {
        let normalized_question = question.to_lowercase();
        let normalized_text = text.to_lowercase();
        normalized_question
            .chars()
            .filter(|ch| !ch.is_whitespace() && !matches!(ch, '，' | '。' | '？' | '?' | '！' | '!' | '、' | ',' | '.'))
            .filter(|ch| normalized_text.contains(*ch))
            .count()
    }

    async fn replace_document_segments(
        tx: &mut Transaction<'_, MySql>,
        document_id: i64,
        version_id: i64,
        content: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM document_segments WHERE document_id = ? AND version_id = ?")
            .bind(document_id)
            .bind(version_id)
            .execute(tx.as_mut())
            .await?;

        for (chunk_order, chunk_text, token_count) in Self::build_segments(content) {
            sqlx::query(
                "INSERT INTO document_segments (
                    version_id, document_id, chunk_order, chunk_text, token_count, is_active
                 ) VALUES (?, ?, ?, ?, ?, 1)",
            )
            .bind(version_id)
            .bind(document_id)
            .bind(chunk_order as i32)
            .bind(chunk_text)
            .bind(token_count as i32)
            .execute(tx.as_mut())
            .await?;
        }

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
            "SELECT version_id, version_no, source_file_id, title, content, summary, change_note, created_at
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
                source_file_id: row
                    .try_get::<Option<i64>, _>("source_file_id")
                    .ok()
                    .flatten()
                    .map(|value| value as u64),
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

    async fn load_segments(&self, document_id: i64) -> Result<Vec<DocumentSegment>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT segment_id, version_id, document_id, chunk_order, chunk_text, token_count,
                    CASE WHEN is_active = 1 THEN 'active' ELSE 'inactive' END AS embedding_status,
                    created_at
             FROM document_segments
             WHERE document_id = ?
             ORDER BY version_id, chunk_order, segment_id",
        )
        .bind(document_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| DocumentSegment {
                segment_id: row.get::<i64, _>("segment_id") as u64,
                version_id: row.get::<i64, _>("version_id") as u64,
                document_id: row.get::<i64, _>("document_id") as u64,
                chunk_order: row.get::<i32, _>("chunk_order") as u32,
                chunk_text: row.get::<String, _>("chunk_text"),
                token_count: row
                    .try_get::<Option<i32>, _>("token_count")
                    .ok()
                    .flatten()
                    .map(|value| value as u32),
                embedding_status: row
                    .try_get::<Option<String>, _>("embedding_status")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "inactive".to_string()),
                created_at: Self::mysql_dt_to_utc(row.get("created_at")),
            })
            .collect())
    }

    async fn load_document_file(&self, file_id: i64) -> Result<Option<DocumentFileMeta>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT file_id, object_key, original_name, mime_type, file_size, sha256, created_at
             FROM document_files
             WHERE file_id = ?",
        )
        .bind(file_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|row| DocumentFileMeta {
            file_id: row.get::<i64, _>("file_id") as u64,
            object_key: row.get::<String, _>("object_key"),
            original_name: row.get::<String, _>("original_name"),
            mime_type: row.get::<String, _>("mime_type"),
            file_size: row.get::<i64, _>("file_size") as u64,
            sha256: row.try_get::<Option<String>, _>("sha256").ok().flatten(),
            created_at: Self::mysql_dt_to_utc(row.get("created_at")),
        }))
    }

    async fn load_document_files(&self) -> Result<Vec<DocumentFileMeta>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT file_id, object_key, original_name, mime_type, file_size, sha256, created_at
             FROM document_files
             ORDER BY created_at DESC, file_id DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|row| DocumentFileMeta {
                file_id: row.get::<i64, _>("file_id") as u64,
                object_key: row.get::<String, _>("object_key"),
                original_name: row.get::<String, _>("original_name"),
                mime_type: row.get::<String, _>("mime_type"),
                file_size: row.get::<i64, _>("file_size") as u64,
                sha256: row.try_get::<Option<String>, _>("sha256").ok().flatten(),
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
                    CASE WHEN fr.favorite_id IS NULL THEN 0 ELSE 1 END AS is_favorite,
                    df.file_id AS source_file_id
             FROM documents d
             JOIN categories c ON d.category_id = c.category_id
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             LEFT JOIN document_files df ON d.source_file_id = df.file_id
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
        let source_file = match row
            .try_get::<Option<i64>, _>("source_file_id")
            .ok()
            .flatten()
        {
            Some(file_id) => self.load_document_file(file_id).await?,
            None => None,
        };

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
            source_file,
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

    async fn list_document_segments(&self, id: u64) -> Option<Vec<DocumentSegment>> {
        self.load_segments(id as i64).await.ok()
    }

    async fn list_document_files(&self) -> Vec<DocumentFileMeta> {
        self.load_document_files().await.unwrap_or_default()
    }

    async fn get_document_file(&self, file_id: u64) -> Option<DocumentFileMeta> {
        self.load_document_file(file_id as i64).await.ok().flatten()
    }

    async fn register_document_file(
        &self,
        _user_id: u64,
        payload: RegisterDocumentFileRequest,
    ) -> Result<DocumentFileMeta, StoreMutationError> {
        let result = sqlx::query(
            "INSERT INTO document_files (object_key, original_name, mime_type, file_size, sha256)
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(payload.object_key.unwrap_or_else(|| {
            format!(
                "minio/pending/{}-{}",
                Utc::now().timestamp_millis(),
                payload.original_name.replace(['\\', '/', ' '], "-")
            )
        }))
        .bind(&payload.original_name)
        .bind(&payload.mime_type)
        .bind(payload.file_size as i64)
        .bind(&payload.sha256)
        .execute(&self.pool)
        .await
        .map_err(|_| StoreMutationError::Conflict)?;

        self.load_document_file(result.last_insert_id() as i64)
            .await
            .map_err(|_| StoreMutationError::Conflict)?
            .ok_or(StoreMutationError::NotFound)
    }

    async fn create_document(&self, user_id: u64, payload: CreateDocumentRequest) -> DocumentDetail {
        let mut tx = self.pool.begin().await.expect("begin create document tx");
        let category_id = Self::ensure_category(&mut tx, &payload.category_name)
            .await
            .expect("ensure category");

        let insert_document = sqlx::query(
            "INSERT INTO documents (
                category_id, creator_id, current_version_no, title, summary, status, source_file_id, created_at, updated_at
             ) VALUES (?, ?, 'v1.0.0', ?, ?, 'draft', ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(category_id)
        .bind(user_id as i64)
        .bind(&payload.title)
        .bind(&payload.summary)
        .bind(payload.source_file_id.map(|value| value as i64))
        .execute(tx.as_mut())
        .await
        .expect("insert document");
        let document_id = insert_document.last_insert_id() as i64;

        let insert_version = sqlx::query(
            "INSERT INTO document_versions (
                document_id, version_no, title, content, summary, change_note, source_file_id, is_published_snapshot, created_by, created_at
             ) VALUES (?, 'v1.0.0', ?, ?, ?, ?, ?, 0, ?, UTC_TIMESTAMP())",
        )
        .bind(document_id)
        .bind(&payload.title)
        .bind(&payload.content)
        .bind(&payload.summary)
        .bind(&payload.change_note)
        .bind(payload.source_file_id.map(|value| value as i64))
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
            Some(document_id),
            Some(version_id),
            None,
            None,
            "success",
            &payload.title,
            "新文档已写入并生成初始摘要",
            Some("{\"stage\":\"create_document\"}"),
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
        let existing_source_file_id = sqlx::query_scalar::<_, i64>(
            "SELECT source_file_id FROM documents WHERE document_id = ?",
        )
        .bind(document_id)
        .fetch_optional(tx.as_mut())
        .await
        .ok()
        .flatten();
        let next_source_file_id = payload
            .source_file_id
            .map(|value| value as i64)
            .or(existing_source_file_id);

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
                document_id, version_no, title, content, summary, change_note, source_file_id, is_published_snapshot, created_by, created_at
             ) VALUES (?, ?, ?, ?, ?, ?, ?, 0, ?, UTC_TIMESTAMP())",
        )
        .bind(document_id)
        .bind(&version_no)
        .bind(&payload.title)
        .bind(&payload.content)
        .bind(&payload.summary)
        .bind(&payload.change_note)
        .bind(next_source_file_id)
        .bind(user_id as i64)
        .execute(tx.as_mut())
        .await
        .ok()?;
        let version_id = version_result.last_insert_id() as i64;

        sqlx::query(
            "UPDATE documents
             SET category_id = ?, current_version_id = ?, current_version_no = ?, title = ?, summary = ?, source_file_id = ?, updated_at = UTC_TIMESTAMP()
             WHERE document_id = ?",
        )
        .bind(category_id)
        .bind(version_id)
        .bind(&version_no)
        .bind(&payload.title)
        .bind(&payload.summary)
        .bind(next_source_file_id)
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
            Some(document_id),
            Some(version_id),
            None,
            None,
            "success",
            &payload.title,
            "文档已更新并生成新版本",
            Some(&format!("{{\"version_no\":\"{}\"}}", version_no)),
        )
        .await
        .ok()?;

        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }

    async fn publish_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let document_id = id as i64;
        let mut tx = self.pool.begin().await.ok()?;
        let row = sqlx::query(
            "SELECT d.title, d.current_version_id, dv.content
             FROM documents d
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             WHERE d.document_id = ?",
        )
            .bind(document_id)
            .fetch_optional(tx.as_mut())
            .await
            .ok()??;
        let title = row.get::<String, _>("title");
        let current_version_id = row.try_get::<Option<i64>, _>("current_version_id").ok().flatten();
        let current_content = row
            .try_get::<Option<String>, _>("content")
            .ok()
            .flatten()
            .unwrap_or_default();

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
            Self::replace_document_segments(&mut tx, document_id, version_id, &current_content)
                .await
                .ok()?;
        }

        Self::create_agent_run(
            &mut tx,
            user_id,
            "audit",
            "document_publish",
            Some(document_id),
            current_version_id,
            None,
            None,
            "success",
            &title,
            "文档发布成功",
            Some("{\"action\":\"publish\"}"),
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
            Some(document_id),
            None,
            None,
            None,
            "success",
            &title,
            "文档已归档",
            Some("{\"action\":\"archive\"}"),
        )
        .await
        .ok()?;

        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }

    async fn ask_question(&self, user_id: u64, question_text: String) -> QaAnswer {
        let mut tx = self.pool.begin().await.expect("begin ask question tx");
        let candidate_rows = sqlx::query(
            "SELECT d.document_id, d.title, d.current_version_id, d.current_version_no, dv.content,
                    ds.segment_id, ds.chunk_text, ds.chunk_order
             FROM documents d
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             LEFT JOIN document_segments ds
               ON ds.document_id = d.document_id
              AND ds.version_id = d.current_version_id
             WHERE d.status <> 'archived'
             ORDER BY d.document_id DESC, ds.chunk_order ASC",
        )
        .fetch_all(tx.as_mut())
        .await
        .expect("load context document");

        let mut citations = candidate_rows
            .iter()
            .map(|row| {
                let document_title = row.get::<String, _>("title");
                let version_no = row
                    .try_get::<Option<String>, _>("current_version_no")
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "v0".to_string());
                let content = row
                    .try_get::<Option<String>, _>("content")
                    .ok()
                    .flatten()
                    .unwrap_or_default();
                let snippet_text = row
                    .try_get::<Option<String>, _>("chunk_text")
                    .ok()
                    .flatten()
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| Self::first_line(&content));
                let score = Self::search_score(&question_text, &snippet_text) as f32;
                (
                    score,
                    row.get::<i64, _>("document_id"),
                    row.try_get::<Option<i32>, _>("chunk_order").ok().flatten().unwrap_or(0),
                    Citation {
                        cite_order: 0,
                        segment_id: row.try_get::<Option<i64>, _>("segment_id").ok().flatten().map(|value| value as u64),
                        document_title,
                        version_no,
                        snippet_text,
                        score: (score / 10.0).min(0.99),
                    },
                )
            })
            .filter(|(score, _, _, citation)| *score > 0.0 || !citation.snippet_text.trim().is_empty())
            .collect::<Vec<_>>();

        citations.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| right.1.cmp(&left.1))
                .then_with(|| left.2.cmp(&right.2))
        });

        let mut citations = citations
            .into_iter()
            .take(3)
            .enumerate()
            .map(|(index, (_, _, _, mut citation))| {
                citation.cite_order = (index + 1) as u32;
                citation
            })
            .collect::<Vec<_>>();

        if citations.is_empty() {
            citations.push(Citation {
                cite_order: 1,
                segment_id: None,
                document_title: "知识库演示文档".to_string(),
                version_no: "v0".to_string(),
                snippet_text: "当前未找到高置信度证据，请尝试提出更具体的问题。".to_string(),
                score: 0.0,
            });
        }

        let llm_answer = generate_answer(&question_text, &citations).await;
        let answer_text = llm_answer
            .as_ref()
            .map(|item| item.answer_text.clone())
            .unwrap_or_else(|| {
                let top = &citations[0];
                format!(
                    "根据当前知识库，最匹配的证据来自 {}：{}",
                    top.document_title,
                    top.snippet_text
                )
            });
        let confidence_score = llm_answer
            .as_ref()
            .map(|item| item.confidence_score)
            .unwrap_or(if citations[0].score > 0.0 { 0.84 } else { 0.58 });
        let model_name = llm_answer
            .as_ref()
            .map(|item| item.model_name.as_str())
            .unwrap_or("demo-rag-agent");

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
             VALUES (?, ?, ?, ?, 'success', 120, UTC_TIMESTAMP())",
        )
        .bind(question_id)
        .bind(&answer_text)
        .bind(confidence_score)
        .bind(model_name)
        .execute(tx.as_mut())
        .await
        .expect("insert answer");
        let answer_id = insert_answer.last_insert_id() as i64;

        for citation in &citations {
            let source_row = candidate_rows.iter().find(|row| {
                row.get::<String, _>("title") == citation.document_title
                    && row
                        .try_get::<Option<String>, _>("current_version_no")
                        .ok()
                        .flatten()
                        .unwrap_or_else(|| "v0".to_string())
                        == citation.version_no
                    && row
                        .try_get::<Option<String>, _>("chunk_text")
                        .ok()
                        .flatten()
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| {
                            let content = row
                                .try_get::<Option<String>, _>("content")
                                .ok()
                                .flatten()
                                .unwrap_or_default();
                            Self::first_line(&content)
                        })
                        == citation.snippet_text
            });

            let document_id = source_row.map(|row| row.get::<i64, _>("document_id")).unwrap_or(0);
            let version_id = source_row
                .and_then(|row| row.try_get::<Option<i64>, _>("current_version_id").ok().flatten())
                .unwrap_or(0);

            sqlx::query(
                "INSERT INTO answer_citations (
                    answer_id, document_id, version_id, segment_id, cite_order, score, snippet_text
                 ) VALUES (?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(answer_id)
            .bind(document_id)
            .bind(version_id)
            .bind(citation.segment_id.map(|value| value as i64))
            .bind(citation.cite_order as i32)
            .bind(citation.score)
            .bind(&citation.snippet_text)
            .execute(tx.as_mut())
            .await
            .expect("insert citation");
        }

        let meta_json = if llm_answer.is_some() {
            format!("{{\"provider\":\"zhipu\",\"citation_count\":{}}}", citations.len())
        } else {
            format!("{{\"provider\":\"demo-rag\",\"citation_count\":{}}}", citations.len())
        };

        sqlx::query(
            "INSERT INTO agent_runs (
                agent_type, trigger_type, operator_user_id, question_id, answer_id, status, input_text, output_text, meta_json, started_at, finished_at
             ) VALUES ('answer', 'question_submit', ?, ?, ?, 'success', ?, ?, ?, UTC_TIMESTAMP(), UTC_TIMESTAMP())",
        )
        .bind(user_id as i64)
        .bind(question_id)
        .bind(answer_id)
        .bind(&question_text)
        .bind(&answer_text)
        .bind(&meta_json)
        .execute(tx.as_mut())
        .await
        .expect("insert agent run");

        tx.commit().await.expect("commit ask question");

        QaAnswer {
            answer_id: answer_id as u64,
            answer_text,
            confidence_score,
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
            "SELECT run_id, agent_type, trigger_type, document_id, version_id, question_id, answer_id,
                    status, input_text, output_text, meta_json, started_at, finished_at
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
                document_id: row.try_get::<Option<i64>, _>("document_id").ok().flatten().map(|value| value as u64),
                version_id: row.try_get::<Option<i64>, _>("version_id").ok().flatten().map(|value| value as u64),
                question_id: row.try_get::<Option<i64>, _>("question_id").ok().flatten().map(|value| value as u64),
                answer_id: row.try_get::<Option<i64>, _>("answer_id").ok().flatten().map(|value| value as u64),
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
                meta_json: row.try_get::<Option<String>, _>("meta_json").ok().flatten(),
                started_at: Self::mysql_dt_to_utc(row.get("started_at")),
                finished_at: row
                    .try_get::<Option<chrono::NaiveDateTime>, _>("finished_at")
                    .ok()
                    .flatten()
                    .map(Self::mysql_dt_to_utc),
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

    async fn reset_user_password(
        &self,
        user_id: u64,
        password: String,
    ) -> Result<UserItem, StoreMutationError> {
        let affected = sqlx::query("UPDATE users SET password_hash = ? WHERE user_id = ?")
            .bind(hash_password(&password))
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

    async fn delete_user(&self, user_id: u64) -> Result<DeletedResource, StoreMutationError> {
        if user_id == 1 {
            return Err(StoreMutationError::Conflict);
        }
        let user_id_i64 = user_id as i64;
        let mut tx = self.pool.begin().await.map_err(|_| StoreMutationError::Conflict)?;
        sqlx::query("DELETE FROM favorite_records WHERE user_id = ?")
            .bind(user_id_i64)
            .execute(tx.as_mut())
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        sqlx::query("DELETE FROM read_records WHERE user_id = ?")
            .bind(user_id_i64)
            .execute(tx.as_mut())
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        let affected = sqlx::query("DELETE FROM users WHERE user_id = ?")
            .bind(user_id_i64)
            .execute(tx.as_mut())
            .await
            .map_err(|_| StoreMutationError::Conflict)?;
        if affected.rows_affected() == 0 {
            return Err(StoreMutationError::NotFound);
        }
        tx.commit().await.map_err(|_| StoreMutationError::Conflict)?;
        Ok(DeletedResource {
            resource_type: "user".to_string(),
            resource_key: user_id.to_string(),
        })
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

    async fn reindex_document(&self, user_id: u64, id: u64) -> Option<DocumentDetail> {
        let document_id = id as i64;
        let mut tx = self.pool.begin().await.ok()?;
        let row = sqlx::query(
            "SELECT d.title, d.current_version_id, dv.content
             FROM documents d
             LEFT JOIN document_versions dv ON dv.version_id = d.current_version_id
             WHERE d.document_id = ?",
        )
        .bind(document_id)
        .fetch_optional(tx.as_mut())
        .await
        .ok()??;
        let title = row.get::<String, _>("title");
        let version_id = row.try_get::<Option<i64>, _>("current_version_id").ok().flatten()?;
        let content = row
            .try_get::<Option<String>, _>("content")
            .ok()
            .flatten()
            .unwrap_or_default();
        Self::replace_document_segments(&mut tx, document_id, version_id, &content)
            .await
            .ok()?;
        Self::create_agent_run(
            &mut tx,
            user_id,
            "index",
            "document_reindex",
            Some(document_id),
            Some(version_id),
            None,
            None,
            "success",
            &title,
            "文档分段已重建",
            Some("{\"action\":\"reindex\"}"),
        )
        .await
        .ok()?;
        tx.commit().await.ok()?;
        self.load_document_detail_for_user(user_id, document_id).await.ok().flatten()
    }
}


