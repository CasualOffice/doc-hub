//! Version diff API (build spec §2 — P1.5). Compares two committed versions of
//! a file and returns a structured diff so the SPA's version-history surface can
//! render "what changed between seq A and seq B".
//!
//! Endpoint (mounted under `/api`, app origin, owner-gated like the sibling
//! version handlers in [`crate::versions`]):
//!
//! - `GET /api/files/{id}/diff?from={seqA}&to={seqB}`
//!
//! Both versions' plaintext bytes are read through the encrypted version engine
//! ([`dochub_db::Registry::read_version`]) — the same decrypt path the version
//! surface already uses. Nothing here mutates history; it is a pure read.
//!
//! ## Kinds
//! The document kind is decided from the file-name extension:
//!
//! - **Text** (`md, txt, csv, json, yaml, yml`) whose *both* versions are valid
//!   UTF-8 → a line diff via the `similar` crate, returned as merged hunks.
//! - **Binary / opaque** (`docx, xlsx, xlsm, pptx, pdf`, or any version that is
//!   not valid UTF-8) → sizes plus a content-equality flag; no byte-level diff.
//!
//! Real text extraction for office/PDF formats (so a `.docx` could diff as text)
//! lives in `core` and lands in Phase 3 — deliberately out of scope here, so
//! office/PDF are reported as opaque `binary` for now.

use axum::{
    extract::{Path, Query, State},
    Json, Router,
};
use dochub_db::{FileRepo, RegistryError};
use serde::{Deserialize, Serialize};
use similar::{ChangeTag, TextDiff};

use crate::files::{version_registry, FilesError};
use crate::HttpState;

/// Text formats we can diff line-by-line. Everything else is opaque `binary`.
/// Mirrors the documents-only ingest allowlist (`CLAUDE.md` → product scope).
const TEXT_EXTENSIONS: &[&str] = &["md", "txt", "csv", "json", "yaml", "yml"];

/// Cap on the total bytes of hunk content we serialize. A diff between two large
/// documents can be many times their size; past this ceiling we stop appending
/// and flag `truncated` so the surface can offer "download full versions". 256
/// KiB is generous for a review UI while keeping responses bounded.
const MAX_DIFF_BYTES: usize = 256 * 1024;

/// `?from=&to=` — the two 1-based version `seq`s to compare.
#[derive(Debug, Deserialize)]
struct DiffQuery {
    from: i64,
    to: i64,
}

/// One run of same-tagged lines. Consecutive per-line changes with the same tag
/// are merged into a single hunk so the surface renders contiguous blocks rather
/// than one node per line.
#[derive(Debug, Serialize)]
struct Hunk {
    /// `"equal"`, `"insert"` (present only in `to`), or `"delete"` (present only
    /// in `from`).
    tag: &'static str,
    /// The lines of this run, concatenated (each keeps its trailing newline).
    content: String,
}

/// `GET /api/files/{id}/diff` body. `kind` discriminates the two shapes.
#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
enum DiffResp {
    /// A line diff of two UTF-8 text versions.
    Text {
        hunks: Vec<Hunk>,
        /// `true` when the diff was cut off at [`MAX_DIFF_BYTES`].
        truncated: bool,
    },
    /// Two opaque versions — no byte diff, just sizes and whether their decrypted
    /// contents are identical.
    Binary {
        from_size: usize,
        to_size: usize,
        identical: bool,
    },
}

/// Map a registry failure onto the shared file-API error surface. An unknown
/// `seq` is a 404; everything else is an internal error — key material and
/// plaintext never appear in these strings by construction. Mirrors the sibling
/// helper in [`crate::versions`] (kept local so this module touches nothing
/// there).
fn map_registry_err(e: RegistryError) -> FilesError {
    match e {
        RegistryError::VersionNotFound => FilesError::NotFound,
        other => FilesError::Internal(other.to_string()),
    }
}

/// Look up the file and enforce the owner gate — the same check the sibling
/// `/api/files/{id}/...` handlers apply.
async fn owned_file(
    s: &HttpState,
    id: &str,
    session: &dochub_auth::AuthSession,
) -> Result<dochub_db::File, FilesError> {
    let file = FileRepo::new(&s.db)
        .find_by_id(id)
        .await
        .map_err(|_| FilesError::NotFound)?;
    if file.owner_id != session.user_id {
        return Err(FilesError::Forbidden);
    }
    Ok(file)
}

/// The lowercase file-name extension (the part after the last `.`), if any.
fn extension_of(name: &str) -> Option<String> {
    let idx = name.rfind('.')?;
    let ext = &name[idx + 1..];
    if ext.is_empty() {
        None
    } else {
        Some(ext.to_ascii_lowercase())
    }
}

/// Build a merged, size-capped line diff of two UTF-8 versions.
fn text_diff(from: &str, to: &str) -> DiffResp {
    let diff = TextDiff::from_lines(from, to);
    let mut hunks: Vec<Hunk> = Vec::new();
    let mut truncated = false;
    let mut total = 0usize;

    for change in diff.iter_all_changes() {
        let tag = match change.tag() {
            ChangeTag::Equal => "equal",
            ChangeTag::Delete => "delete",
            ChangeTag::Insert => "insert",
        };
        let line = change.value();
        total += line.len();
        if total > MAX_DIFF_BYTES {
            truncated = true;
            break;
        }
        match hunks.last_mut() {
            Some(h) if h.tag == tag => h.content.push_str(line),
            _ => hunks.push(Hunk {
                tag,
                content: line.to_string(),
            }),
        }
    }

    DiffResp::Text { hunks, truncated }
}

/// `GET /api/files/{id}/diff?from={a}&to={b}` — diff two committed versions.
/// 404 when either `seq` is unknown. Owner-gated like the sibling version
/// endpoints.
async fn diff(
    State(s): State<HttpState>,
    session: dochub_auth::AuthSession,
    Path(id): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Result<Json<DiffResp>, FilesError> {
    let file = owned_file(&s, &id, &session).await?;

    let registry = version_registry(&s);
    // Read both versions' *plaintext* bytes; an unknown seq is a 404.
    let from_bytes = registry
        .read_version(&id, q.from)
        .await
        .map_err(map_registry_err)?;
    let to_bytes = registry
        .read_version(&id, q.to)
        .await
        .map_err(map_registry_err)?;

    let is_text_ext = extension_of(&file.name)
        .as_deref()
        .is_some_and(|e| TEXT_EXTENSIONS.contains(&e));

    // Text branch only when the extension is a text format AND both versions
    // decode as UTF-8. A `.txt` holding non-UTF-8 bytes falls through to binary.
    if is_text_ext {
        if let (Ok(from_str), Ok(to_str)) = (
            std::str::from_utf8(&from_bytes),
            std::str::from_utf8(&to_bytes),
        ) {
            return Ok(Json(text_diff(from_str, to_str)));
        }
    }

    // Binary / opaque. `identical` compares the decrypted *plaintext* bytes.
    // NOTE: the version row's `content_hash` is SHA-256 of the *ciphertext*, and
    // every seal draws a fresh random nonce (`dochub-storage::blob::put_blob`),
    // so two commits of identical plaintext have different `content_hash`es.
    // Comparing plaintext is therefore the only correct content-equality test.
    Ok(Json(DiffResp::Binary {
        from_size: from_bytes.len(),
        to_size: to_bytes.len(),
        identical: from_bytes == to_bytes,
    }))
}

pub(crate) fn router(state: HttpState) -> Router {
    Router::new()
        .route("/api/files/{id}/diff", axum::routing::get(diff))
        .with_state(state)
}
