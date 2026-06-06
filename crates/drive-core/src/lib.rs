//! Domain types, errors, and IDs shared across Casual Drive crates.
//!
//! Crates rule: this is the only crate that depends on nothing else in the
//! workspace. Everything else depends on it (or its narrower siblings).

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Opaque storage ID. ULID-encoded for sortability + opacity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub ulid::Ulid);

impl FileId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for FileId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Opaque folder ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FolderId(pub ulid::Ulid);

impl FolderId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new())
    }
}

impl Default for FolderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for FolderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Top-level error type. Specific crates layer their own errors and convert
/// up via `?`.
#[derive(Debug, Error)]
pub enum DriveError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}
