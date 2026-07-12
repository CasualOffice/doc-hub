//! Text chunking — split a document's extracted plaintext into overlapping
//! windows small enough to embed and retrieve individually.
//!
//! Retrieval quality depends on chunk size: too large and a hit's snippet is
//! diluted; too small and it loses context. We use a sliding character window
//! with overlap, preferring to break on whitespace so a chunk never ends
//! mid-word. Chunking is deterministic and pure — no allocation beyond the
//! returned chunks — so it is trivially unit-testable and reproducible across
//! runs (the same document always chunks the same way, which keeps re-embedding
//! idempotent).

use serde::{Deserialize, Serialize};

/// Chunking parameters. Defaults target ~1000-char chunks with 150 chars of
/// overlap — a common RAG starting point that keeps a chunk within a few
/// hundred tokens while preserving cross-boundary context.
#[derive(Debug, Clone, Copy)]
pub struct ChunkConfig {
    /// Maximum characters per chunk (hard upper bound).
    pub max_chars: usize,
    /// Characters of overlap carried from the end of one chunk into the start
    /// of the next. Clamped below `max_chars` so the window always advances.
    pub overlap_chars: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            max_chars: 1000,
            overlap_chars: 150,
        }
    }
}

/// One chunk of a document, with its character span in the source text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chunk {
    /// 0-based position of this chunk within the document.
    pub index: usize,
    /// The chunk's text.
    pub text: String,
    /// Inclusive char offset where the chunk starts in the source.
    pub char_start: usize,
    /// Exclusive char offset where the chunk ends in the source.
    pub char_end: usize,
}

/// Split `text` into overlapping [`Chunk`]s per `cfg`.
///
/// Empty or whitespace-only input yields no chunks. Input shorter than
/// `max_chars` yields a single chunk. Otherwise a window of at most `max_chars`
/// slides forward by `max_chars - overlap`, backing off to the last whitespace
/// in the second half of the window so chunks break on word boundaries where
/// possible.
#[must_use]
pub fn chunk_text(text: &str, cfg: &ChunkConfig) -> Vec<Chunk> {
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    if text.trim().is_empty() {
        return Vec::new();
    }

    let max_chars = cfg.max_chars.max(1);
    // Guarantee forward progress: overlap can never consume the whole window.
    let overlap = cfg.overlap_chars.min(max_chars.saturating_sub(1));
    let step = max_chars - overlap;

    let mut chunks = Vec::new();
    let mut start = 0;
    while start < n {
        let hard_end = (start + max_chars).min(n);
        // Prefer a whitespace break in the back half of the window, unless this
        // is the final chunk (hard_end == n) where we take everything left.
        let end = if hard_end < n {
            break_at_whitespace(&chars, start, hard_end)
        } else {
            hard_end
        };

        let piece: String = chars[start..end].iter().collect();
        let trimmed = piece.trim();
        if !trimmed.is_empty() {
            chunks.push(Chunk {
                index: chunks.len(),
                text: trimmed.to_string(),
                char_start: start,
                char_end: end,
            });
        }

        if end >= n {
            break;
        }
        // Advance by `step` from the window start, but never past `end` (which a
        // whitespace back-off could otherwise allow) and always by at least 1.
        start = (start + step).min(end).max(start + 1);
    }
    chunks
}

/// Find a good break point in `chars[start..hard_end]`: the last whitespace at
/// or after the window's midpoint, so we don't cut a word and don't shrink the
/// chunk too aggressively. Falls back to `hard_end` when there is none.
fn break_at_whitespace(chars: &[char], start: usize, hard_end: usize) -> usize {
    let midpoint = start + (hard_end - start) / 2;
    for i in (midpoint..hard_end).rev() {
        if chars[i].is_whitespace() {
            return i;
        }
    }
    hard_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_whitespace_yield_no_chunks() {
        assert!(chunk_text("", &ChunkConfig::default()).is_empty());
        assert!(chunk_text("   \n\t ", &ChunkConfig::default()).is_empty());
    }

    #[test]
    fn short_text_is_one_chunk() {
        let chunks = chunk_text("just a short note", &ChunkConfig::default());
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].text, "just a short note");
        assert_eq!(chunks[0].index, 0);
    }

    #[test]
    fn long_text_splits_with_overlap_and_covers_everything() {
        // 50 space-separated words → well over a small max_chars.
        let words: Vec<String> = (0..50).map(|i| format!("word{i}")).collect();
        let text = words.join(" ");
        let cfg = ChunkConfig {
            max_chars: 40,
            overlap_chars: 10,
        };
        let chunks = chunk_text(&text, &cfg);
        assert!(chunks.len() > 1, "should split into several chunks");

        // Indices are sequential.
        for (i, c) in chunks.iter().enumerate() {
            assert_eq!(c.index, i);
            assert!(c.text.chars().count() <= cfg.max_chars);
        }
        // Consecutive chunks overlap (next starts before previous ended).
        for pair in chunks.windows(2) {
            assert!(
                pair[1].char_start < pair[0].char_end,
                "expected overlap between chunks"
            );
        }
        // Every word survives somewhere.
        let joined: String = chunks
            .iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join(" ");
        for i in 0..50 {
            assert!(joined.contains(&format!("word{i}")), "missing word{i}");
        }
    }

    #[test]
    fn overlap_larger_than_window_still_terminates() {
        let text = "a ".repeat(500);
        let cfg = ChunkConfig {
            max_chars: 20,
            overlap_chars: 1000, // absurd — must be clamped
        };
        let chunks = chunk_text(&text, &cfg);
        assert!(!chunks.is_empty());
        // No infinite loop = test returns; also each chunk is bounded.
        assert!(chunks.iter().all(|c| c.text.chars().count() <= 20));
    }

    #[test]
    fn breaks_on_whitespace_not_mid_word() {
        let text = "alpha bravo charlie delta echo foxtrot golf hotel india juliet";
        let cfg = ChunkConfig {
            max_chars: 25,
            overlap_chars: 5,
        };
        let chunks = chunk_text(text, &cfg);
        // No chunk should start or end splitting a word: trimmed text has no
        // leading/trailing partial token adjacency we can cheaply assert, but we
        // can assert each chunk's words are all whole words from the source.
        let source_words: std::collections::HashSet<&str> = text.split(' ').collect();
        for c in &chunks {
            for w in c.text.split_whitespace() {
                assert!(source_words.contains(w), "chunk split a word: {w:?}");
            }
        }
    }
}
