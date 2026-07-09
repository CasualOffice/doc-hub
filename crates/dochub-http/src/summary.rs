//! `GET /api/files/{id}/summary` — Phase 3 P3.5 on-demand document summary.
//!
//! Read-only + audited (build spec §3). The AI layer never mutates a document,
//! its versions, or the hash chain — this handler only *reads* the head
//! version's bytes, calls the configured [`dochub_ai::AiProvider`], caches the
//! result by the head `content_hash`, and appends one `ai.summary` audit row.
//!
//! Flow:
//! 1. Session-auth (the [`AuthSession`] extractor 401s without a session).
//! 2. Owner-or-member gate on the file (404 unknown, 403 unauthorized).
//! 3. If AI is `off` → 409 disabled (no provider is even built).
//! 4. Resolve the head version + its `content_hash`.
//! 5. Cache hit for that hash → return it verbatim (no provider call, no audit).
//! 6. Miss → extract text (reusing the same extraction as `dochub-index` for
//!    `md/txt/csv/json/yaml`; `docx/xlsx/xlsm/pptx/pdf` summarize the title /
//!    metadata for now — `core` content extraction is the documented follow-up),
//!    call the provider, cache it, write the audit row, and return it.
//!
//! The provider is built lazily and cached in a process-global registry keyed by
//! the owning `Arc<Config>` pointer — the same idiom [`crate::content_search`]
//! uses for the index, so no new field is forced onto `HttpState` (and thus onto
//! `dochub-bin` + every test fixture).

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dochub_ai::{provider_from_config, AiProvider, AiProviderKind, SummarizeOpts};
use dochub_auth::AuthSession;
use dochub_core::{Config, DocKind};
use dochub_db::{
    action, AiSummaryRepo, AuditRepo, FileRepo, FileVersionsRepo, NewAuditEvent,
    WorkspaceMemberRepo,
};
use serde::Serialize;

use crate::files::version_registry;
use crate::HttpState;

// ── Process-global provider registry (see module docs) ─────────────────────

type ProviderRegistry = Mutex<HashMap<usize, (Arc<Config>, Arc<dyn AiProvider>)>>;

fn provider_registry() -> &'static ProviderRegistry {
    static REG: OnceLock<ProviderRegistry> = OnceLock::new();
    REG.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Resolve (building on first use) the configured AI provider for this state, or
/// `None` when AI is `off`.
fn provider_for(state: &HttpState) -> Result<Option<Arc<dyn AiProvider>>, SummaryError> {
    if state.config.ai.provider == AiProviderKind::Off {
        return Ok(None);
    }
    let key = Arc::as_ptr(&state.config) as usize;
    let mut reg = provider_registry()
        .lock()
        .map_err(|_| SummaryError::Internal("provider registry poisoned".into()))?;
    if let Some((_, p)) = reg.get(&key) {
        return Ok(Some(p.clone()));
    }
    let Some(provider) = provider_from_config(&state.config.ai)
        .map_err(|e| SummaryError::Internal(e.to_string()))?
    else {
        return Ok(None);
    };
    reg.insert(key, (state.config.clone(), provider.clone()));
    Ok(Some(provider))
}

// ── Errors ─────────────────────────────────────────────────────────────────

enum SummaryError {
    /// File does not exist (or has no committed version to summarize).
    NotFound,
    /// Caller is neither the owner nor a member of the file's workspace.
    Forbidden,
    /// AI is configured `off`.
    Disabled,
    Internal(String),
}

#[derive(Serialize)]
struct ErrBody {
    error: &'static str,
}

impl IntoResponse for SummaryError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => {
                (StatusCode::NOT_FOUND, Json(ErrBody { error: "not found" })).into_response()
            }
            Self::Forbidden => {
                (StatusCode::FORBIDDEN, Json(ErrBody { error: "forbidden" })).into_response()
            }
            Self::Disabled => (
                StatusCode::CONFLICT,
                Json(ErrBody {
                    error: "ai disabled",
                }),
            )
                .into_response(),
            Self::Internal(m) => {
                tracing::error!(error = %m, "summary internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrBody {
                        error: "internal error",
                    }),
                )
                    .into_response()
            }
        }
    }
}

// ── Response ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct SummaryResp {
    summary: String,
    model: String,
    /// True when served from the cache (no provider call, no new audit row).
    cached: bool,
    input_tokens: u32,
    output_tokens: u32,
}

// ── Handler ──────────────────────────────────────────────────────────────────

