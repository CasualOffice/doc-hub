//! Spike #4 — Two-origin Axum + `/raw/{token}`.
//!
//! Proves:
//! 1. One Axum binary can serve two Host-differentiated origins (app vs
//!    user-content) with route dispatch enforced at middleware level
//!    (421 Misdirected Request on cross-origin).
//! 2. The `/raw/{token}` handler verifies HMAC tokens (issued by spike #1's
//!    Storage::signed_get for fs/memory backends) and streams bytes with the
//!    right security headers.
//! 3. Production boot refuses to start when app_origin == usercontent_origin.

use std::sync::Arc;

use axum::{
    Router,
    body::Body,
    extract::{Path, Request, State},
    http::{HeaderName, HeaderValue, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use futures::TryStreamExt;
use spike_01_storage::Storage;

#[derive(Clone)]
pub struct Config {
    pub app_origin_host: Arc<str>,        // e.g. "drive.example.org" or "127.0.0.1:8080"
    pub usercontent_origin_host: Arc<str>,// e.g. "usercontent-drive.example.org"
    pub is_prod: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Storage,
    pub cfg: Config,
}

#[derive(Debug, thiserror::Error)]
pub enum BootError {
    #[error("app_origin and usercontent_origin must differ in production")]
    OriginsMatch,
}

/// Boot-time check from ARCHITECTURE.md §"Two-origin security model".
pub fn validate_config(cfg: &Config) -> Result<(), BootError> {
    if cfg.is_prod && cfg.app_origin_host == cfg.usercontent_origin_host {
        return Err(BootError::OriginsMatch);
    }
    Ok(())
}

/// Origin a route belongs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin { App, UserContent }

/// Middleware that 421s if the Host header doesn't match the origin this
/// router was mounted as. Defence-in-depth against misconfigured proxies.
pub async fn host_dispatch(
    State(state): State<AppState>,
    expected: Origin,
    req: Request,
    next: Next,
) -> Response {
    let host = req.headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let matches = match expected {
        Origin::App         => host == state.cfg.app_origin_host.as_ref(),
        Origin::UserContent => host == state.cfg.usercontent_origin_host.as_ref(),
    };
    if !matches {
        return (StatusCode::MISDIRECTED_REQUEST, format!("Wrong origin for this route (got Host={host:?})")).into_response();
    }
    next.run(req).await
}

// ─── App-origin handlers ─────────────────────────────────────────────────

async fn app_root() -> &'static str { "Drive SPA placeholder" }
async fn api_files() -> &'static str { r#"{"files":[]}"# }

// ─── User-content handlers ──────────────────────────────────────────────

async fn raw(
    State(state): State<AppState>,
    Path(token): Path<String>,
    req: Request,
) -> Result<Response, RawError> {
    let (key, method) = state.storage.verify_token(&token).map_err(map_token_err)?;
    let req_method = req.method().as_str();
    if method != req_method {
        return Err(RawError::MethodNotAllowed);
    }
    if method != "GET" {
        // Spike scope: only GET; PUT path is symmetrical.
        return Err(RawError::MethodNotAllowed);
    }

    let (meta, stream) = state.storage.get(&key, None).await
        .map_err(|_| RawError::NotFound)?;

    let content_type = meta.content_type
        .as_deref()
        .unwrap_or("application/octet-stream")
        .to_string();
    let filename = key.rsplit('/').next().unwrap_or("file").to_string();

    let body = Body::from_stream(
        stream.map_ok(|b| b).map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))
    );

    let mut r = Response::new(body);
    let h = r.headers_mut();
    h.insert(header::CONTENT_TYPE, HeaderValue::from_str(&content_type).unwrap());
    h.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{}", urlencode(&filename))).unwrap(),
    );
    h.insert(HeaderName::from_static("x-content-type-options"), HeaderValue::from_static("nosniff"));
    h.insert(HeaderName::from_static("content-security-policy"),
             HeaderValue::from_static("sandbox; default-src 'none'"));
    h.insert(HeaderName::from_static("cross-origin-resource-policy"),
             HeaderValue::from_static("same-site"));
    Ok(r)
}

#[derive(Debug, thiserror::Error)]
pub enum RawError {
    #[error("invalid token")]    InvalidToken,
    #[error("expired token")]    ExpiredToken,
    #[error("method not allowed")] MethodNotAllowed,
    #[error("not found")]        NotFound,
}

fn map_token_err(e: spike_01_storage::StorageError) -> RawError {
    match e {
        spike_01_storage::StorageError::InvalidToken => RawError::InvalidToken,
        spike_01_storage::StorageError::ExpiredToken => RawError::ExpiredToken,
        _ => RawError::InvalidToken,
    }
}

impl IntoResponse for RawError {
    fn into_response(self) -> Response {
        match self {
            RawError::InvalidToken     => (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
            RawError::ExpiredToken     => (StatusCode::UNAUTHORIZED, "expired token").into_response(),
            RawError::MethodNotAllowed => StatusCode::METHOD_NOT_ALLOWED.into_response(),
            RawError::NotFound         => StatusCode::NOT_FOUND.into_response(),
        }
    }
}

fn urlencode(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~') {
                vec![c.to_string()]
            } else {
                let mut buf = [0u8; 4];
                let bytes = c.encode_utf8(&mut buf);
                bytes.bytes().map(|b| format!("%{b:02X}")).collect()
            }
        })
        .collect()
}

// ─── Router assembly ───────────────────────────────────────────────────────

fn app_origin_router(state: AppState) -> Router {
    let st = state.clone();
    Router::new()
        .route("/", get(app_root))
        .route("/api/files", get(api_files))
        // App-origin baseline CSP: strict.
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static(
                "default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
            ),
        ))
        .layer(tower_http::set_header::SetResponseHeaderLayer::overriding(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(middleware::from_fn_with_state(
            st.clone(),
            |s: State<AppState>, req, next| host_dispatch(s, Origin::App, req, next),
        ))
        .with_state(st)
}

fn usercontent_router(state: AppState) -> Router {
    Router::new()
        .route("/raw/{token}", get(raw))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            |s: State<AppState>, req, next| host_dispatch(s, Origin::UserContent, req, next),
        ))
        .with_state(state)
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(app_origin_router(state.clone()))
        .merge(usercontent_router(state))
}
