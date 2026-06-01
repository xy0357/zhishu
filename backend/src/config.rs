use std::env;

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub app_name: String,
    pub storage_backend: String,
    pub storage_file: String,
    pub mysql_url: String,
    pub access_token_secret: String,
    pub access_token_ttl_hours: i64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            host: env::var("APP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("APP_PORT")
                .ok()
                .and_then(|value| value.parse::<u16>().ok())
                .unwrap_or(8080),
            app_name: env::var("APP_NAME").unwrap_or_else(|_| "zhishu-backend".to_string()),
            storage_backend: env::var("APP_STORAGE_BACKEND")
                .unwrap_or_else(|_| "file".to_string()),
            storage_file: env::var("APP_STORAGE_FILE")
                .unwrap_or_else(|_| "data/demo-store.json".to_string()),
            mysql_url: env::var("MYSQL_URL")
                .unwrap_or_else(|_| "mysql://zhishu:zhishu@127.0.0.1:3306/zhishu".to_string()),
            access_token_secret: env::var("APP_ACCESS_TOKEN_SECRET")
                .unwrap_or_else(|_| "zhishu-dev-secret-change-me".to_string()),
            access_token_ttl_hours: env::var("APP_ACCESS_TOKEN_TTL_HOURS")
                .ok()
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(12),
        }
    }
}
