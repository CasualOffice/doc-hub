//! Document-summary cache (build spec §3, P3.5). A summary is a pure function of
//! the document bytes, so it is cached by the head version's `content_hash`: a
//! cache hit skips the provider call *and* the audit write; a new version (new
//! head hash) is a natural miss.
//!
//! This repo is a thin key/value over the `ai_summaries` table
//! (migration 0022): [`AiSummaryRepo::get_cached_summary`] probes the
//! `(file_id, content_hash)` key, [`AiSummaryRepo::put_summary`] writes one. No
//! row is ever UPDATEd/DELETEd — `put_summary` is idempotent on the key.

use sqlx::Row;

use crate::{
    users::{parse_ts, ts},
    Db, DbError,
};

/// A cached summary row, as read back from the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedSummary {
    pub summary: String,
    pub model: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub created_at: time::OffsetDateTime,
}

#[derive(Debug, Clone)]
pub struct AiSummaryRepo<'a> {
    db: &'a Db,
}

impl<'a> AiSummaryRepo<'a> {
    #[must_use]
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    /// Look up a cached summary for `(file_id, content_hash)`. `Ok(None)` on a
    /// miss.
    pub async fn get_cached_summary(
        &self,
        file_id: &str,
        content_hash: &str,
    ) -> Result<Option<CachedSummary>, DbError> {
        let row = sqlx::query(
            "SELECT summary, model, input_tokens, output_tokens, created_at \
             FROM ai_summaries WHERE file_id = ? AND content_hash = ?",
        )
        .bind(file_id)
        .bind(content_hash)
        .fetch_optional(self.db.pool())
        .await?;

        let Some(row) = row else {
            return Ok(None);
        };
        Ok(Some(CachedSummary {
            summary: row.get("summary"),
            model: row.get("model"),
            input_tokens: row.get("input_tokens"),
            output_tokens: row.get("output_tokens"),
            created_at: parse_ts(row.get::<String, _>("created_at"))?,
        }))
    }

    /// Cache a summary for `(file_id, content_hash)`. Idempotent: a repeated
    /// write for the same key is a no-op (`ON CONFLICT DO NOTHING`, portable
    /// across SQLite + Postgres), so the first-committed summary for a given head
    /// wins and is never overwritten.
    #[allow(clippy::too_many_arguments)]
    pub async fn put_summary(
        &self,
        file_id: &str,
        content_hash: &str,
        summary: &str,
        model: &str,
        input_tokens: i64,
        output_tokens: i64,
    ) -> Result<(), DbError> {
        let created_at = ts(time::OffsetDateTime::now_utc());
        sqlx::query(
            "INSERT INTO ai_summaries \
             (file_id, content_hash, summary, model, input_tokens, output_tokens, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT (file_id, content_hash) DO NOTHING",
        )
        .bind(file_id)
        .bind(content_hash)
        .bind(summary)
        .bind(model)
        .bind(input_tokens)
        .bind(output_tokens)
        .bind(&created_at)
        .execute(self.db.pool())
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_db() -> Db {
        Db::connect("sqlite::memory:").await.expect("connect")
    }

    #[tokio::test]
    async fn miss_then_hit_round_trips() {
        let db = fresh_db().await;
        let repo = AiSummaryRepo::new(&db);

        assert!(repo
            .get_cached_summary("F_1", "hash-a")
            .await
            .unwrap()
            .is_none());

        repo.put_summary(
            "F_1",
            "hash-a",
            "a short summary",
            "mock-summarizer-v1",
            12,
            4,
        )
        .await
        .unwrap();

        let got = repo
            .get_cached_summary("F_1", "hash-a")
            .await
            .unwrap()
            .expect("cached");
        assert_eq!(got.summary, "a short summary");
        assert_eq!(got.model, "mock-summarizer-v1");
        assert_eq!(got.input_tokens, 12);
        assert_eq!(got.output_tokens, 4);
    }

    #[tokio::test]
    async fn distinct_hash_is_a_distinct_entry() {
        let db = fresh_db().await;
        let repo = AiSummaryRepo::new(&db);
        repo.put_summary("F_1", "hash-a", "old head summary", "m", 1, 1)
            .await
            .unwrap();
        // Same file, new head hash ⇒ miss (produces a fresh summary upstream).
        assert!(repo
            .get_cached_summary("F_1", "hash-b")
            .await
            .unwrap()
            .is_none());
    }

    #[tokio::test]
    async fn put_is_idempotent_on_key() {
        let db = fresh_db().await;
        let repo = AiSummaryRepo::new(&db);
        repo.put_summary("F_1", "hash-a", "first", "m", 1, 1)
            .await
            .unwrap();
        // A second write for the same key does not overwrite (DO NOTHING).
        repo.put_summary("F_1", "hash-a", "second", "m", 2, 2)
            .await
            .unwrap();
        let got = repo
            .get_cached_summary("F_1", "hash-a")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(got.summary, "first");
        assert_eq!(got.input_tokens, 1);
    }
}
