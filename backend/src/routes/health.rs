use std::time::Duration;

use axum::{extract::State, Json};
use tokio::{net::TcpStream, time::timeout};

use crate::{app::AppState, models::ApiResponse};

async fn test_tcp_endpoint(host: &str, port: u16) -> bool {
    matches!(
        timeout(Duration::from_millis(800), TcpStream::connect((host, port))).await,
        Ok(Ok(_))
    )
}

fn parse_host_port(value: &str, default_host: &str, default_port: u16) -> (String, u16) {
    let trimmed = value.trim();
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .or_else(|| trimmed.strip_prefix("redis://"))
        .unwrap_or(trimmed);
    let host_port = without_scheme.split('/').next().unwrap_or(without_scheme);
    let mut parts = host_port.split(':');
    let host = parts
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(default_host)
        .to_string();
    let port = parts
        .next()
        .and_then(|part| part.parse::<u16>().ok())
        .unwrap_or(default_port);
    (host, port)
}

fn parse_mysql_host_port(value: &str, default_host: &str, default_port: u16) -> (String, u16) {
    let trimmed = value.trim();
    let without_scheme = trimmed.strip_prefix("mysql://").unwrap_or(trimmed);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let mut parts = host_port.split(':');
    let host = parts
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or(default_host)
        .to_string();
    let port = parts
        .next()
        .and_then(|part| part.parse::<u16>().ok())
        .unwrap_or(default_port);
    (host, port)
}

pub async fn health_check(State(state): State<AppState>) -> Json<ApiResponse<serde_json::Value>> {
    let (mysql_host, mysql_port) =
        parse_mysql_host_port(&state.config.mysql_url, "127.0.0.1", 3306);
    let (redis_host, redis_port) = parse_host_port(&state.config.redis_url, "127.0.0.1", 6379);
    let (qdrant_host, qdrant_port) = parse_host_port(&state.config.qdrant_url, "127.0.0.1", 6333);
    let (minio_host, minio_port) =
        parse_host_port(&state.config.minio_endpoint, "127.0.0.1", 9000);

    let mysql_reachable = if state.config.storage_backend == "mysql" {
        test_tcp_endpoint(&mysql_host, mysql_port).await
    } else {
        false
    };
    let redis_reachable = test_tcp_endpoint(&redis_host, redis_port).await;
    let qdrant_reachable = test_tcp_endpoint(&qdrant_host, qdrant_port).await;
    let minio_reachable = test_tcp_endpoint(&minio_host, minio_port).await;

    Json(ApiResponse::ok(
        "ok",
        serde_json::json!({
            "service": state.config.app_name,
            "status": "healthy",
            "storage_backend": state.config.storage_backend,
            "route_profile": if state.config.storage_backend == "mysql" { "mysql" } else { "file" },
            "dependencies": {
                "mysql": {
                    "configured": state.config.mysql_url,
                    "host": mysql_host,
                    "port": mysql_port,
                    "required": state.config.storage_backend == "mysql",
                    "reachable": mysql_reachable
                },
                "redis": {
                    "configured": state.config.redis_url,
                    "host": redis_host,
                    "port": redis_port,
                    "required": false,
                    "reachable": redis_reachable
                },
                "qdrant": {
                    "configured": state.config.qdrant_url,
                    "host": qdrant_host,
                    "port": qdrant_port,
                    "required": false,
                    "reachable": qdrant_reachable
                },
                "minio": {
                    "configured": state.config.minio_endpoint,
                    "host": minio_host,
                    "port": minio_port,
                    "bucket": state.config.minio_bucket,
                    "required": false,
                    "reachable": minio_reachable,
                    "mode": "object_storage_dir_mirror"
                }
            }
        }),
    ))
}
