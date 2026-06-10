/**
 * SR15 — keystroke → paint latency instrumentation.
 *
 * Spec: docs/ux/12-search-surface.md §"Performance budget" — p95
 * keystroke → paint < 200 ms.
 *
 * How it works:
 *   - TopBar calls `markKeystroke()` on every input change.
 *   - The FIRST keystroke after a paint opens a measurement window;
 *     subsequent keystrokes inside that window are ignored (debounce
 *     + fetch are part of the perceived wait, not separate events).
 *   - Files.tsx calls `markPaint()` in a double-rAF after the search
 *     effect setStates the result pane, so the timestamp reflects an
 *     actually-painted frame, not just a React commit.
 *
 * Storage:
 *   - Each pair emits `performance.mark`s + a `performance.measure`,
 *     so the DevTools Performance panel picks it up automatically.
 *   - PerformanceObserver aggregates measures into an in-memory
 *     rolling buffer (last 100 samples). `getStats()` returns
 *     p50/p95/max — surfaced on `window.__cd_search_perf` so
 *     Playwright can read it without depending on the DevTools
 *     protocol.
 *
 * Why not just instrument from the debounce fire:
 *   - The user is *waiting* from the keystroke, not from the
 *     debounce. The spec budget is the user-perceived latency. The
 *     200 ms debounce is part of that latency, by design.
 */

const MEASURE_NAME = "cd-search-keystroke-to-paint";
const KEYSTROKE_PREFIX = "cd-search-keystroke-";
const PAINT_PREFIX = "cd-search-paint-";
const MAX_SAMPLES = 100;

let seqCounter = 0;
let pendingSeq: number | null = null;
const samples: number[] = [];

/** Open a measurement window if none is open. No-op when called
 * during an already-pending search (the user is still typing). */
export function markKeystroke(): void {
  if (typeof performance === "undefined") return;
  if (pendingSeq !== null) return;
  seqCounter += 1;
  pendingSeq = seqCounter;
  try {
    performance.mark(`${KEYSTROKE_PREFIX}${pendingSeq}`);
  } catch {
    /* mark name collision is harmless; reset and bail */
    pendingSeq = null;
  }
}

/** Close the active measurement window and record the duration.
 * No-op when no window is open (e.g. an effect re-fired with no
 * preceding keystroke — workspace switch, filter chip click handled
 * elsewhere). */
export function markPaint(): void {
  if (typeof performance === "undefined") return;
  if (pendingSeq === null) return;
  const seq = pendingSeq;
  pendingSeq = null;
  try {
    performance.mark(`${PAINT_PREFIX}${seq}`);
    performance.measure(
      MEASURE_NAME,
      `${KEYSTROKE_PREFIX}${seq}`,
      `${PAINT_PREFIX}${seq}`,
    );
  } catch {
    /* a missing start mark just drops this sample — silent */
  }
}

/** Reset state, including the rolling buffer. Useful in tests; not
 * called by the SPA. */
export function _resetForTests(): void {
  seqCounter = 0;
  pendingSeq = null;
  samples.length = 0;
  try {
    performance.clearMarks();
    performance.clearMeasures(MEASURE_NAME);
  } catch {
    /* not available in some test runtimes */
  }
}

export interface PerfStats {
  count: number;
  p50_ms: number;
  p95_ms: number;
  max_ms: number;
}

/** Returns null when the buffer is empty so callers can distinguish
 * "no data yet" from "everything happened in 0 ms". */
export function getStats(): PerfStats | null {
  if (samples.length === 0) return null;
  const sorted = [...samples].sort((a, b) => a - b);
  const p = (q: number) => sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * q))];
  return {
    count: sorted.length,
    p50_ms: round1(p(0.5)),
    p95_ms: round1(p(0.95)),
    max_ms: round1(sorted[sorted.length - 1]),
  };
}

function round1(n: number): number {
  return Math.round(n * 10) / 10;
}

// ── Observer + window exposure ──────────────────────────────────────
// We register exactly once at module load; the SPA's a single page so
// the observer outlives any feature toggling.

if (typeof PerformanceObserver !== "undefined") {
  try {
    const obs = new PerformanceObserver((list) => {
      for (const entry of list.getEntriesByName(MEASURE_NAME)) {
        samples.push(entry.duration);
        if (samples.length > MAX_SAMPLES) samples.shift();
      }
    });
    obs.observe({ type: "measure", buffered: true });
  } catch {
    /* older browsers — feature is opt-in via stats access, no need
     * to crash boot. */
  }
}

if (typeof window !== "undefined") {
  // Stable global so Playwright + manual DevTools probes can read the
  // current stats without reaching into the module graph.
  (window as unknown as { __cd_search_perf?: () => PerfStats | null }).__cd_search_perf =
    getStats;
}
