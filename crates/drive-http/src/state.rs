//! Shared HTTP layer state. Cheap to clone — everything is `Arc` internally.

use std::{sync::Arc, time::Instant};

use axum::extract::FromRef;
use drive_auth::AuthState;
use drive_core::Config;
use drive_db::Db;
use drive_storage::Storage;
use drive_wopi::WopiState;

use crate::rate_limit::{RateLimitConfig, RateLimiter};

/// Process start instant, captured at first state construction. Drives the
/// Admin → System → Uptime readout. Static so we get "real" uptime even
/// across cheap HttpState clones in tests.
fn process_started_at() -> Instant {
    use std::sync::OnceLock;
    static STARTED: OnceLock<Instant> = OnceLock::new();
    *STARTED.get_or_init(Instant::now)
}

#[derive(Clone)]
pub struct HttpState {
    pub storage: Storage,
    pub wopi: WopiState,
    pub db: Db,
    pub auth: AuthState,
    pub jwt_secret: Arc<[u8; 32]>,
    pub config: Arc<Config>,
    /// Upload-throttle bucket per user (pipeline §6.5). Cheap to clone
    /// — the limiter is `Arc<Mutex<HashMap>>`. Constructed via
    /// `HttpState::with_default_upload_limit` so call sites don't have
    /// to know the numbers.
    pub upload_limiter: Arc<RateLimiter>,
}

impl HttpState {
    /// Default upload limiter: 30 uploads per minute per user (burst of
    /// 30, refill at 0.5 / sec). Adjust via the constructor below when
    /// the operator dials it down for shared instances.
    #[must_use]
    pub fn default_upload_limiter() -> Arc<RateLimiter> {
        Arc::new(RateLimiter::new(RateLimitConfig {
            capacity: 30.0,
            refill_per_sec: 0.5,
        }))
    }
}

impl HttpState {
    /// Seconds since the process started. Capped at `u64`.
    #[must_use]
    pub fn uptime_seconds(&self) -> u64 {
        process_started_at().elapsed().as_secs()
    }
}

impl std::fmt::Debug for HttpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpState")
            .field("storage", &self.storage)
            .field("backend", &self.config.backend)
            .field("db_backend", &self.db.backend())
            .finish_non_exhaustive()
    }
}

// `FromRef` lets the AuthSession extractor pull AuthState out of HttpState
// at request time without forcing every handler to take both.
impl FromRef<HttpState> for AuthState {
    fn from_ref(state: &HttpState) -> Self {
        state.auth.clone()
    }
}
