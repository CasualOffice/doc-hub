//! CSRF defense for cookie-authenticated, state-changing requests.
//!
//! Audit finding: the app minted a per-session CSRF token and the SPA sent it in
//! `X-CSRF-Token`, but **nothing on the server ever verified it** — the
//! documented double-submit defense didn't exist, leaving cookie-auth mutations
//! reliant on `SameSite=Lax` alone.
//!
//! This layer adds an **Origin/Referer check** (the other defense named in
//! `docs/research/06-security.md` §11), which composes with `SameSite=Lax` for
//! defense-in-depth and — unlike a mandatory double-submit token — needs no
//! per-endpoint exemption list.
//!
//! Policy, applied only when a request could be a cross-site forgery:
//!   - Safe methods (GET/HEAD/OPTIONS/TRACE) never mutate → allowed.
//!   - Bearer-token (headless agent) requests carry no ambient cookie credential
//!     an attacker could ride → allowed.
//!   - Requests without a session cookie carry no ambient credential → allowed
//!     (the handler's own auth still rejects them).
//!   - For the rest (cookie-auth mutations): if an `Origin` (or, failing that,
//!     `Referer`) header is present, it MUST match the app origin, else 403. A
//!     browser always attaches `Origin` to a cross-site state-changing request,
//!     so a forgery is caught; a same-origin SPA call matches and passes. When
//!     neither header is present the request cannot be a browser CSRF (only a
//!     non-browser client omits both, and it wouldn't hold the victim's cookie),
//!     so it is allowed — which also keeps header-free API/test clients working.

use axum::{
    extract::{Request, State},
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::HttpState;

pub(crate) async fn guard(State(s): State<HttpState>, req: Request, next: Next) -> Response {
    let headers = req.headers();
    let exempt = is_safe(req.method()) || has_bearer(headers) || !has_session_cookie(headers);
    if exempt || origin_ok(headers, &s) {
        return next.run(req).await;
    }
    (
        StatusCode::FORBIDDEN,
        "cross-origin state-changing request refused",
    )
        .into_response()
}

fn is_safe(m: &Method) -> bool {
    matches!(
        *m,
        Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE
    )
}

fn has_bearer(h: &header::HeaderMap) -> bool {
    h.get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            let v = v.trim_start();
            v.len() > 7 && v[..7].eq_ignore_ascii_case("bearer ")
        })
}

/// True if the request carries a Doc-Hub session cookie (prod `__Host-` prefixed
/// or the dev name). We match by name only — validating the session is the
/// handler's job; here we just decide whether an ambient credential exists.
fn has_session_cookie(h: &header::HeaderMap) -> bool {
    let Some(cookie) = h.get(header::COOKIE).and_then(|v| v.to_str().ok()) else {
        return false;
    };
    cookie.split(';').any(|piece| {
        let name = piece.trim().split('=').next().unwrap_or("").trim();
        name == "__Host-dh_sid" || name == "dh_sid"
    })
}

/// Verify the request's `Origin` (or `Referer`) against the configured app
/// origin. Returns true when neither header is present (not a browser CSRF).
fn origin_ok(h: &header::HeaderMap, s: &HttpState) -> bool {
    let expected = s.config.app_origin.origin().ascii_serialization();

    if let Some(origin) = h.get(header::ORIGIN).and_then(|v| v.to_str().ok()) {
        return origin == expected;
    }
    if let Some(referer) = h.get(header::REFERER).and_then(|v| v.to_str().ok()) {
        // Compare the referer's origin, not a raw prefix (avoids
        // `https://app.evil.com` matching `https://app`).
        return url::Url::parse(referer)
            .is_ok_and(|u| u.origin().ascii_serialization() == expected);
    }
    // No Origin and no Referer — cannot be a cross-site browser request.
    true
}