/// `GET /api/files/{id}/summary`.
async fn file_summary(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(id): Path<String>,
) -> Result<Json<SummaryResp>, SummaryError> {
    // 1. File lookup.
    let file = FileRepo::new(&s.db)
        .find_by_id(&id)
        .await
        .map_err(|_| SummaryError::NotFound)?;

    // 2. Owner-or-member gate. A non-member who isn't the owner gets 403 (never
    //    a hint that the file exists beyond the generic forbidden).
    let is_owner = file.owner_id == session.user_id;
    let is_member = match file.workspace_id.as_deref() {
        Some(ws) => WorkspaceMemberRepo::new(&s.db)
            .role_of(ws, &session.user_id)
            .await
            .map_err(|e| SummaryError::Internal(e.to_string()))?
            .is_some(),
        None => false,
    };
    if !is_owner && !is_member {
        return Err(SummaryError::Forbidden);
    }

    // 3. AI off ⇒ disabled. Build the provider before touching content so a
    //    disabled instance never decrypts document bytes for nothing.
    let Some(provider) = provider_for(&s)? else {
        return Err(SummaryError::Disabled);
    };

    // 4. Head version + its content_hash (the cache key). No head ⇒ nothing to
    //    summarize.
    let head = FileVersionsRepo::new(&s.db)
        .head(&id)
        .await
        .map_err(|e| SummaryError::Internal(e.to_string()))?
        .ok_or(SummaryError::NotFound)?;
    let content_hash = head.content_hash.clone();

    // 5. Cache hit → return verbatim. No provider call, no audit row.
    let cache = AiSummaryRepo::new(&s.db);
    if let Some(cached) = cache
        .get_cached_summary(&id, &content_hash)
        .await
        .map_err(|e| SummaryError::Internal(e.to_string()))?
    {
        return Ok(Json(SummaryResp {
            summary: cached.summary,
            model: cached.model,
            cached: true,
            input_tokens: cached.input_tokens as u32,
            output_tokens: cached.output_tokens as u32,
        }));
    }

    // 6. Miss → extract the text to summarize. Text formats decrypt + read the
    //    head bytes (a pure `read_version`, never a backfill/commit); other
    //    formats summarize the title/metadata for now (core extraction is the
    //    documented follow-up, build spec §1/§3).
    let text = extract_text(&s, &file, head.seq).await?;

    // 7. Summarize, cache, audit, return.
    let summary = provider
        .summarize(&text, SummarizeOpts::default())
        .await
        .map_err(|e| SummaryError::Internal(e.to_string()))?;

    cache
        .put_summary(
            &id,
            &content_hash,
            &summary.text,
            &summary.model,
            i64::from(summary.input_tokens),
            i64::from(summary.output_tokens),
        )
        .await
        .map_err(|e| SummaryError::Internal(e.to_string()))?;

    // Await the audit write so the compliance record is durable before we answer
    // (an `ai.summary` row with the model + token counts). This is the only side
    // effect besides the cache — both are additive; the document, its versions,
    // and the hash chain are untouched.
    AuditRepo::new(&s.db)
        .insert(NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: action::AI_SUMMARY.into(),
            target_kind: Some("file".into()),
            target_id: Some(id.clone()),
            target_name: Some(file.name.clone()),
            ip_address: None,
            metadata: Some(
                serde_json::json!({
                    "model": summary.model,
                    "input_tokens": summary.input_tokens,
                    "output_tokens": summary.output_tokens,
                    "content_hash": content_hash,
                    "cached": false,
                })
                .to_string(),
            ),
        })
        .await
        .map_err(|e| SummaryError::Internal(e.to_string()))?;

    Ok(Json(SummaryResp {
        summary: summary.text,
        model: summary.model,
        cached: false,
        input_tokens: summary.input_tokens,
        output_tokens: summary.output_tokens,
    }))
}

/// Extract the text to summarize for `file`'s head version `seq`.
///
/// Text formats (`md/txt/csv/json/yaml`) decrypt + read the head bytes via a
/// pure [`dochub_db::Registry::read_version`] (no backfill, no commit). Binary
/// document formats (`docx/xlsx/xlsm/pptx/pdf`) summarize the title/metadata for
/// now — `core`-backed content extraction is the documented follow-up. An empty
/// text document also falls back to the title so the provider always has
/// something to work with.
async fn extract_text(
    s: &HttpState,
    file: &dochub_db::File,
    seq: i64,
) -> Result<String, SummaryError> {
    let kind = extension_of(&file.name)
        .as_deref()
        .and_then(DocKind::from_extension);
    let is_text = matches!(
        kind,
        Some(DocKind::Md | DocKind::Txt | DocKind::Csv | DocKind::Json | DocKind::Yaml)
    );

    if is_text {
        let bytes = version_registry(s)
            .read_version(&file.id, seq)
            .await
            .map_err(|e| SummaryError::Internal(e.to_string()))?;
        let content = String::from_utf8_lossy(&bytes).into_owned();
        if !content.trim().is_empty() {
            return Ok(content);
        }
    }
    // Binary format, or an empty text document: metadata-only.
    Ok(format!("Document titled \"{}\".", file.name))
}

/// Lowercase filename extension, or `None`. Mirrors the helper in
/// [`crate::content_search`].
fn extension_of(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let (stem, ext) = base.rsplit_once('.')?;
    if stem.is_empty() || ext.is_empty() {
        return None;
    }
    Some(ext.to_ascii_lowercase())
}

pub(crate) fn router(state: HttpState) -> Router {
    Router::new()
        .route("/api/files/{id}/summary", get(file_summary))
        .with_state(state)
}
