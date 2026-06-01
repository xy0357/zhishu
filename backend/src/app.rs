use axum::{
    routing::{get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::AppConfig,
    routes::{agent_runs, auth, behaviors, categories, dashboard, documents, faq, health, qa, tags, users},
    store::DynStore,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub store: DynStore,
}

pub fn build_router(config: AppConfig, store: DynStore) -> Router {
    let state = AppState {
        config,
        store,
    };

    Router::new()
        .route("/api/health", get(health::health_check))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/me", get(auth::current_user))
        .route("/api/dashboard", get(dashboard::dashboard_summary))
        .route("/api/categories", get(categories::list_categories).post(categories::create_category))
        .route("/api/categories/:name", axum::routing::put(categories::update_category).delete(categories::delete_category))
        .route("/api/tags", get(tags::list_tags).post(tags::create_tag))
        .route("/api/tags/:name", axum::routing::put(tags::update_tag).delete(tags::delete_tag))
        .route("/api/roles", get(users::list_roles))
        .route("/api/users", get(users::list_users).post(users::create_user))
        .route("/api/users/:id", axum::routing::put(users::update_user))
        .route("/api/favorites", get(behaviors::list_favorites))
        .route("/api/read-records/recent", get(behaviors::list_recent_reads))
        .route("/api/documents", get(documents::list_documents).post(documents::create_document))
        .route("/api/documents/:id", get(documents::get_document).put(documents::update_document))
        .route("/api/documents/:id/versions", get(documents::list_versions))
        .route("/api/documents/:id/faqs", get(faq::list_document_faq).post(faq::create_document_faq))
        .route("/api/faqs/:id", axum::routing::put(faq::update_faq).delete(faq::delete_faq))
        .route("/api/documents/:id/read", post(behaviors::record_document_read))
        .route("/api/documents/:id/favorite", post(behaviors::toggle_favorite_document))
        .route("/api/documents/:id/publish", post(documents::publish_document))
        .route("/api/documents/:id/archive", post(documents::archive_document))
        .route("/api/qa/ask", post(qa::ask_question))
        .route("/api/questions/history", get(qa::list_question_history))
        .route("/api/agent-runs", get(agent_runs::list_agent_runs))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::{env, sync::Arc};

    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;
    use uuid::Uuid;

    use crate::{config::AppConfig, security::issue_test_access_token, store::memory::MemoryStore};

    use super::build_router;

    fn temp_store_path(test_name: &str) -> String {
        env::temp_dir()
            .join(format!("zhishu-router-{}-{}.json", test_name, Uuid::new_v4()))
            .to_string_lossy()
            .to_string()
    }

    #[tokio::test]
    async fn management_routes_should_support_category_tag_and_faq_changes() {
        let store_path = temp_store_path("manage-routes");
        let config = AppConfig::from_env();
        let app = build_router(config.clone(), Arc::new(MemoryStore::from_path(&store_path)));
        let auth_value = format!("Bearer {}", issue_test_access_token(&config, "admin"));

        let create_category = app
            .clone()
            .oneshot(
                Request::post("/api/categories")
                    .header("authorization", &auth_value)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"category_name":"运维规范","description":"运维制度与规范"}"#,
                    ))
                    .expect("category request"),
            )
            .await
            .expect("category response");
        assert_eq!(create_category.status(), StatusCode::CREATED);

        let create_tag = app
            .clone()
            .oneshot(
                Request::post("/api/tags")
                    .header("authorization", &auth_value)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"tag_name":"手册","description":"操作手册标签"}"#,
                    ))
                    .expect("tag request"),
            )
            .await
            .expect("tag response");
        assert_eq!(create_tag.status(), StatusCode::CREATED);

        let create_faq = app
            .clone()
            .oneshot(
                Request::post("/api/documents/1/faqs")
                    .header("authorization", &auth_value)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"question":"权限开通后如何确认？","answer":"由申请人登录后自行验证。"}"#,
                    ))
                    .expect("faq request"),
            )
            .await
            .expect("faq response");
        assert_eq!(create_faq.status(), StatusCode::CREATED);

        let body = to_bytes(create_faq.into_body(), usize::MAX)
            .await
            .expect("faq body");
        let json: serde_json::Value = serde_json::from_slice(&body).expect("faq json");
        let faq_id = json["data"]["faq_id"].as_u64().expect("faq id");

        let delete_faq = app
            .oneshot(
                Request::delete(format!("/api/faqs/{}", faq_id))
                    .header("authorization", &auth_value)
                    .body(Body::empty())
                    .expect("delete faq request"),
            )
            .await
            .expect("delete faq response");
        assert_eq!(delete_faq.status(), StatusCode::OK);

        let _ = std::fs::remove_file(store_path);
    }

    #[tokio::test]
    async fn auth_routes_should_support_login_and_me() {
        let store_path = temp_store_path("auth-routes");
        let app = build_router(
            AppConfig::from_env(),
            Arc::new(MemoryStore::from_path(&store_path)),
        );

        let login = app
            .clone()
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"admin","password":"Admin@123456"}"#,
                    ))
                    .expect("login request"),
            )
            .await
            .expect("login response");
        assert_eq!(login.status(), StatusCode::OK);

        let login_body = to_bytes(login.into_body(), usize::MAX)
            .await
            .expect("login body");
        let login_json: serde_json::Value =
            serde_json::from_slice(&login_body).expect("login json");
        let token = login_json["data"]["access_token"]
            .as_str()
            .expect("access token");

        let me = app
            .oneshot(
                Request::get("/api/auth/me")
                    .header("authorization", format!("Bearer {}", token))
                    .body(Body::empty())
                    .expect("me request"),
            )
            .await
            .expect("me response");
        assert_eq!(me.status(), StatusCode::OK);

        let _ = std::fs::remove_file(store_path);
    }

    #[tokio::test]
    async fn editor_should_be_forbidden_from_admin_routes_but_allowed_for_content_routes() {
        let store_path = temp_store_path("role-routes");
        let config = AppConfig::from_env();
        let app = build_router(config.clone(), Arc::new(MemoryStore::from_path(&store_path)));
        let editor_auth = format!("Bearer {}", issue_test_access_token(&config, "editor"));

        let users = app
            .clone()
            .oneshot(
                Request::get("/api/users")
                    .header("authorization", &editor_auth)
                    .body(Body::empty())
                    .expect("users request"),
            )
            .await
            .expect("users response");
        assert_eq!(users.status(), StatusCode::FORBIDDEN);

        let categories = app
            .clone()
            .oneshot(
                Request::post("/api/categories")
                    .header("authorization", &editor_auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"category_name":"编辑不可建","description":"应被禁止"}"#,
                    ))
                    .expect("categories request"),
            )
            .await
            .expect("categories response");
        assert_eq!(categories.status(), StatusCode::FORBIDDEN);

        let create_document = app
            .clone()
            .oneshot(
                Request::post("/api/documents")
                    .header("authorization", &editor_auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"title":"编辑器文档","summary":"编辑可创建文档","content":"1. 编辑可维护内容。","category_name":"制度流程","tags":["编辑"],"change_note":"初始化"}"#,
                    ))
                    .expect("document request"),
            )
            .await
            .expect("document response");
        assert_eq!(create_document.status(), StatusCode::CREATED);

        let create_faq = app
            .clone()
            .oneshot(
                Request::post("/api/documents/1/faqs")
                    .header("authorization", &editor_auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"question":"编辑能维护FAQ吗？","answer":"可以，属于内容维护权限。"}"#,
                    ))
                    .expect("faq request"),
            )
            .await
            .expect("faq response");
        assert_eq!(create_faq.status(), StatusCode::CREATED);

        let agent_runs = app
            .oneshot(
                Request::get("/api/agent-runs")
                    .header("authorization", &editor_auth)
                    .body(Body::empty())
                    .expect("agent runs request"),
            )
            .await
            .expect("agent runs response");
        assert_eq!(agent_runs.status(), StatusCode::FORBIDDEN);

        let _ = std::fs::remove_file(store_path);
    }

    #[tokio::test]
    async fn admin_should_manage_users_and_new_user_should_login() {
        let store_path = temp_store_path("user-routes");
        let config = AppConfig::from_env();
        let app = build_router(config.clone(), Arc::new(MemoryStore::from_path(&store_path)));
        let admin_auth = format!("Bearer {}", issue_test_access_token(&config, "admin"));

        let create_user = app
            .clone()
            .oneshot(
                Request::post("/api/users")
                    .header("authorization", &admin_auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"viewer","role_name":"普通用户","department":"财务","email":"viewer@example.com","password":"Viewer@123456"}"#,
                    ))
                    .expect("create user request"),
            )
            .await
            .expect("create user response");
        assert_eq!(create_user.status(), StatusCode::CREATED);

        let update_user = app
            .clone()
            .oneshot(
                Request::put("/api/users/3")
                    .header("authorization", &admin_auth)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"viewer","role_name":"知识管理员","department":"知识运营","email":"viewer2@example.com","password":"Viewer@654321"}"#,
                    ))
                    .expect("update user request"),
            )
            .await
            .expect("update user response");
        assert_eq!(update_user.status(), StatusCode::OK);

        let login = app
            .oneshot(
                Request::post("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"username":"viewer","password":"Viewer@654321"}"#,
                    ))
                    .expect("viewer login request"),
            )
            .await
            .expect("viewer login response");
        assert_eq!(login.status(), StatusCode::OK);

        let _ = std::fs::remove_file(store_path);
    }
}
