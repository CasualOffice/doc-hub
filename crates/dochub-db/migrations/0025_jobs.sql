-- Durable background-job queue — the shared substrate for content indexing,
-- AI embeds, and (later) agentic pipelines. A worker claims the next runnable
-- job, processes it, then marks it done or reschedules with backoff. Survives
-- restarts (state lives in the DB, not memory). Portable SQLite + Postgres
-- (TEXT ULIDs, ISO-8601 text timestamps, no enums/JSONB).

CREATE TABLE jobs (
  id           TEXT PRIMARY KEY,
  -- Handler discriminator, e.g. 'index_file' | 'embed_file'.
  kind         TEXT NOT NULL,
  -- Opaque JSON payload the handler for `kind` understands.
  payload      TEXT NOT NULL,
  -- 'queued' | 'running' | 'done' | 'failed'.
  state        TEXT NOT NULL DEFAULT 'queued',
  attempts     INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 5,
  -- Not runnable before this time (scheduling + retry backoff).
  run_after    TEXT NOT NULL,
  last_error   TEXT,
  created_at   TEXT NOT NULL,
  updated_at   TEXT NOT NULL
);
-- The claim query scans queued jobs that are due (run_after <= now).
CREATE INDEX jobs_claim_idx ON jobs(state, run_after);
