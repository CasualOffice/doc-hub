//! Security header constants per origin. See ARCHITECTURE.md §"Two-origin
//! security model". Layers are constructed inline in `lib.rs` for type
//! ergonomics (tower's Stack types are unwieldy as return types).

use axum::http::header::HeaderName;
use url::Url;

/// SHA-256 (base64) of the inline theme-bootstrap script in `web/index.html`.
///
/// That script runs pre-paint to set `data-theme` from `localStorage`, so the
/// page never flashes light→dark on load (see the `<script>` comment in
/// `web/index.html`). Under our strict CSP, a bare `script-src 'self'` blocks
/// *all* inline scripts — which silently killed the bootstrap in production
/// (dev has no CSP), bringing the flash back. Allow-listing exactly this one
/// script by hash keeps `script-src` strict (no `'unsafe-inline'`) while
/// letting the bootstrap run. `headers::tests::csp_hash_matches_index_html`
/// recomputes this from the actual file so an edit to the script that forgets
/// to update the hash fails CI instead of silently reintroducing the flash.
pub const THEME_BOOTSTRAP_SHA256: &str = "sha256-IZxsG6bsnjcOrK1Ca6RCFDuGerN2nf1d1k3fsKV24EA=";

/// Build the app-origin Content-Security-Policy.
///
/// `script-src` stays strict — same-origin bundles, the one hashed inline
/// theme bootstrap, and `'wasm-unsafe-eval'`. That last token lets the editor
/// SDKs instantiate their WebAssembly (the docx⇄markdown converter and the
/// spreadsheet engine), which browsers block under a bare `script-src`; it
/// permits WASM compilation only, NOT JavaScript `eval`, so the XSS-critical
/// defense (no `'unsafe-inline'` for scripts) is intact.
///
/// The remaining directives cover what the SPA legitimately loads — every one
/// of which a bare `default-src 'self'` was silently blocking in production
/// (dev serves no CSP, so these surface only once deployed):
///   - `style-src 'unsafe-inline'`: ~16 components render app-controlled
///     `<style>` elements (keyframes / dynamic geometry); none embed user HTML.
///     (`style={{…}}` attributes go through the CSSOM and need no allowance.)
///   - `style-src` / `font-src` fonts.googleapis/gstatic: the sheet editor's
///     Material Symbols icon font (the app's own Inter + mono are self-hosted).
///   - `img-src data: blob:`: client-generated thumbnails (`data:`) and
///     object-URL previews (`blob:`).
///   - `connect-src` + the collab origin: real-time co-editing opens a
///     (usually cross-origin) Yjs WebSocket at `DOCHUB_COLLAB_URL`
///     (e.g. `wss://collab.example.org`), which `'self'` alone blocks.
pub fn app_csp(collab_url: Option<&Url>) -> String {
    let mut connect_src = String::from("'self'");
    if let Some(origin) = collab_url.and_then(ws_origin) {
        connect_src.push(' ');
        connect_src.push_str(&origin);
    }
    format!(
        "default-src 'self'; \
         script-src 'self' '{THEME_BOOTSTRAP_SHA256}' 'wasm-unsafe-eval'; \
         style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; \
         font-src 'self' https://fonts.gstatic.com; \
         img-src 'self' data: blob:; \
         connect-src {connect_src}; \
         object-src 'none'; \
         base-uri 'none'; \
         frame-ancestors 'none'"
    )
}

/// The WebSocket origin (`wss://host[:port]`, no path) of the collab server,
/// for `connect-src`. Mirrors `collab::collab_ws_url`'s scheme swap but yields
/// just the origin. `None` if the URL has no host.
fn ws_origin(u: &Url) -> Option<String> {
    let scheme = match u.scheme() {
        "https" | "wss" => "wss",
        _ => "ws",
    };
    let host = u.host_str()?;
    Some(match u.port() {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    })
}

pub const UCN_CSP: &str = "sandbox; default-src 'none'";

pub const REFERRER_POLICY: &str = "strict-origin-when-cross-origin";
pub const PERMISSIONS_POLICY: &str = "camera=(), microphone=(), geolocation=(), interest-cohort=()";

