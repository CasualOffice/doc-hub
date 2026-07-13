//! AI provider selection for the RAG + agentic endpoints.
//!
//! The hosted LLM is resolved from the environment once and cached. Provider-
//! agnostic: set `DOCHUB_AI_PROVIDER` to `anthropic` (Claude), `openai`
//! (ChatGPT), or `local` (a self-hosted OpenAI-compatible server) — see
//! [`dochub_ai::RemoteAnswerer::from_env`].
//!
//! Two capabilities are derived from the one cached client:
//! - [`answerer`] — single-shot RAG. Falls back to the offline
//!   [`dochub_ai::ExtractiveAnswerer`] when no provider is configured, so the
//!   `ask` endpoint stays self-hostable and every test is deterministic (no test
//!   sets the vars).
//! - [`chat_model`] — the multi-turn chat the agentic loop drives. Returns
//!   `None` when no provider is configured: the agent needs a real model to
//!   reason, so its endpoint degrades explicitly rather than pretending.

use std::sync::{Arc, OnceLock};

use axum::{
    http::{header::RETRY_AFTER, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use dochub_ai::{Answerer, ChatModel, ExtractiveAnswerer, RemoteAnswerer};
use serde_json::json;

use crate::rate_limit::{RateLimitConfig, RateLimiter};

/// The hosted LLM client, resolved once from the environment. `None` on an
/// offline install (no `DOCHUB_AI_PROVIDER`). Shared by both capabilities below
/// so they always use the same configured model.
fn remote() -> Option<Arc<RemoteAnswerer>> {
    static REMOTE: OnceLock<Option<Arc<RemoteAnswerer>>> = OnceLock::new();
    REMOTE
        .get_or_init(|| match RemoteAnswerer::from_env() {
            Some(a) => {
                tracing::info!("AI: hosted LLM configured (DOCHUB_AI_PROVIDER)");
                Some(Arc::new(a))
            }
            None => {
                tracing::info!(
                    "AI: no hosted LLM (set DOCHUB_AI_PROVIDER for RAG generation + the agent); \
                     ask falls back to offline extractive"
                );
                None
            }
        })
        .clone()
}

/// The configured RAG answerer. Hosted LLM when configured, else the offline
/// extractive baseline. Shared by the `ask` endpoint and the MCP `ask` tool so
/// they answer identically.
pub(crate) fn answerer() -> Arc<dyn Answerer> {
    match remote() {
        Some(a) => {
            let a: Arc<dyn Answerer> = a;
            a
        }
        None => {
            let a: Arc<dyn Answerer> = Arc::new(ExtractiveAnswerer::default());
            a
        }
    }
}

/// The chat model driving the agentic loop, or `None` when no hosted/local LLM
/// is configured (the agent requires a real model — there is no offline
/// substitute for multi-step reasoning).
pub(crate) fn chat_model() -> Option<Arc<dyn ChatModel>> {
    remote().map(|a| {
        let m: Arc<dyn ChatModel> = a;
        m
    })
}

/// Per-user throttle shared across every AI surface (`ask`, the agent, and MCP
/// tool calls). Each fans out to retrieval + an LLM on every call — the agent
/// several times — so they draw on **one** budget per user: 20 burst, refilling
/// ~1 every 5s (~12/min sustained). Keyed by the caller's id, in-memory (single
/// instance; a Redis backend is the cluster follow-up, like the upload limiter).
pub(crate) fn ai_limiter() -> &'static RateLimiter {
    static LIMITER: OnceLock<RateLimiter> = OnceLock::new();
    LIMITER.get_or_init(|| {
        RateLimiter::new(RateLimitConfig {
            capacity: 20.0,
            refill_per_sec: 0.2,
        })
    })
}

/// Error type for the AI HTTP endpoints. `Status` carries an ordinary code
/// (via `?` from `StatusCode`), `RateLimited` renders a `429` + `Retry-After`.
#[derive(Debug)]
pub(crate) enum AiHttpError {
    Status(StatusCode),
    RateLimited(u64),
}

impl From<StatusCode> for AiHttpError {
    fn from(s: StatusCode) -> Self {
        Self::Status(s)
    }
}

impl IntoResponse for AiHttpError {
    fn into_response(self) -> Response {
        match self {
            Self::Status(s) => s.into_response(),
            Self::RateLimited(secs) => rate_limited_response(secs),
        }
    }
}

/// A `429 Too Many Requests` with a `Retry-After` header + JSON body — shared by
/// the AI HTTP endpoints and the MCP tool-call path.
pub(crate) fn rate_limited_response(retry_after_secs: u64) -> Response {
    let mut resp = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": "rate limited", "retry_after_seconds": retry_after_secs })),
    )
        .into_response();
    if let Ok(v) = HeaderValue::from_str(&retry_after_secs.to_string()) {
        resp.headers_mut().insert(RETRY_AFTER, v);
    }
    resp
}
