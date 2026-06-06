//! Spike #2 — Minimal WOPI host.
//!
//! Implements the 7 endpoints required for real-time edit-and-save against
//! purely in-memory state. The goal is to confirm the wire shape from
//! `docs/research/01-wopi.md` §1 + §4 — especially the asymmetric
//! 409 + `X-WOPI-Lock` response header contract — works in Axum 0.8.
//!
//! Out of scope: proof-keys (no MS365 federation in v0), persistence,
//! multi-file co-edit (single lock per file is the spec).

use std::{collections::HashMap, sync::Arc};

use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::get,
};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, encode, decode};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

// ─── Types ──────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub files: Arc<Mutex<HashMap<String, FileEntry>>>,
    pub jwt_secret: Arc<[u8; 32]>,
}

#[derive(Clone, Debug)]
pub struct FileEntry {
    pub name: String,
    pub bytes: Bytes,
    pub version: u32,
    pub lock: Option<LockEntry>,
}

#[derive(Clone, Debug)]
pub struct LockEntry {
    pub id: String,
    pub acquired_at: time::OffsetDateTime,
}

impl LockEntry {
    /// WOPI spec: locks auto-expire after 30 minutes unless refreshed.
    pub fn expired(&self) -> bool {
        time::OffsetDateTime::now_utc() - self.acquired_at > time::Duration::minutes(30)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WopiClaims {
    pub user_id: String,
    pub file_id: String,
    pub perms: WopiPerms,
    pub exp: i64,
    pub jti: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum WopiPerms {
    #[serde(rename = "read")] Read,
    #[serde(rename = "write")] Write,
}

#[derive(Deserialize)]
pub struct TokenQuery {
    pub access_token: String,
}

#[derive(Serialize, Deserialize)]
pub struct CheckFileInfo {
    #[serde(rename = "BaseFileName")] pub base_file_name: String,
    #[serde(rename = "OwnerId")]      pub owner_id: String,
    #[serde(rename = "Size")]         pub size: u64,
    #[serde(rename = "UserId")]       pub user_id: String,
    #[serde(rename = "Version")]      pub version: String,
    #[serde(rename = "UserCanWrite")] pub user_can_write: bool,
    #[serde(rename = "SupportsUpdate")] pub supports_update: bool,
    #[serde(rename = "SupportsLocks")] pub supports_locks: bool,
    #[serde(rename = "SupportsExtendedLockLength")] pub supports_extended_lock_length: bool,
    #[serde(rename = "IsAnonymousUser")] pub is_anonymous_user: bool,
}

// ─── Token mint / verify ────────────────────────────────────────────────────

pub fn mint_token(state: &AppState, claims: &WopiClaims) -> String {
    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        claims,
        &EncodingKey::from_secret(state.jwt_secret.as_ref()),
    )
    .expect("HS256 encode")
}

fn verify_token(state: &AppState, token: &str, expected_file: &str) -> Result<WopiClaims, WopiError> {
    let mut v = Validation::new(jsonwebtoken::Algorithm::HS256);
    v.validate_exp = true;
    v.leeway = 0;
    let data = decode::<WopiClaims>(
        token,
        &DecodingKey::from_secret(state.jwt_secret.as_ref()),
        &v,
    )
    .map_err(|_| WopiError::Unauthorized)?;
    if data.claims.file_id != expected_file {
        return Err(WopiError::Unauthorized);
    }
    Ok(data.claims)
}

// ─── Errors → status mapping ────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum WopiError {
    #[error("bad request")]   BadRequest,
    #[error("unauthorized")]  Unauthorized,
    #[error("not found")]     NotFound,
    /// 409 + `X-WOPI-Lock: <current>` — mandatory + asymmetric per spec §4.
    #[error("lock conflict")] LockConflict(String),
    #[error("size mismatch")] PreconditionFailed,
    #[error("payload too large")] PayloadTooLarge,
}

impl IntoResponse for WopiError {
    fn into_response(self) -> Response {
        match self {
            WopiError::BadRequest         => StatusCode::BAD_REQUEST.into_response(),
            WopiError::Unauthorized       => StatusCode::UNAUTHORIZED.into_response(),
            WopiError::NotFound           => StatusCode::NOT_FOUND.into_response(),
            WopiError::PreconditionFailed => StatusCode::PRECONDITION_FAILED.into_response(),
            WopiError::PayloadTooLarge    => StatusCode::PAYLOAD_TOO_LARGE.into_response(),
            WopiError::LockConflict(current) => {
                let mut r = Response::new(Body::empty());
                *r.status_mut() = StatusCode::CONFLICT;
                r.headers_mut().insert(
                    HeaderName::from_static("x-wopi-lock"),
                    HeaderValue::from_str(&current).unwrap_or(HeaderValue::from_static("")),
                );
                r
            }
        }
    }
}

// ─── Header helpers ─────────────────────────────────────────────────────────

const H_LOCK:      HeaderName = HeaderName::from_static("x-wopi-lock");
const H_OLDLOCK:   HeaderName = HeaderName::from_static("x-wopi-oldlock");
const H_OVERRIDE:  HeaderName = HeaderName::from_static("x-wopi-override");
const H_ITEMVER:   HeaderName = HeaderName::from_static("x-wopi-itemversion");

fn header_str<'a>(h: &'a HeaderMap, name: &HeaderName) -> Option<&'a str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

