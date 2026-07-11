-- Tags — workspace-scoped labels on documents, plus the file↔tag join that
-- powers "search by tag". A tag is unique by (workspace_id, name). Portable
-- across SQLite + Postgres (TEXT ULIDs, ISO-8601 text timestamps, no
-- enums/JSONB). file_tags rows are removed explicitly on tag delete / unassign
-- (the app does not rely on FK cascade, which SQLite leaves off by default).

CREATE TABLE tags (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id),
  name         TEXT NOT NULL,
  -- Optional UI accent (e.g. a hex like '#8B5CF6'); NULL = default chip color.
  color        TEXT,
  created_at   TEXT NOT NULL,
  created_by   TEXT NOT NULL REFERENCES users(id)
);
-- One tag name per workspace.
CREATE UNIQUE INDEX tags_workspace_name_idx ON tags(workspace_id, name);

CREATE TABLE file_tags (
  file_id    TEXT NOT NULL REFERENCES files(id),
  tag_id     TEXT NOT NULL REFERENCES tags(id),
  created_at TEXT NOT NULL,
  created_by TEXT NOT NULL REFERENCES users(id),
  PRIMARY KEY (file_id, tag_id)
);
-- Read side for "which files carry tag X" (search-by-tag).
CREATE INDEX file_tags_tag_id_idx ON file_tags(tag_id);
