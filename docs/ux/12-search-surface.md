# 12 — Global search surface

Companion to `02-surface-v2.md` §"Top bar". Closes pipeline §3.7 — turns the existing top-bar search input from a current-folder filter into a real recursive-across-the-Drive search.

## Pattern reference

**Dropbox / Google Drive / Notion** — typing in the search field switches the main pane from "this folder" to "results across everything". The results pane lives in the same grid/list as the file browser so the rest of the chrome (preview modal, context menus, sort) keeps working unchanged.

We pick the same shape because:

1. Zero extra surface — the search field is already in the top bar.
2. Results re-use the existing FileCard / ListRow components, so context-menu + multi-select + preview all work for free.
3. No nested route — the "search mode" is a state flip inside `<Files />`, not a separate page.

## Behaviour

- Search input lives where it does today.
- When the trimmed query is empty: behaviour unchanged (renders the current folder).
- When the trimmed query is ≥ 2 chars: `<Files />` calls `GET /api/search?q=…&limit=50` and renders the response instead of `listChildren`.
- Header title flips from `My Drive` (or folder name) to **`Search results`** with the count chip + the matched query echoed alongside.
- Empty result: existing EmptyState component, copy `No files match "<query>".`
- Click a folder result → navigate into it (clears search).
- Click a file result → preview modal (current behaviour).
- Folder grouping rule from sort surface (`folders first`) is preserved.
- Debounce: 200 ms after the last keystroke before firing. Cancels the in-flight request on new input via an AbortController.

## Backend contract

### `GET /api/search?q=<query>&limit=<n>` (authed)

```json
{
  "files":   [ { /* FileDto, no thumbnails for tighter wire shape */ }, ... ],
  "folders": [ { /* FolderDto */ }, ... ]
}
```

- `q` is trimmed server-side; empty queries return empty arrays without hitting the DB.
- `limit` clamped to `[1, 200]`, default 50. Files and folders each get their own slice up to `limit`.
- Owner-scoped — admins still see only their own owned files in v0; multi-user share comes with RBAC in v0.2.
- Trashed files / folders are excluded.
- Match: case-insensitive substring against the display `name`. Database does this via `LOWER(name) LIKE LOWER(?)` with `%q%` placeholders. Phase-2 swaps to OpenSearch when DRIVE_OPENSEARCH_URL is set (pipeline §11.5 / Drive optional infra memory).

## State checklist

| | Required | Notes |
|---|---|---|
| Default (query length 0–1) | yes | renders the current folder, unchanged |
| Loading | yes | existing GridSkeleton, no flicker between keystrokes (debounce + AbortController) |
| Default (results) | yes | grid/list of matched entries, "Search results · 4 items" header |
| Empty | yes | EmptyState component, query echoed |
| Error | yes | inline ErrorState ("Couldn't reach the server.") |

## Out of scope (v0)

- Full-text search (file contents) — v0.2 alongside OpenSearch.
- Result snippets / highlight — v0.2.
- Searchable attributes beyond `name` (tags, share-link state, owner) — v0.2.
- Recently-opened / suggested results when the field is focused but empty — Phase 2 (cmdk command palette work, §2.8).