// ─── Handlers ───────────────────────────────────────────────────────────────

pub async fn check_file_info(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
) -> Result<Response, WopiError> {
    let claims = verify_token(&state, &access_token, &file_id)?;
    let files = state.files.lock().await;
    let f = files.get(&file_id).ok_or(WopiError::NotFound)?;
    let info = CheckFileInfo {
        base_file_name: f.name.clone(),
        owner_id: "admin".into(),
        size: f.bytes.len() as u64,
        user_id: claims.user_id.clone(),
        version: f.version.to_string(),
        user_can_write: matches!(claims.perms, WopiPerms::Write),
        supports_update: true,
        supports_locks: true,
        supports_extended_lock_length: true,
        is_anonymous_user: false,
    };
    Ok(([(axum::http::header::CONTENT_TYPE, "application/json")],
        serde_json::to_vec(&info).unwrap()).into_response())
}

pub async fn get_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
) -> Result<Response, WopiError> {
    verify_token(&state, &access_token, &file_id)?;
    let files = state.files.lock().await;
    let f = files.get(&file_id).ok_or(WopiError::NotFound)?;
    let mut r = Response::new(Body::from(f.bytes.clone()));
    r.headers_mut().insert(
        H_ITEMVER,
        HeaderValue::from_str(&f.version.to_string()).unwrap(),
    );
    Ok(r)
}

/// `POST /wopi/files/{id}/contents` — PutFile.
pub async fn put_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, WopiError> {
    let claims = verify_token(&state, &access_token, &file_id)?;
    if !matches!(claims.perms, WopiPerms::Write) {
        return Err(WopiError::Unauthorized);
    }
    let mut files = state.files.lock().await;
    let f = files.get_mut(&file_id).ok_or(WopiError::NotFound)?;

    // 0-byte createnew is the only PutFile-without-lock allowed (spec §4).
    let lock_header = header_str(&headers, &H_LOCK);
    let current_lock = f.lock.as_ref().filter(|l| !l.expired()).map(|l| l.id.clone());
    match (current_lock.as_deref(), lock_header) {
        (Some(cur), Some(h)) if cur == h => {} // happy path
        (Some(cur), _) => return Err(WopiError::LockConflict(cur.to_string())),
        (None, _) if f.bytes.is_empty() && body.is_empty() => {} // createnew
        (None, _) => return Err(WopiError::LockConflict(String::new())),
    }

    f.bytes = body;
    f.version += 1;

    let mut r = Response::new(Body::empty());
    r.headers_mut().insert(
        H_ITEMVER,
        HeaderValue::from_str(&f.version.to_string()).unwrap(),
    );
    Ok(r)
}

