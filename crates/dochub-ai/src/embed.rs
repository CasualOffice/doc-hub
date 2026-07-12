//! Embeddings — turn chunk text into a fixed-length vector for similarity
//! search, behind a pluggable [`Embedder`] trait.
//!
//! The trait lets the RAG pipeline stay provider-agnostic (CLAUDE.md: "a
//! pluggable LLM provider … with a local-model option for air-gapped
//! installs"). A hosted semantic embedder is a follow-up; this crate ships the
//! **offline baseline**, [`LocalEmbedder`], so the whole pipeline — chunk →
//! embed → store → retrieve — is testable and self-hostable with no network.
//!
//! [`LocalEmbedder`] uses signed feature hashing: each token is hashed into one
//! of `dims` buckets (its sign decided by another hash bit to cancel collision
//! bias), and the resulting vector is L2-normalized. This is a bag-of-words
//! embedding — it captures lexical overlap, not deep semantics — but it is
//! deterministic, dependency-free, and good enough to retrieve chunks that
//! share vocabulary with a query, which is the retrieval baseline a hosted
//! embedder later improves on behind the same trait.

use async_trait::async_trait;
use thiserror::Error;

/// A dense embedding vector.
pub type Embedding = Vec<f32>;

/// Errors an embedder can surface.
#[derive(Debug, Error)]
pub enum AiError {
    /// The provider failed (network, auth, rate limit, …).
    #[error("embedding provider error: {0}")]
    Provider(String),
    /// A returned vector had the wrong dimensionality.
    #[error("embedding dimension mismatch: expected {expected}, got {got}")]
    Dim { expected: usize, got: usize },
}

/// Produces embeddings for a batch of texts. Implementations must return one
/// vector per input, in order, each of length [`Embedder::dims`].
#[async_trait]
pub trait Embedder: Send + Sync {
    /// Dimensionality of every vector this embedder returns.
    fn dims(&self) -> usize;

    /// Embed each text in `texts`, returning vectors in the same order.
    async fn embed(&self, texts: &[String]) -> Result<Vec<Embedding>, AiError>;

    /// Convenience: embed a single text.
    async fn embed_one(&self, text: &str) -> Result<Embedding, AiError> {
        let mut out = self.embed(std::slice::from_ref(&text.to_string())).await?;
        out.pop().ok_or(AiError::Dim {
            expected: self.dims(),
            got: 0,
        })
    }
}

/// Offline, deterministic embedder (feature hashing). The self-hostable default
/// and the embedder every test uses.
#[derive(Debug, Clone)]
pub struct LocalEmbedder {
    dims: usize,
}

impl LocalEmbedder {
    /// New embedder producing `dims`-length vectors (`dims` >= 1).
    #[must_use]
    pub fn new(dims: usize) -> Self {
        Self { dims: dims.max(1) }
    }
}

impl Default for LocalEmbedder {
    /// 256 dimensions — enough buckets to keep hash collisions low for
    /// document-sized vocabularies while staying cheap to store and compare.
    fn default() -> Self {
        Self::new(256)
    }
}

#[async_trait]
impl Embedder for LocalEmbedder {
    fn dims(&self) -> usize {
        self.dims
    }

    async fn embed(&self, texts: &[String]) -> Result<Vec<Embedding>, AiError> {
        Ok(texts.iter().map(|t| embed_hashed(t, self.dims)).collect())
    }
}

/// Feature-hash `text` into a normalized `dims`-vector.
fn embed_hashed(text: &str, dims: usize) -> Embedding {
    let mut v = vec![0f32; dims];
    for token in tokenize(text) {
        let h = fnv1a(token.as_bytes());
        let bucket = (h % dims as u64) as usize;
        // A separate bit picks the sign so colliding-but-different tokens don't
        // all push the same bucket the same way.
        let sign = if (h >> 33) & 1 == 0 { 1.0 } else { -1.0 };
        v[bucket] += sign;
    }
    l2_normalize(&mut v);
    v
}

/// Lowercase alphanumeric tokens of length >= 2 (drops punctuation and
/// single-char noise).
fn tokenize(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|t| t.len() >= 2)
        .map(str::to_lowercase)
}

/// FNV-1a 64-bit — a fast, deterministic, dependency-free hash. Good enough for
/// feature bucketing (not cryptographic).
fn fnv1a(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Scale `v` to unit L2 norm in place (leaves an all-zero vector unchanged).
fn l2_normalize(v: &mut [f32]) {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dims_and_batch_size_match() {
        let e = LocalEmbedder::new(64);
        assert_eq!(e.dims(), 64);
        let out = e
            .embed(&["hello world".into(), "another one".into()])
            .await
            .unwrap();
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|v| v.len() == 64));
    }

    #[tokio::test]
    async fn deterministic_same_input_same_vector() {
        let e = LocalEmbedder::default();
        let a = e.embed_one("the quarterly revenue report").await.unwrap();
        let b = e.embed_one("the quarterly revenue report").await.unwrap();
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn vectors_are_unit_norm() {
        let e = LocalEmbedder::default();
        let v = e
            .embed_one("some text with several words here")
            .await
            .unwrap();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5, "norm was {norm}");
    }

    #[tokio::test]
    async fn empty_text_is_zero_vector() {
        let e = LocalEmbedder::new(32);
        let v = e.embed_one("").await.unwrap();
        assert!(v.iter().all(|&x| x.abs() < f32::EPSILON));
    }

    #[tokio::test]
    async fn overlapping_texts_are_closer_than_disjoint() {
        use crate::retrieve::cosine;
        let e = LocalEmbedder::default();
        let base = e
            .embed_one("annual budget for the marketing department")
            .await
            .unwrap();
        let similar = e
            .embed_one("the marketing department annual budget plan")
            .await
            .unwrap();
        let different = e
            .embed_one("photosynthesis in tropical rainforest plants")
            .await
            .unwrap();
        assert!(
            cosine(&base, &similar) > cosine(&base, &different),
            "shared-vocabulary text should score higher"
        );
    }
}
