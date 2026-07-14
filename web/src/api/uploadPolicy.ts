// Client-side mirror of the server's documents-only ingest allowlist. The
// server is the real gate (dochub_core::ingest::guard — extension allowlist
// AND magic-byte sniff, on every upload path); this helper only exists so the
// SPA can refuse an off-allowlist name with a clear toast before the round-trip.
//
// Keep in sync with `ALLOWED_EXTENSIONS` in crates/dochub-core/src/ingest.rs.
// `yml` is accepted as an alias of `yaml`. Note the client can only check the
// extension — the byte-level sniff (a .pdf that isn't a PDF, a .txt that isn't
// UTF-8) is enforced server-side and surfaced as a 415 after upload.

const ALLOWED = new Set<string>([
  "docx", "xlsx", "xlsm", "pptx", "pdf",
  "md", "txt", "csv", "json", "yaml", "yml",
]);

/**
 * Returns the offending extension (lowercase, no dot) when the filename is
 * NOT on the documents-only allowlist — including the empty string when the
 * name has no usable extension. Returns `null` when the name is allowed.
 *
 * Only inspects the LAST dotted extension — `report.tar.gz` → `gz` (rejected,
 * not a document), `budget.xlsx.exe` → `exe` (rejected). Mirrors the server.
 */
export function disallowedUploadExtension(filename: string): string | null {
  const lower = filename.toLowerCase();
  const dot = lower.lastIndexOf(".");
  if (dot === -1 || dot === lower.length - 1) return "";
  const ext = lower.slice(dot + 1);
  return ALLOWED.has(ext) ? null : ext;
}
