//! Retrieval — cosine similarity and top-k nearest-neighbour selection over a
//! candidate set of embeddings.
//!
//! Doc-Hub's metadata store is SQLite/Postgres with **no vector extension**
//! (the portability rule bans them), so nearest-neighbour search is brute-force
//! cosine over a workspace's chunk vectors. That is linear in the number of
//! chunks but trivial at the scale a single workspace holds, needs no extra
//! infrastructure, and keeps the math here — pure, testable, and independent of
//! where the vectors are stored.

/// Cosine similarity of two vectors in `[-1, 1]`. Returns `0.0` when the
/// vectors differ in length or either is all-zero (no shared direction).
#[must_use]
pub fn cosine(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom > 0.0 {
        dot / denom
    } else {
        0.0
    }
}

/// One retrieval result: the candidate's id and its similarity to the query.
#[derive(Debug, Clone, PartialEq)]
pub struct Scored<T> {
    pub item: T,
    pub score: f32,
}

/// Return the `k` candidates most similar to `query`, highest score first.
///
/// `candidates` pairs an id with its embedding. Ties break by input order
/// (stable sort). Fewer than `k` results are returned when there are fewer
/// candidates. A `min_score` floor drops weak matches (`f32::MIN` keeps all).
#[must_use]
pub fn top_k<T: Clone>(
    query: &[f32],
    candidates: &[(T, Vec<f32>)],
    k: usize,
    min_score: f32,
) -> Vec<Scored<T>> {
    let mut scored: Vec<Scored<T>> = candidates
        .iter()
        .map(|(item, vec)| Scored {
            item: item.clone(),
            score: cosine(query, vec),
        })
        .filter(|s| s.score >= min_score)
        .collect();
    // Descending by score; stable so equal scores keep input order.
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    scored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_of_identical_is_one() {
        let v = vec![0.3, 0.4, 0.5];
        assert!((cosine(&v, &v) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn cosine_of_orthogonal_is_zero() {
        assert!(cosine(&[1.0, 0.0], &[0.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn cosine_handles_length_mismatch_and_zero() {
        assert!(cosine(&[1.0, 2.0], &[1.0]).abs() < 1e-6);
        assert!(cosine(&[0.0, 0.0], &[1.0, 1.0]).abs() < 1e-6);
    }

    #[test]
    fn top_k_ranks_and_limits() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            ("a", vec![1.0, 0.0]), // identical → 1.0
            ("b", vec![0.9, 0.1]), // close
            ("c", vec![0.0, 1.0]), // orthogonal → 0.0
        ];
        let hits = top_k(&query, &candidates, 2, f32::MIN);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].item, "a");
        assert_eq!(hits[1].item, "b");
    }

    #[test]
    fn top_k_min_score_floor_drops_weak_matches() {
        let query = vec![1.0, 0.0];
        let candidates = vec![("a", vec![1.0, 0.0]), ("c", vec![0.0, 1.0])];
        let hits = top_k(&query, &candidates, 10, 0.5);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].item, "a");
    }
}
