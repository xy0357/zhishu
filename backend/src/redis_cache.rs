use std::time::Duration;

use serde::{de::DeserializeOwned, Serialize};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

fn parse_redis_host_port(redis_url: &str) -> (String, u16) {
    let trimmed = redis_url.trim();
    let without_scheme = trimmed.strip_prefix("redis://").unwrap_or(trimmed);
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    let host_port = authority.rsplit('@').next().unwrap_or(authority);
    let mut parts = host_port.split(':');
    let host = parts
        .next()
        .filter(|part| !part.is_empty())
        .unwrap_or("127.0.0.1")
        .to_string();
    let port = parts
        .next()
        .and_then(|part| part.parse::<u16>().ok())
        .unwrap_or(6379);
    (host, port)
}

fn build_resp(parts: &[&str]) -> Vec<u8> {
    let mut request = format!("*{}\r\n", parts.len()).into_bytes();
    for part in parts {
        request.extend_from_slice(format!("${}\r\n", part.len()).as_bytes());
        request.extend_from_slice(part.as_bytes());
        request.extend_from_slice(b"\r\n");
    }
    request
}

async fn open_redis(redis_url: &str) -> Option<TcpStream> {
    let (host, port) = parse_redis_host_port(redis_url);
    timeout(Duration::from_millis(800), TcpStream::connect((host.as_str(), port)))
        .await
        .ok()
        .and_then(Result::ok)
}

pub async fn get_json<T: DeserializeOwned>(redis_url: &str, key: &str) -> Option<T> {
    let stream = open_redis(redis_url).await?;
    let mut reader = BufReader::new(stream);
    let request = build_resp(&["GET", key]);
    reader.get_mut().write_all(&request).await.ok()?;
    reader.get_mut().flush().await.ok()?;

    let mut first_line = String::new();
    reader.read_line(&mut first_line).await.ok()?;
    if first_line.starts_with("$-1") {
        return None;
    }
    if !first_line.starts_with('$') {
        return None;
    }

    let length = first_line
        .trim()
        .trim_start_matches('$')
        .parse::<usize>()
        .ok()?;
    let mut payload = vec![0_u8; length + 2];
    reader.read_exact(&mut payload).await.ok()?;
    let json_bytes = &payload[..length];
    serde_json::from_slice(json_bytes).ok()
}

pub async fn set_json<T: Serialize>(
    redis_url: &str,
    key: &str,
    ttl_seconds: u64,
    value: &T,
) -> bool {
    let json = match serde_json::to_string(value) {
        Ok(json) => json,
        Err(_) => return false,
    };

    let mut stream = match open_redis(redis_url).await {
        Some(stream) => stream,
        None => return false,
    };

    let ttl = ttl_seconds.to_string();
    let request = build_resp(&["SETEX", key, ttl.as_str(), json.as_str()]);
    if stream.write_all(&request).await.is_err() || stream.flush().await.is_err() {
        return false;
    }

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    if reader.read_line(&mut response).await.is_err() {
        return false;
    }

    response.starts_with("+OK")
}
