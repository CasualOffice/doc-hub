// Client-side mirror of the server's upload blocklist. The server is the
// real gate (drive-http::files::check_upload_extension); this helper only
// exists so the SPA can refuse with a clear toast before the round-trip.
//
// Office macro-enabled formats (.docm/.xlsm/.pptm) are intentionally
// allowed per CLAUDE.md — opaque blobs, never auto-opened in editor.

const FORBIDDEN = new Set<string>([
  // Windows scripts / executables
  "exe", "com", "scr", "bat", "cmd", "msi", "msp",
  "ps1", "psm1", "vbs", "vbe", "wsf", "wsh", "jse",
  "reg", "lnk", "scf",
  // POSIX shells / runnable bundles
  "sh", "bash", "zsh", "fish", "csh", "ksh", "command",
  "app", "dmg", "pkg",
  // Runtime artefacts
  "jar", "class", "dll", "so", "dylib",
  // Shortcut-style files
  "url", "desktop",
]);

/**
 * Returns the offending extension (lowercase, no dot) when the filename
 * is in the blocklist; returns `null` otherwise.
 *
 * Only inspects the LAST dotted extension — `setup.tar.gz.exe` is blocked
 * because of `exe`, but `setup.tar.gz` slips through (matches the server).
 */
export function forbiddenUploadExtension(filename: string): string | null {
  const lower = filename.toLowerCase();
  const dot = lower.lastIndexOf(".");
  if (dot === -1 || dot === lower.length - 1) return null;
  const ext = lower.slice(dot + 1);
  return FORBIDDEN.has(ext) ? ext : null;
}
