//! Deterministic, network-free provider. This is what tests, CI, and the local
//! demo use — it never opens a socket, and identical input always yields
//! identical output (summary text + token counts), so assertions are stable.
//!
//! The "summary" is a transparent, deterministic condensation: the first N
//! sentences of the input (falling back to a leading character window when the
//! text has no sentence terminator). Token counts are a fake-but-stable
//! whitespace-word count of the input / output — enough to exercise the audit +
//! caching plumbing without pretending to be a real tokenizer.

use async_trait::async_trait;

use crate::{AiError, AiProvider, SummarizeOpts, Summary};

/// Stable model id the mock reports. Tests assert on this exact string, so it
/// must not change casually.
pub const MOCK_MODEL: &str = "mock-summarizer-v1";

/// A deterministic, no-network summarizer.
#[derive(Debug, Clone)]
pub struct MockProvider {
    model: String,
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MockProvider {
    #[must_use]
    pub fn new() -> Self {
        Self {
            model: MOCK_MODEL.to_string(),
        }
    }

    /// Construct with a caller-chosen model id (e.g. to distinguish two mock
    /// instances in a test). Rarely needed — [`MockProvider::new`] is the norm.
    #[must_use]
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
        }
    }
}

#[async_trait]
impl AiProvider for MockProvider {
    async fn summarize(&self, text: &str, opts: SummarizeOpts) -> Result<Summary, AiError> {
        if text.trim().is_empty() {
            return Err(AiError::EmptyInput);
        }
        let max = opts.max_sentences.max(1);
        let summary = first_sentences(text, max);
        Ok(Summary {
            text: summary.clone(),
            model: self.model.clone(),
            input_tokens: word_count(text),
            output_tokens: word_count(&summary),
        })
    }

    fn model(&self) -> &str {
        &self.model
    }
}

/// Whitespace-word count, as a `u32`. A fake-but-stable stand-in for a real
/// token count — deterministic for any given input.
fn word_count(s: &str) -> u32 {
    s.split_whitespace().count() as u32
}

/// The first `n` sentences of `text`, joined with a single space. A "sentence"
/// ends at `.`, `!`, or `?`. When the text has no terminator at all, falls back
/// to a leading char window so the summary is never empty for non-empty input.
fn first_sentences(text: &str, n: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    let mut cur = String::new();
    for ch in text.chars() {
        cur.push(ch);
        if matches!(ch, '.' | '!' | '?') {
            let sentence = cur.trim();
            if !sentence.is_empty() {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(sentence);
                count += 1;
                if count >= n {
                    break;
                }
            }
            cur.clear();
        }
    }
    if out.is_empty() {
        // No sentence terminator: take a leading window of the trimmed text.
        out = text.trim().chars().take(280).collect();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn summarize_is_deterministic_with_token_counts() {
        let p = MockProvider::new();
        let text = "Alpha beta gamma delta. Second sentence here. Third one now. A fourth.";
        let a = p.summarize(text, SummarizeOpts::default()).await.unwrap();
        let b = p.summarize(text, SummarizeOpts::default()).await.unwrap();
        // Deterministic: identical input ⇒ identical output.
        assert_eq!(a, b);
        // First 3 sentences (the default) are kept; the fourth is dropped.
        assert_eq!(
            a.text,
            "Alpha beta gamma delta. Second sentence here. Third one now."
        );
        assert_eq!(a.model, MOCK_MODEL);
        // Token counts are the stable word counts of input / output.
        assert_eq!(a.input_tokens, word_count(text));
        assert_eq!(a.output_tokens, word_count(&a.text));
        assert!(a.input_tokens > a.output_tokens);
    }

    #[tokio::test]
    async fn respects_max_sentences() {
        let p = MockProvider::new();
        let text = "One. Two. Three. Four.";
        let s = p
            .summarize(text, SummarizeOpts { max_sentences: 1 })
            .await
            .unwrap();
        assert_eq!(s.text, "One.");
    }

    #[tokio::test]
    async fn no_terminator_falls_back_to_window() {
        let p = MockProvider::new();
        let s = p
            .summarize("just a bare title with no period", SummarizeOpts::default())
            .await
            .unwrap();
        assert_eq!(s.text, "just a bare title with no period");
        assert!(s.output_tokens > 0);
    }

    #[tokio::test]
    async fn empty_input_is_error() {
        let p = MockProvider::new();
        assert!(matches!(
            p.summarize("   ", SummarizeOpts::default()).await,
            Err(AiError::EmptyInput)
        ));
    }
}
