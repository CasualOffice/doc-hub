/**
 * SR6 — encode / decode search state ↔ URL params.
 *
 * Spec: docs/ux/12-search-surface.md §"State checklist".
 *
 * Bookmarkable, reload-safe shape — `?q=…&t=pdf,image&sort=name&dir=asc`.
 * The keys are deliberately short so a copy-pasted URL doesn't look
 * like a tracking blob. Defaults are omitted (no `&scope=workspace`,
 * no `&sort=relevance&dir=desc`) so the URL stays clean when the user
 * is just browsing.
 *
 * Both halves are pure functions over a plain `URLSearchParams` so
 * they're trivially unit-testable and don't touch `window`.
 */
import {
  defaultFilters,
  hasActiveFilters,
  type SearchFilters,
  type SearchScope,
  type SortBy,
  type SortDir,
  type TypeBucket,
} from "../api/client.ts";

/** Subset of TypeBucket values we accept from URL input. Anything
 * outside this allow-list is silently dropped — a malformed `?t=`
 * shouldn't poison the filter chip row. */
const TYPE_BUCKETS: ReadonlySet<TypeBucket> = new Set([
  "folder",
  "document",
  "spreadsheet",
  "pdf",
  "image",
  "video",
  "audio",
  "markdown",
  "archive",
  "other",
  "note",
]);

const SORT_BYS: ReadonlySet<SortBy> = new Set([
  "relevance",
  "modified",
  "created",
  "name",
  "size",
]);

const SCOPES: ReadonlySet<SearchScope> = new Set(["folder", "workspace", "all"]);

export interface UrlState {
  query: string;
  filters: SearchFilters;
  sort: SortBy;
  sortDir: SortDir;
}

/**
 * Serialize the current search state into a query string (no leading
 * `?`). Returns an empty string when nothing is set — the caller can
 * use that to decide between `replaceState(null, "", "/")` (clean URL)
 * and `replaceState(null, "", `?${qs}`)`.
 */
export function encodeSearchState(state: UrlState): string {
  const p = new URLSearchParams();
  const { query, filters, sort, sortDir } = state;

  if (query.trim().length > 0) p.set("q", query.trim());

  if (filters.scope !== "workspace") p.set("sc", filters.scope);
  if (filters.scope === "folder" && filters.folder_id) {
    p.set("fid", filters.folder_id);
  }
  if (filters.workspace_ids?.length) {
    p.set("ws", filters.workspace_ids.join(","));
  }
  if (filters.types.length) p.set("t", filters.types.join(","));
  if (filters.owner_ids.length) p.set("o", filters.owner_ids.join(","));
  if (filters.modified_after) p.set("ma", filters.modified_after);
  if (filters.modified_before) p.set("mb", filters.modified_before);
  if (filters.created_after) p.set("ca", filters.created_after);
  if (filters.created_before) p.set("cb", filters.created_before);
  if (filters.size_min !== undefined) p.set("sn", String(filters.size_min));
  if (filters.size_max !== undefined) p.set("sx", String(filters.size_max));
  if (filters.has_share_link === true) p.set("sl", "1");
  if (filters.include_trashed === true) p.set("it", "1");

  // Sort defaults are relevance / desc — omit when matching.
  if (sort !== "relevance") p.set("sort", sort);
  if (sortDir !== "desc") p.set("dir", sortDir);

  return p.toString();
}

/**
 * Parse a URL search string back into search state. Bad input is
 * coerced to defaults — no throws, no surprises. The complement of
 * `encodeSearchState`: encode → decode round-trips losslessly for any
 * state we can produce.
 */
export function decodeSearchState(search: string): UrlState {
  const p = new URLSearchParams(search);
  const filters = defaultFilters();

  const scope = p.get("sc");
  if (scope && SCOPES.has(scope as SearchScope)) {
    filters.scope = scope as SearchScope;
  }
  if (filters.scope === "folder") {
    const fid = p.get("fid");
    if (fid) filters.folder_id = fid;
  }
  const ws = p.get("ws");
  if (ws) {
    const ids = ws.split(",").map((s) => s.trim()).filter(Boolean);
    if (ids.length) filters.workspace_ids = ids;
  }
  const types = p.get("t");
  if (types) {
    filters.types = types
      .split(",")
      .map((s) => s.trim())
      .filter((s): s is TypeBucket => TYPE_BUCKETS.has(s as TypeBucket));
  }
  const owners = p.get("o");
  if (owners) {
    filters.owner_ids = owners.split(",").map((s) => s.trim()).filter(Boolean);
  }
  const ma = p.get("ma");
  if (ma) filters.modified_after = ma;
  const mb = p.get("mb");
  if (mb) filters.modified_before = mb;
  const ca = p.get("ca");
  if (ca) filters.created_after = ca;
  const cb = p.get("cb");
  if (cb) filters.created_before = cb;
  const sn = p.get("sn");
  if (sn && /^\d+$/.test(sn)) filters.size_min = Number(sn);
  const sx = p.get("sx");
  if (sx && /^\d+$/.test(sx)) filters.size_max = Number(sx);
  if (p.get("sl") === "1") filters.has_share_link = true;
  if (p.get("it") === "1") filters.include_trashed = true;

  const rawSort = p.get("sort");
  const sort: SortBy =
    rawSort && SORT_BYS.has(rawSort as SortBy) ? (rawSort as SortBy) : "relevance";
  const rawDir = p.get("dir");
  const sortDir: SortDir = rawDir === "asc" ? "asc" : "desc";

  return {
    query: p.get("q") ?? "",
    filters,
    sort,
    sortDir,
  };
}

/** True when this state would render any visible search UI — used to
 * decide whether to bother writing the URL at all. A purely-default
 * state writes nothing so the URL stays at `/`. */
export function isStateNonEmpty(state: UrlState): boolean {
  return (
    state.query.trim().length > 0 ||
    hasActiveFilters(state.filters) ||
    state.sort !== "relevance" ||
    state.sortDir !== "desc"
  );
}
