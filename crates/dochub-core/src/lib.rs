//! Domain types, errors, IDs, and runtime configuration shared across the
//! Casual Drive workspace.
//!
//! This crate depends on `dochub-crypto` (the leaf crypto crate, so `Config` can
//! hold the master KEK — build spec §8) and `dochub-ai` (the leaf AI-provider
//! crate, so `Config` can carry the parsed [`AiConfig`] — build spec §3). Both
//! are leaves with no workspace dependencies of their own, so there is no cycle.
//! Everything else in the workspace depends on this crate.

#![forbid(unsafe_code)]

pub mod config;
pub mod error;
pub mod id;
pub mod ingest;

pub use config::{dev_master_kek, dev_master_kek_next, Backend, Config, ConfigError, OidcConfig};
// Phase 3 §3 — the AI provider layer lives in `dochub-ai`; re-export the config
// types so callers (and test fixtures) that already depend on `dochub-core` can
// name them without a direct `dochub-ai` dependency.
pub use dochub_ai::{AiConfig, AiProviderKind};
pub use error::DriveError;
pub use id::{FileId, FolderId};
pub use ingest::{guard, DocKind, IngestError, ALLOWED_EXTENSIONS};
