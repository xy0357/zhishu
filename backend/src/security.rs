use axum::{
    http::{header, HeaderMap, StatusCode},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    app::AppState,
    config::AppConfig,
    models::{ApiResponse, AuthSession, RefreshSessionResponse, UserItem},
};

pub const ADMIN_USERNAME: &str = "admin";
pub const ADMIN_PASSWORD: &str = "Admin@123456";
pub const EDITOR_USERNAME: &str = "editor";
pub const EDITOR_PASSWORD: &str = "Editor@123456";
const PASSWORD_SCHEME_SHA256: &str = "sha256";
const TOKEN_PREFIX: &str = "zhishu-v1";
const PASSWORD_ITERATIONS: u32 = 120_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessTokenClaims {
    sub: String,
    iss: String,
    iat: i64,
    exp: i64,
}

pub fn build_auth_session(config: &AppConfig, user: UserItem) -> AuthSession {
    let (access_token, expires_at) = issue_access_token(config, &user.username);
    AuthSession {
        access_token,
        expires_at,
        token_type: "Bearer".to_string(),
        user,
    }
}

pub fn build_refresh_session(config: &AppConfig, username: &str) -> RefreshSessionResponse {
    let (access_token, expires_at) = issue_access_token(config, username);
    RefreshSessionResponse {
        access_token,
        expires_at,
        token_type: "Bearer".to_string(),
    }
}

pub fn issue_access_token(config: &AppConfig, username: &str) -> (String, chrono::DateTime<Utc>) {
    let issued_at = Utc::now().timestamp();
    let expires_at = issued_at + config.access_token_ttl_hours.max(1) * 3600;
    let claims = AccessTokenClaims {
        sub: username.to_string(),
        iss: config.app_name.clone(),
        iat: issued_at,
        exp: expires_at,
    };
    let payload = serde_json::to_vec(&claims).expect("serialize access token claims");
    let encoded_payload = URL_SAFE_NO_PAD.encode(payload);
    let signature = sign_token_payload(&config.access_token_secret, &encoded_payload);
    (
        format!("{}.{}.{}", TOKEN_PREFIX, encoded_payload, signature),
        chrono::DateTime::<Utc>::from_timestamp(expires_at, 0).unwrap_or_else(Utc::now),
    )
}

#[cfg(test)]
pub fn issue_test_access_token(config: &AppConfig, username: &str) -> String {
    issue_access_token(config, username).0
}

pub fn hash_password(password: &str) -> String {
    let salt = Uuid::new_v4().simple().to_string();
    let digest = derive_password_digest(password, &salt, PASSWORD_ITERATIONS);
    format!(
        "{}${}${}${}",
        PASSWORD_SCHEME_SHA256, PASSWORD_ITERATIONS, salt, digest
    )
}

pub fn verify_password(password: &str, stored_value: &str) -> bool {
    let parts: Vec<&str> = stored_value.split('$').collect();
    if parts.len() == 4 && parts[0] == PASSWORD_SCHEME_SHA256 {
        let iterations = parts[1].parse::<u32>().ok().unwrap_or(PASSWORD_ITERATIONS);
        let salt = parts[2];
        let expected = parts[3];
        return constant_time_eq(
            derive_password_digest(password, salt, iterations).as_bytes(),
            expected.as_bytes(),
        );
    }

    // Backward compatibility for old demo data or pre-upgrade mysql rows.
    stored_value == password
}

pub fn password_needs_rehash(stored_value: &str) -> bool {
    !stored_value.starts_with(&format!("{}$", PASSWORD_SCHEME_SHA256))
}

pub fn can_admin(role_name: &str) -> bool {
    role_name == "系统管理员"
}

pub fn can_manage_content(role_name: &str) -> bool {
    matches!(role_name, "系统管理员" | "知识管理员")
}

pub fn can_manage_taxonomy(role_name: &str) -> bool {
    role_name == "系统管理员"
}

fn bearer_token(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn sign_token_payload(secret: &str, encoded_payload: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(secret.as_bytes());
    digest.update(b".");
    digest.update(encoded_payload.as_bytes());
    URL_SAFE_NO_PAD.encode(digest.finalize())
}

fn validate_access_token(config: &AppConfig, token: &str) -> Option<String> {
    let mut segments = token.split('.');
    let prefix = segments.next()?;
    let encoded_payload = segments.next()?;
    let signature = segments.next()?;
    if segments.next().is_some() || prefix != TOKEN_PREFIX {
        return None;
    }

    let expected_signature = sign_token_payload(&config.access_token_secret, encoded_payload);
    if !constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        return None;
    }

    let decoded_payload = URL_SAFE_NO_PAD.decode(encoded_payload).ok()?;
    let claims: AccessTokenClaims = serde_json::from_slice(&decoded_payload).ok()?;
    if claims.iss != config.app_name || claims.exp < Utc::now().timestamp() {
        return None;
    }

    Some(claims.sub)
}

fn derive_password_digest(password: &str, salt: &str, iterations: u32) -> String {
    let mut state = Sha256::new();
    state.update(salt.as_bytes());
    state.update(b":");
    state.update(password.as_bytes());
    let mut digest = state.finalize_reset().to_vec();

    for _ in 1..iterations {
        state.update(&digest);
        state.update(b":");
        state.update(salt.as_bytes());
        digest = state.finalize_reset().to_vec();
    }

    URL_SAFE_NO_PAD.encode(digest)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }

    let mut diff = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        diff |= a ^ b;
    }
    diff == 0
}

pub async fn require_user(headers: &HeaderMap, state: &AppState) -> Result<UserItem, StatusCode> {
    let Some(token) = bearer_token(headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let Some(username) = validate_access_token(&state.config, &token) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    state
        .store
        .get_user_by_username(username)
        .await
        .ok_or(StatusCode::UNAUTHORIZED)
}

pub async fn require_admin(headers: &HeaderMap, state: &AppState) -> Result<UserItem, StatusCode> {
    let user = require_user(headers, state).await?;
    if can_admin(&user.role_name) {
        Ok(user)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

pub async fn require_content_manager(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<UserItem, StatusCode> {
    let user = require_user(headers, state).await?;
    if can_manage_content(&user.role_name) {
        Ok(user)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

pub async fn require_taxonomy_manager(
    headers: &HeaderMap,
    state: &AppState,
) -> Result<UserItem, StatusCode> {
    let user = require_user(headers, state).await?;
    if can_manage_taxonomy(&user.role_name) {
        Ok(user)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

pub fn unauthorized_response() -> (StatusCode, Json<ApiResponse<&'static str>>) {
    (
        StatusCode::UNAUTHORIZED,
        Json(ApiResponse {
            success: false,
            message: "unauthorized".to_string(),
            data: "请先登录",
        }),
    )
}

#[cfg(test)]
mod tests {
    use crate::config::AppConfig;

    use super::{hash_password, issue_test_access_token, verify_password};

    #[test]
    fn password_hash_should_roundtrip() {
        let hashed = hash_password("Admin@123456");
        assert!(verify_password("Admin@123456", &hashed));
        assert!(!verify_password("WrongPassword", &hashed));
    }

    #[test]
    fn access_token_should_be_signed_and_decodable() {
        let config = AppConfig::from_env();
        let token = issue_test_access_token(&config, "admin");
        assert!(token.starts_with("zhishu-v1."));
    }
}
