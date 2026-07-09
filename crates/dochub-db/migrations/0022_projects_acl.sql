-- Foundation F1 — projects, project memberships, and per-resource ACL grants.
-- Spec: docs/design/foundation-access-rag-mcp.md §3.
--
-- A Project is an access container inside a workspace. Folders/files gain a
-- nullable project_id (backfilled to a per-workspace default project in the
-- companion 0023 migration). acl_grants hold explicit per-resource grants,
-- including user-to-user sharing (subject_kind = 'user'). All portable across
-- SQLite + Postgres (TEXT ULIDs, ISO-8601 timestamps, no enums/JSONB).

CREATE TABLE projects (
  id           TEXT PRIMARY KEY,
  workspace_id TEXT NOT NULL REFERENCES workspaces(id),
  name         TEXT NOT NULL,
  -- "team" | "personal" — mirrors the owning workspace's kind.
  kind         TEXT NOT NULL,
  created_at   TEXT NOT NULL
);
CREATE INDEX projects_workspace_id_idx ON projects(workspace_id);

CREATE TABLE project_members (
  project_id TEXT NOT NULL REFERENCES projects(id),
  user_id    TEXT NOT NULL REFERENCES users(id),
  -- "viewer" | "editor" | "admin" | "owner" (authz::Role). Absence of a row
  -- means the user inherits their workspace role for this project.
  role       TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (project_id, user_id)
);
CREATE INDEX project_members_user_id_idx ON project_members(user_id);

CREATE TABLE acl_grants (
  id            TEXT PRIMARY KEY,
  -- "workspace" | "project" | "folder" | "file"
  resource_kind TEXT NOT NULL,
  resource_id   TEXT NOT NULL,
  -- "user" | "role"
  subject_kind  TEXT NOT NULL,
  -- a user id (subject_kind='user') or a role name (subject_kind='role').
  subject_id    TEXT NOT NULL,
  -- granted authz::Role ("viewer" | "editor" | "admin" | "owner").
  role          TEXT NOT NULL,
  created_at    TEXT NOT NULL,
  created_by    TEXT NOT NULL
);
CREATE INDEX acl_grants_resource_idx ON acl_grants(resource_kind, resource_id);
CREATE INDEX acl_grants_subject_idx ON acl_grants(subject_kind, subject_id);

-- Folders/files gain a nullable project_id (populated by 0023).
ALTER TABLE folders ADD COLUMN project_id TEXT;
ALTER TABLE files ADD COLUMN project_id TEXT;
CREATE INDEX folders_project_id_idx ON folders(project_id);
CREATE INDEX files_project_id_idx ON files(project_id);