/// `POST /wopi/files/{id}` — dispatched on `X-WOPI-Override`.
pub async fn lock_dispatch(
    state: State<AppState>,
    path: Path<String>,
    query: Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, WopiError> {
    match header_str(&headers, &H_OVERRIDE) {
        Some("LOCK") => {
            if header_str(&headers, &H_OLDLOCK).is_some() {
                unlock_and_relock(state, path, query, headers).await
            } else {
                lock(state, path, query, headers).await
            }
        }
        Some("UNLOCK")        => unlock(state, path, query, headers).await,
        Some("REFRESH_LOCK")  => refresh_lock(state, path, query, headers).await,
        _ => Err(WopiError::BadRequest),
    }
}

async fn lock(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, WopiError> {
    verify_token(&state, &access_token, &file_id)?;
    let new_lock = header_str(&headers, &H_LOCK)
        .ok_or(WopiError::BadRequest)?
        .to_string();
    let mut files = state.files.lock().await;
    let f = files.get_mut(&file_id).ok_or(WopiError::NotFound)?;

    let current = f.lock.as_ref().filter(|l| !l.expired()).map(|l| l.id.clone());
    match current {
        // Lock-with-current-id ≡ RefreshLock (spec §4).
        Some(cur) if cur == new_lock => {
            f.lock = Some(LockEntry {
                id: new_lock,
                acquired_at: time::OffsetDateTime::now_utc(),
            });
        }
        Some(cur) => return Err(WopiError::LockConflict(cur)),
        None => {
            f.lock = Some(LockEntry {
                id: new_lock,
                acquired_at: time::OffsetDateTime::now_utc(),
            });
        }
    }
    Ok(StatusCode::OK.into_response())
}

async fn unlock(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, WopiError> {
    verify_token(&state, &access_token, &file_id)?;
    let req_lock = header_str(&headers, &H_LOCK)
        .ok_or(WopiError::BadRequest)?
        .to_string();
    let mut files = state.files.lock().await;
    let f = files.get_mut(&file_id).ok_or(WopiError::NotFound)?;

    let current = f.lock.as_ref().filter(|l| !l.expired()).map(|l| l.id.clone());
    match current {
        Some(cur) if cur == req_lock => {
            f.lock = None;
            Ok(StatusCode::OK.into_response())
        }
        Some(cur) => Err(WopiError::LockConflict(cur)),
        None      => Err(WopiError::LockConflict(String::new())),
    }
}

async fn refresh_lock(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, WopiError> {
    verify_token(&state, &access_token, &file_id)?;
    let req_lock = header_str(&headers, &H_LOCK)
        .ok_or(WopiError::BadRequest)?
        .to_string();
    let mut files = state.files.lock().await;
    let f = files.get_mut(&file_id).ok_or(WopiError::NotFound)?;

    let current = f.lock.as_ref().filter(|l| !l.expired()).map(|l| l.id.clone());
    match current {
        Some(cur) if cur == req_lock => {
            f.lock = Some(LockEntry {
                id: req_lock,
                acquired_at: time::OffsetDateTime::now_utc(),
            });
            Ok(StatusCode::OK.into_response())
        }
        Some(cur) => Err(WopiError::LockConflict(cur)),
        None      => Err(WopiError::LockConflict(String::new())),
    }
}

async fn unlock_and_relock(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
    Query(TokenQuery { access_token }): Query<TokenQuery>,
    headers: HeaderMap,
) -> Result<Response, WopiError> {
    verify_token(&state, &access_token, &file_id)?;
    let new_lock = header_str(&headers, &H_LOCK).ok_or(WopiError::BadRequest)?.to_string();
    let old_lock = header_str(&headers, &H_OLDLOCK).ok_or(WopiError::BadRequest)?.to_string();
    let mut files = state.files.lock().await;
    let f = files.get_mut(&file_id).ok_or(WopiError::NotFound)?;

    let current = f.lock.as_ref().filter(|l| !l.expired()).map(|l| l.id.clone());
    match current {
        Some(cur) if cur == old_lock => {
            f.lock = Some(LockEntry {
                id: new_lock,
                acquired_at: time::OffsetDateTime::now_utc(),
            });
            Ok(StatusCode::OK.into_response())
        }
        Some(cur) => Err(WopiError::LockConflict(cur)),
        None      => Err(WopiError::LockConflict(String::new())),
    }
}

// ─── Router assembly ────────────────────────────────────────────────────────

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/wopi/files/{file_id}",          get(check_file_info).post(lock_dispatch))
        .route("/wopi/files/{file_id}/contents", get(get_file).post(put_file))
        .with_state(state)
}