/// HSTS for the app origin — two years, subdomains, preload-eligible (docs/
/// research/06-security.md §11). Emitted **only in production**: it pins HTTPS,
/// so sending it in a local http dev session would wedge the browser onto a
/// non-existent localhost TLS endpoint.
pub const HSTS: &str = "max-age=63072000; includeSubDomains; preload";

pub const H_CSP: HeaderName = HeaderName::from_static("content-security-policy");
pub const H_XCTO: HeaderName = HeaderName::from_static("x-content-type-options");
pub const H_REF: HeaderName = HeaderName::from_static("referrer-policy");
pub const H_PP: HeaderName = HeaderName::from_static("permissions-policy");
pub const H_CORP: HeaderName = HeaderName::from_static("cross-origin-resource-policy");
pub const H_COOP: HeaderName = HeaderName::from_static("cross-origin-opener-policy");
pub const H_HSTS: HeaderName = HeaderName::from_static("strict-transport-security");

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine as _;
    use sha2::{Digest, Sha256};

    /// The CSP hash must match the inline theme-bootstrap script that
    /// `web/index.html` actually ships. If the script is edited without
    /// updating [`THEME_BOOTSTRAP_SHA256`] (and the copy inside [`APP_CSP`]),
    /// the browser blocks the bootstrap and the light↔dark flash returns in
    /// production — silently, since dev serves no CSP. This test turns that
    /// silent regression into a CI failure.
    #[test]
    fn csp_hash_matches_index_html() {
        // headers.rs lives in crates/dochub-http/src; index.html is at web/.
        let index =
            std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../web/index.html"))
                .expect("read web/index.html");

        // The bootstrap is the first bare `<script>` (no attributes). The
        // browser hashes the exact text between the tags.
        let open = "<script>";
        let start = index.find(open).expect("inline <script> present") + open.len();
        let end = index[start..].find("</script>").expect("closing </script>") + start;
        let content = &index[start..end];

        let digest = Sha256::digest(content.as_bytes());
        let expected = format!(
            "sha256-{}",
            base64::engine::general_purpose::STANDARD.encode(digest)
        );

        assert_eq!(
            THEME_BOOTSTRAP_SHA256, expected,
            "web/index.html bootstrap changed — update THEME_BOOTSTRAP_SHA256 to {expected}"
        );
        assert!(
            app_csp(None).contains(THEME_BOOTSTRAP_SHA256),
            "app CSP script-src must allow the bootstrap hash"
        );
    }

    #[test]
    fn csp_permits_wasm_and_inline_styles_but_not_inline_scripts() {
        let csp = app_csp(None);
        // WASM engines (docx⇄md converter, sheets) need this; inline styles too.
        assert!(csp.contains("'wasm-unsafe-eval'"));
        assert!(csp.contains("style-src 'self' 'unsafe-inline'"));
        assert!(csp.contains("img-src 'self' data: blob:"));
        // script-src must NOT grant 'unsafe-inline' — the XSS-critical guard.
        let script_src = csp
            .split("; ")
            .find(|d| d.starts_with("script-src"))
            .expect("script-src directive present");
        assert!(
            !script_src.contains("'unsafe-inline'"),
            "script-src must stay strict: {script_src}"
        );
    }

    #[test]
    fn csp_allows_the_cross_origin_collab_websocket() {
        // No collab configured → connect-src is just self.
        assert!(app_csp(None).contains("connect-src 'self';"));

        // Cross-origin collab server → its wss origin joins connect-src so the
        // SPA can open the Yjs socket. https→wss, path dropped.
        let u = Url::parse("https://collab.example.org/yjs?x=1").unwrap();
        assert!(
            app_csp(Some(&u)).contains("connect-src 'self' wss://collab.example.org;"),
            "{}",
            app_csp(Some(&u))
        );

        // A custom port is preserved; plain http→ws.
        let u2 = Url::parse("http://localhost:1234").unwrap();
        assert!(app_csp(Some(&u2)).contains("connect-src 'self' ws://localhost:1234;"));
    }
}
