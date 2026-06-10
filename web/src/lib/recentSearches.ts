/**
 * SR11 — recent searches (per-user localStorage, max 10).
 *
 * Spec: docs/ux/12-search-surface.md §"Search history".
 * Storage key fixed to `cd-search-history-v1` per the spec.
 *
 * Each entry captures the query AND the filter set that was active at
 * commit-time — clicking a recent re-applies both, not just the query
 * text. Dedup is by (query + filter-fingerprint) so repeated runs of
 * the same search bubble to the top instead of stacking.
 *
 * Never sent to the server in v0.3. Phase 4 may surface team-level
 * "popular searches" but that's its own brief.
 */
import { defaultFilters, type SearchFilters } from "../api/client.ts";

const STORAGE_KEY = "cd-search-history-v1";
const MAX_ENTRIES = 10;

export interface RecentSearch {
  /** The query text the user typed. Trimmed; never empty. */
  query: string;
  /** Snapshot of the filter state at commit-time. Restoring re-applies
   * the chip row exactly as the user had it. */
  filters: SearchFilters;
  /** Epoch millis. Surfaced as "2m ago" in the dropdown. */
  ts: number;
}

/** Read the list (newest-first). Returns [] on parse error, SSR, or
 * Safari private mode. Filters with unknown shape are dropped. */
export function getRecent(): RecentSearch[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    const out: RecentSearch[] = [];
    for (const item of parsed) {
      if (!isRecentLike(item)) continue;
      out.push({
        query: String(item.query),
        filters: { ...defaultFilters(), ...item.filters },
        ts: typeof item.ts === "number" ? item.ts : 0,
      });
      if (out.length >= MAX_ENTRIES) break;
    }
    return out;
  } catch {
    return [];
  }
}

/** Prepend or move-to-top a recent entry. Dedup is by query +
 * filter fingerprint so re-running the same search promotes the
 * existing row instead of stacking. */
export function recordRecent(
  query: string,
  filters: SearchFilters,
  now: number = Date.now(),
): RecentSearch[] {
  const q = query.trim();
  if (q.length === 0) return getRecent();

  const fp = fingerprint(q, filters);
  const previous = getRecent();
  const deduped = previous.filter((r) => fingerprint(r.query, r.filters) !== fp);
  const next: RecentSearch[] = [{ query: q, filters, ts: now }, ...deduped].slice(
    0,
    MAX_ENTRIES,
  );

  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* private mode — accept the in-memory result for this session. */
  }
  return next;
}

export function clearRecent(): void {
  try {
    window.localStorage.removeItem(STORAGE_KEY);
  } catch {
    /* private mode — silent. */
  }
}

function fingerprint(query: string, filters: SearchFilters): string {
  // Stable serialization — sort keys so JSON output is deterministic.
  const f = {
    scope: filters.scope,
    folder_id: filters.folder_id ?? "",
    workspace_ids: [...(filters.workspace_ids ?? [])].sort(),
    types: [...filters.types].sort(),
    owner_ids: [...filters.owner_ids].sort(),
    modified_after: filters.modified_after ?? "",
    modified_before: filters.modified_before ?? "",
    created_after: filters.created_after ?? "",
    created_before: filters.created_before ?? "",
    size_min: filters.size_min ?? null,
    size_max: filters.size_max ?? null,
    has_share_link: filters.has_share_link ?? null,
    include_trashed: filters.include_trashed ?? false,
  };
  return `${query}${JSON.stringify(f)}`;
}

function isRecentLike(x: unknown): x is { query: string; filters?: SearchFilters; ts?: number } {
  if (typeof x !== "object" || x === null) return false;
  const r = x as { query?: unknown };
  return typeof r.query === "string" && r.query.length > 0;
}
