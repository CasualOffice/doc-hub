//! Claude via the Anthropic **Messages API** (`POST /v1/messages`).
//!
//! This is the default *real* provider. It is constructed only when the operator
//! sets `DOCHUB_AI_PROVIDER=anthropic` with a `DOCHUB_AI_API_KEY`, and it is
//! **never** invoked in tests or CI — every test path uses [`crate::MockProvider`]
//! (no network). The model defaults to a Haiku id ([`DEFAULT_ANTHROPIC_MODEL`]),
//! which is the right cost/speed tier for extraction-shaped work like summaries.
//!
//! Transport is `reqwest` with the rustls-tls backend (no OpenSSL). The API key
//! is held privately and never appears in `Debug`, logs, or errors.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use crate::{AiError, AiProvider, SummarizeOpts, Summary};

/// Default Haiku model id for summaries. Haiku is the cost/latency-appropriate
/// tier for extraction/summarization (Sonnet/Opus are reserved for Q&A/reasoning
/// per the locked stack in CLAUDE.md).
pub const DEFAULT_ANTHROPIC_MODEL: &str = "claude-haiku-4-5";

/// Anthropic API version header value. Pinned; bump deliberately.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default Messages API base URL. Overridable (constructor) so a self-hosted
/// gateway can be pointed at, and so a future integration test could target a
/// stub — CI never does.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// Output-token ceiling for a summary request. Summaries are short; this bounds
/// spend and latency.
const MAX_TOKENS: u32 = 1024;

/// Claude Messages API provider. Cheap to clone (the inner `reqwest::Client`
/// shares a connection pool).
#[derive(Clone)]
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
    base_url: String,
}

impl std::fmt::Debug for AnthropicProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never render the API key.
        f.debug_struct("AnthropicProvider")
            .field("model", &self.model)
            .field("base_url", &self.base_url)
            .field("api_key", &"<redacted>")
            // `client` (a reqwest::Client) is intentionally omitted.
            .finish_non_exhaustive()
    }
}

impl AnthropicProvider {
    /// Build a provider from an API key and model id. Fails only if the HTTP
    /// client can't be constructed (rustls/TLS init).
    pub fn new(api_key: String, model: String) -> Result<Self, AiError> {
        Self::with_base_url(api_key, model, DEFAULT_BASE_URL.to_string())
    }

    /// Like [`AnthropicProvider::new`] but with an explicit base URL.
    pub fn with_base_url(
        api_key: String,
        model: String,
        base_url: String,
    ) -> Result<Self, AiError> {
        if api_key.trim().is_empty() {
            return Err(AiError::MissingApiKey);
        }
        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| AiError::Request(e.to_string()))?;
        let model = if model.trim().is_empty() {
            DEFAULT_ANTHROPIC_MODEL.to_string()
        } else {
            model
        };
        Ok(Self {
            client,
            api_key,
            model,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }
}

/// The subset of the Messages API response we read.
#[derive(Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    usage: Usage,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Deserialize, Default)]
struct Usage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    async fn summarize(&self, text: &str, opts: SummarizeOpts) -> Result<Summary, AiError> {
        if text.trim().is_empty() {
            return Err(AiError::EmptyInput);
        }
        let max_sentences = opts.max_sentences.max(1);
        let prompt = format!(
            "Summarize the following document in at most {max_sentences} sentence(s). \
             Respond with only the summary, no preamble.\n\n{text}"
        );
        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "messages": [{ "role": "user", "content": prompt }],
        });

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Request(e.to_string()))?;

        let status = resp.status();
        if !status.is_success() {
            // Body may carry a provider error message, but never the key.
            let detail = resp.text().await.unwrap_or_default();
            return Err(AiError::Request(format!("status {status}: {detail}")));
        }

        let parsed: MessagesResponse = resp
            .json()
            .await
            .map_err(|e| AiError::Protocol(e.to_string()))?;

        if parsed.stop_reason.as_deref() == Some("refusal") {
            return Err(AiError::Protocol("provider refused the request".into()));
        }

        let summary_text: String = parsed
            .content
            .iter()
            .filter(|b| b.kind == "text")
            .map(|b| b.text.as_str())
            .collect::<Vec<_>>()
            .join("")
            .trim()
            .to_string();

        if summary_text.is_empty() {
            return Err(AiError::Protocol("empty summary in response".into()));
        }

        Ok(Summary {
            text: summary_text,
            model: parsed.model.unwrap_or_else(|| self.model.clone()),
            input_tokens: parsed.usage.input_tokens,
            output_tokens: parsed.usage.output_tokens,
        })
    }

    fn model(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NB: no network tests here — CI never talks to the provider. These only
    // exercise construction + the secret-redaction invariant.

    #[test]
    fn missing_key_is_error() {
        assert!(matches!(
            AnthropicProvider::new(String::new(), "claude-haiku-4-5".into()),
            Err(AiError::MissingApiKey)
        ));
    }

    #[test]
    fn debug_redacts_api_key() {
        let p = AnthropicProvider::new("sk-secret-value".into(), "claude-haiku-4-5".into())
            .expect("build");
        let dbg = format!("{p:?}");
        assert!(!dbg.contains("sk-secret-value"));
        assert!(dbg.contains("<redacted>"));
        assert_eq!(p.model(), "claude-haiku-4-5");
    }

    #[test]
    fn empty_model_falls_back_to_default() {
        let p = AnthropicProvider::new("sk-x".into(), String::new()).expect("build");
        assert_eq!(p.model(), DEFAULT_ANTHROPIC_MODEL);
    }
}
