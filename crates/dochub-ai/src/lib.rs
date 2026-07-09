//! `dochub-ai` — the pluggable, **read-only**, audited AI provider layer
//! (Phase 3 build spec §3, PR P3.3 + P3.5).
//!
//! This crate is the seam between Doc-Hub and a large-language-model provider.
//! It never touches storage, the DB, or the encrypted version chain — it takes
//! plaintext text in and returns a [`Summary`] (and, later, embeddings /
//! completions). The layers above it own extraction, caching, auditing, and
//! tenant isolation; this crate is a small, unit-testable primitive so the
//! provider can be swapped without touching a handler.
//!
//! Design rules (CLAUDE.md — AI is pluggable, off by default, mock in CI):
//!
//! - **Pluggable.** [`AiProvider`] is an object-safe async trait. The default
//!   real provider is Claude via the Anthropic Messages API
//!   ([`AnthropicProvider`], Haiku for summaries); an air-gapped / local adapter
//!   is a documented follow-up.
//! - **Off by default.** [`AiConfig`] parses `DOCHUB_AI_PROVIDER` = `off` (the
//!   default) | `mock` | `anthropic`. When `off`, [`provider_from_config`]
//!   returns `None` and the HTTP layer treats AI as disabled.
//! - **Mock in CI.** [`MockProvider`] is deterministic and does **no** network
//!   I/O. It is what every test, CI run, and demo uses. [`AnthropicProvider`] is
//!   constructed only when explicitly configured and is never invoked in tests.
//! - **Secrets never leak.** The API key lives on [`AiConfig`] behind a
//!   redacting [`std::fmt::Debug`]; it never appears in logs, errors, or the
//!   provider's `Debug`.

#![forbid(unsafe_code)]

mod anthropic;
mod config;
mod mock;

pub use anthropic::{AnthropicProvider, DEFAULT_ANTHROPIC_MODEL};
pub use config::{provider_from_config, AiConfig, AiProviderKind};
pub use mock::{MockProvider, MOCK_MODEL};

use async_trait::async_trait;
use thiserror::Error;

/// The result of a summarization: the summary text plus provenance the audit
/// row records (`model` + token counts). Token counts let the compliance surface
/// account for spend without re-deriving it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    /// The generated summary text.
    pub text: String,
    /// Model id that produced it (e.g. `claude-haiku-4-5`, `mock-summarizer-v1`).
    pub model: String,
    /// Prompt (input) tokens the provider reported / estimated.
    pub input_tokens: u32,
    /// Completion (output) tokens the provider reported / estimated.
    pub output_tokens: u32,
}

/// Knobs for a summarization request. Deliberately small for P3.5; grows as the
/// summary surface does (per-section, length targets, language, …).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummarizeOpts {
    /// Target upper bound on the number of sentences in the summary. Providers
    /// treat it as a hint, not a hard cap.
    pub max_sentences: usize,
}

impl Default for SummarizeOpts {
    fn default() -> Self {
        Self { max_sentences: 3 }
    }
}

/// Errors surfaced by an [`AiProvider`]. None carry the API key or provider
/// credentials by construction.
#[derive(Debug, Error)]
pub enum AiError {
    /// The provider is configured `off`; callers should treat AI as disabled.
    #[error("ai provider disabled")]
    Disabled,
    /// The input text was empty — nothing to summarize.
    #[error("empty input")]
    EmptyInput,
    /// `anthropic` selected but no `DOCHUB_AI_API_KEY` was configured.
    #[error("ai api key not configured")]
    MissingApiKey,
    /// Transport / connection failure talking to the provider.
    #[error("ai provider request failed: {0}")]
    Request(String),
    /// The provider answered, but the response wasn't the shape we expect.
    #[error("ai provider returned an unexpected response: {0}")]
    Protocol(String),
}

/// A pluggable AI backend. Object-safe (`Arc<dyn AiProvider>`) so the HTTP layer
/// can hold whichever provider the config selected without generics.
///
/// **Read-only by contract**: implementations take text in and return derived
/// output; they never mutate documents, versions, or history. Every invocation
/// is audited one layer up.
#[async_trait]
pub trait AiProvider: Send + Sync + std::fmt::Debug {
    /// Summarize `text`. Returns the summary plus the model id and token counts
    /// the caller audits.
    async fn summarize(&self, text: &str, opts: SummarizeOpts) -> Result<Summary, AiError>;

    /// The model id this provider summarizes with. Handy for logging / display
    /// before a call is made.
    fn model(&self) -> &str;
}
