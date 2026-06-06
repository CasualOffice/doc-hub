//! `drive` — the Casual Drive binary.
//!
//! Phase 1 wires this into the real router. Today: a one-liner that proves
//! the workspace builds + the binary runs.

#![forbid(unsafe_code)]

use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Keep the Result return type — Phase 1 fills this in with real fallible work.
#[allow(clippy::unnecessary_wraps)]
fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,drive=debug".into()))
        .with(fmt::layer())
        .init();

    tracing::info!(
        "Casual Drive v{} — see PLAN.md Phase 1 for the real entry point.",
        env!("CARGO_PKG_VERSION")
    );
    Ok(())
}
