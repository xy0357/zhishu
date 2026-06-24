mod app;
mod config;
mod llm;
mod models;
mod object_storage;
mod redis_cache;
mod routes;
mod security;
mod store;

use std::net::{IpAddr, SocketAddr};

use app::build_router;
use config::AppConfig;
use store::{memory::MemoryStore, mysql::MySqlStore, DynStore};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = AppConfig::from_env();
    let state: DynStore = match config.storage_backend.as_str() {
        "mysql" => std::sync::Arc::new(
            MySqlStore::new(&config.mysql_url)
                .await
                .expect("initialize mysql store"),
        ),
        _ => std::sync::Arc::new(MemoryStore::from_path(&config.storage_file)),
    };
    let app = build_router(config.clone(), state);
    let host_ip = config
        .host
        .parse::<IpAddr>()
        .unwrap_or(IpAddr::from([127, 0, 0, 1]));
    let addr = SocketAddr::from((host_ip, config.port));

    tracing::info!("zhishu backend listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind server");
    axum::serve(listener, app).await.expect("run server");
}
