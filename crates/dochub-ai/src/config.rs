//! AI configuration + the [`provider_from_config`] factory.
//!
//! Env contract (all optional; AI is **off by default**):
//!
//! - `DOCHUB_AI_PROVIDER` — `off` (default) | `mock` | `anthropic`.
//! - `DOCHUB_AI_API_KEY`  — required only for `anthropic`. Never logged.
//! - `DOCHUB_AI_MODEL`    — model id; defaults to [`crate::DEFAULT_ANTHROPIC_MODEL`]
//!   (a Haiku id) for the `anthropic` provider.

use std::sync::Arc;

use crate::{AiError, AiProvider, AnthropicProvider, MockProvider, DEFAULT_ANTHROPIC_MODEL};

/// Which provider backs the AI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiProviderKind {
    /// AI is disabled; the HTTP layer returns a "disabled" status. The default —
    /// AI is off unless explicitly configured on (privacy, build spec D3).
    #[default]
    Off,
    /// Deterministic, no-network mock — tests / CI / demo.
    Mock,
    /// Claude via the Anthropic Messages API (real, network).
    Anthropic,
}

impl AiProviderKind {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "mock" => Self::Mock,
            "anthropic" | "claude" => Self::Anthropic,
            // Anything else (including "off", empty, or an unknown value) is
            // off — a typo must never silently enable content egress.
            _ => Self::Off,
        }
    }
}

/// AI configuration. Parsed from env by [`AiConfig::from_env`]; also embedded in
/// `dochub_core::Config`. `Default` is **off**.
///
/// The API key is held here but redacted from [`std::fmt::Debug`] so it can't
/// leak through a `{:?}` on `Config`.
#[derive(Clone, Default)]
pub struct AiConfig {
    pub provider: AiProviderKind,
    /// Provider API key. `None` unless `anthropic` is configured. Never logged.
    pub api_key: Option<String>,
    /// Model id. Defaults to a Haiku id for summaries.
    pub model: String,
}

impl std::fmt::Debug for AiConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiConfig")
            .field("provider", &self.provider)
            .field("api_key", &self.api_key.as_ref().map(|_| "<redacted>"))
            .field("model", &self.model)
            .finish()
    }
}

impl AiConfig {
    /// A disabled config (provider `off`). The default; handy in tests.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            provider: AiProviderKind::Off,
            api_key: None,
            model: DEFAULT_ANTHROPIC_MODEL.to_string(),
        }
    }

    /// A mock config (provider `mock`) — deterministic, no network. For tests.
    #[must_use]
    pub fn mock() -> Self {
        Self {
            provider: AiProviderKind::Mock,
            api_key: None,
            model: DEFAULT_ANTHROPIC_MODEL.to_string(),
        }
    }

    /// Whether AI is enabled (any provider other than `off`).
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.provider != AiProviderKind::Off
    }

    /// Parse from the environment. Never fails — an unknown/malformed provider
    /// value falls back to `off` so a misconfiguration disables AI rather than
    /// aborting boot or silently enabling content egress.
    #[must_use]
    pub fn from_env() -> Self {
        let provider = std::env::var("DOCHUB_AI_PROVIDER")
            .ok()
            .map_or(AiProviderKind::Off, |v| AiProviderKind::parse(&v));
        let api_key = std::env::var("DOCHUB_AI_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let model = std::env::var("DOCHUB_AI_MODEL")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_ANTHROPIC_MODEL.to_string());
        Self {
            provider,
            api_key,
            model,
        }
    }
}

/// Build the configured provider. Returns:
///
/// - `Ok(None)` when the provider is `off` — the caller treats AI as disabled.
/// - `Ok(Some(provider))` for `mock` / `anthropic`.
/// - `Err(AiError::MissingApiKey)` when `anthropic` is selected without a key.
///
/// The mock and Anthropic providers are constructed here; the Anthropic client
/// is built **only** when explicitly configured, never in tests.
pub fn provider_from_config(cfg: &AiConfig) -> Result<Option<Arc<dyn AiProvider>>, AiError> {
    match cfg.provider {
        AiProviderKind::Off => Ok(None),
        AiProviderKind::Mock => Ok(Some(Arc::new(MockProvider::new()))),
        AiProviderKind::Anthropic => {
            let key = cfg.api_key.clone().ok_or(AiError::MissingApiKey)?;
            let provider = AnthropicProvider::new(key, cfg.model.clone())?;
            Ok(Some(Arc::new(provider)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_defaults_to_off() {
        assert_eq!(AiProviderKind::parse(""), AiProviderKind::Off);
        assert_eq!(AiProviderKind::parse("off"), AiProviderKind::Off);
        assert_eq!(AiProviderKind::parse("nonsense"), AiProviderKind::Off);
        assert_eq!(AiProviderKind::parse("MOCK"), AiProviderKind::Mock);
        assert_eq!(
            AiProviderKind::parse("anthropic"),
            AiProviderKind::Anthropic
        );
        assert_eq!(AiProviderKind::parse("claude"), AiProviderKind::Anthropic);
    }

    #[test]
    fn default_is_off() {
        assert!(!AiConfig::default().is_enabled());
        assert_eq!(AiConfig::default().provider, AiProviderKind::Off);
    }

    #[test]
    fn off_yields_no_provider() {
        assert!(provider_from_config(&AiConfig::disabled())
            .unwrap()
            .is_none());
    }

    #[test]
    fn mock_yields_provider() {
        let p = provider_from_config(&AiConfig::mock()).unwrap();
        assert!(p.is_some());
        assert_eq!(p.unwrap().model(), crate::MOCK_MODEL);
    }

    #[test]
    fn anthropic_without_key_errors() {
        let cfg = AiConfig {
            provider: AiProviderKind::Anthropic,
            api_key: None,
            model: DEFAULT_ANTHROPIC_MODEL.to_string(),
        };
        assert!(matches!(
            provider_from_config(&cfg),
            Err(AiError::MissingApiKey)
        ));
    }

    #[test]
    fn debug_redacts_key() {
        let cfg = AiConfig {
            provider: AiProviderKind::Anthropic,
            api_key: Some("sk-super-secret".into()),
            model: "claude-haiku-4-5".into(),
        };
        let dbg = format!("{cfg:?}");
        assert!(!dbg.contains("sk-super-secret"));
        assert!(dbg.contains("<redacted>"));
    }
}
