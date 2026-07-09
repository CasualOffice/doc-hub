-- Phase 3 P3.5 — document-summary cache (build spec §3, "Summaries … cached by
-- content_hash").
--
-- The AI layer (`dochub-ai`) is read-only + audited. A summary is a pure
-- function of the document bytes, so it is cached by the head version's
-- `content_hash`: the same bytes never re-hit the provider, and a new version
-- (new head, new hash) is a natural cache miss that produces a fresh summary.
-- No row here is ever UPDATEd — a superseded head simply leaves a stale
-- (file_id, content_hash) row that is never read again (history is append-only,
-- CLAUDE.md rule 6); a re-summarize of the *same* head is idempotent.
--
-- Keyed for lookup by (file_id, content_hash) — the composite primary key is
-- exactly the cache key the summary endpoint probes on.
--
-- `file_id` is a loose reference, deliberately *without* a foreign key — like
-- `audit_log.target_id`, this is derived/compliance-adjacent data, not part of
-- the immutable record. A stale entry for a since-removed file is harmless and
-- simply never read again (and files are tombstoned, never hard-deleted, per
-- CLAUDE.md rule 6). Omitting the FK also keeps the cache independently
-- writable/testable without seeding the full files graph.
--
-- Portable across SQLite + Postgres: TEXT ids/hashes, INTEGER token counts,
-- ISO-8601 UTC `created_at`. No JSONB / enum / native UUID.

CREATE TABLE ai_summaries (
  file_id       TEXT    NOT NULL,          -- file the summary is for (loose ref, no FK)
  content_hash  TEXT    NOT NULL,          -- head version's content_hash the summary was built from
  summary       TEXT    NOT NULL,          -- the generated summary text
  model         TEXT    NOT NULL,          -- provider model id (e.g. claude-haiku-4-5)
  input_tokens  INTEGER NOT NULL,          -- prompt tokens the provider reported / estimated
  output_tokens INTEGER NOT NULL,          -- completion tokens
  created_at    TEXT    NOT NULL,          -- ISO-8601 UTC
  PRIMARY KEY (file_id, content_hash)
);
