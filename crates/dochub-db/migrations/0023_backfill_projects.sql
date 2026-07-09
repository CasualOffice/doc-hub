-- Foundation F1 backfill — one default project per existing workspace, wire
-- existing folders/files to it, and migrate workspace roles to the new model.
-- Spec: docs/design/foundation-access-rag-mcp.md §3 (migration note).
--
-- Idempotent + portable (SQLite + Postgres). Generating a ULID in SQL is not
-- portable, so the default project reuses its workspace's id as its own PK
-- (distinct tables → no collision). That makes the "does a default project
-- already exist" check trivial and re-runnable, and lets the folder/file
-- backfill set project_id = workspace_id directly.

-- 1. Default project per workspace that has none yet.
INSERT INTO projects (id, workspace_id, name, kind, created_at)
SELECT w.id, w.id, 'General', w.kind, w.created_at
FROM workspaces w
WHERE NOT EXISTS (SELECT 1 FROM projects p WHERE p.workspace_id = w.id);

-- 2. Point existing folders/files at their workspace's default project.
UPDATE folders SET project_id = workspace_id
WHERE project_id IS NULL AND workspace_id IS NOT NULL;
UPDATE files SET project_id = workspace_id
WHERE project_id IS NULL AND workspace_id IS NOT NULL;

-- 3. Map legacy workspace roles onto the new model: member -> editor,
-- owner stays owner. Idempotent (no 'member' rows remain after the first run).
UPDATE workspace_members SET role = 'editor' WHERE role = 'member';
