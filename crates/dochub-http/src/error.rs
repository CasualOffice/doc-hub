//! Consistent JSON error envelope for the API.
//!
//! Every JSON API endpoint returns errors as [`ApiError`], which renders a
//! single, stable shape:
//!
//! ```json
//! { "error": { "code": "not_found", "message": "…", "retry_after_seconds": 5 } }
//! ```
//!
//! `code` is a stable machine-readable slug (safe to branch on); `message` is a
//! human string (do not parse). `retry_after_seconds` appears only on `429`.
//! A programmatic client — the SPA's fetch layer, an MCP-adjacent agent, a
//! curl script — always gets a parseable body instead of an empty one.
//!
//! Adoption is gradual: [`ApiError`] is `From<StatusCode>`, so a handler can
//! switch its `Result<_, StatusCode>` to `Result<_, ApiError>` and keep using
//! `?` on inner helpers that still yield a bare status. `From` derives the
//! `code`/`message` from the status' canonical reason phrase, so no call site
//! is forced to hand-write copy for the common cases.

use axum::{
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// A JSON API error: an HTTP status plus a stable envelope body.
#[derive(Debug, Clone)]
pub(crate) struct ApiError {
    status: StatusCode,
    /// Stable machine-readable slug, e.g. `"not_found"`, `"rate_limited"`.
    code: String,
    /// Human-readable detail. Never parsed by clients.
    message: String,
    /// Seconds to wait before retrying — set on `429` only; drives the
    /// `Retry-After` header and a body field.
    retry_after_seconds: Option<u64>,
}

impl ApiError {
    fn new(status: StatusCode, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            retry_after_seconds: None,
        }
    }

    pub(crate) fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    pub(crate) fn unprocessable(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNPROCESSABLE_ENTITY, "unprocessable", message)
    }

    /// A `500` with a generic client-facing message. Log the real cause at the
    /// call site; never leak it into the body.
    pub(crate) fn internal() -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal server error",
        )
    }

    /// A `429` carrying `Retry-After`. Shared by every AI surface and any other
    /// throttled endpoint.
    pub(crate) fn rate_limited(retry_after_seconds: u64) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: "rate_limited".into(),
            message: "rate limited".into(),
            retry_after_seconds: Some(retry_after_seconds),
        }
    }
}

impl From<StatusCode> for ApiError {
    /// Derive `code`/`message` from the status' canonical reason phrase, so a
    /// handler converting a bare `StatusCode` gets a sensible envelope for free.
    fn from(status: StatusCode) -> Self {
        let reason = status.canonical_reason().unwrap_or("error");
        // "Not Found" -> "not_found"; a stable slug clients can branch on.
        let code = reason.to_ascii_lowercase().replace([' ', '-'], "_");
        Self::new(status, code, reason)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let mut body = json!({
            "error": {
                "code": self.code,
                "message": self.message,
            }
        });
        if let Some(secs) = self.retry_after_seconds {
            body["error"]["retry_after_seconds"] = json!(secs);
        }
        let mut resp = (self.status, Json(body)).into_response();
        if let Some(secs) = self.retry_after_seconds {
            if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
                resp.headers_mut().insert(RETRY_AFTER, v);
            }
        }
        resp
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    async fn body_json(err: ApiError) -> (StatusCode, serde_json::Value, Option<String>) {
        let resp = err.into_response();
        let status = resp.status();
        let retry = resp
            .headers()
            .get(RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        (status, v, retry)
    }

    #[tokio::test]
    async fn envelope_has_stable_code_and_message() {
        let (status, v, retry) = body_json(ApiError::not_found("no such file")).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(v["error"]["code"], "not_found");
        assert_eq!(v["error"]["message"], "no such file");
        // Non-429 carries no retry hint, in header or body.
        assert!(retry.is_none());
        assert!(v["error"]["retry_after_seconds"].is_null());
    }

    #[tokio::test]
    async fn from_status_derives_slug_from_reason() {
        let (status, v, _) = body_json(StatusCode::UNPROCESSABLE_ENTITY.into()).await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(v["error"]["code"], "unprocessable_entity");
        assert_eq!(v["error"]["message"], "Unprocessable Entity");
    }

    #[tokio::test]
    async fn rate_limited_sets_header_and_body() {
        let (status, v, retry) = body_json(ApiError::rate_limited(7)).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(v["error"]["code"], "rate_limited");
        assert_eq!(v["error"]["retry_after_seconds"], 7);
        assert_eq!(retry.as_deref(), Some("7"));
    }

    #[test]
    fn internal_never_leaks_detail() {
        // The generic 500 body is a fixed string; callers log the real cause.
        let e = ApiError::internal();
        assert_eq!(e.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(e.message, "internal server error");
    }
}
