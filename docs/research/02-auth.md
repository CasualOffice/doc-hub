# 02 — Authentication & Identity for Casual Drive

> Research brief. Casual Drive = Rust/Axum backend + web UI, hands files to sibling editors (Sheets, Editor) via WOPI. Must self-host on a $5 VPS and also scale to multi-user. Sibling editors today have **no auth** (anonymous share-links). Drive adding accounts is a real shift.

**Methodology note.** `WebFetch` was blocked in this run despite the user enabling it; all sources below were grounded via `WebSearch` snippets against official docs (Nextcloud, Seafile, Pydio, ownCloud, OWASP, Microsoft Learn, crates.io, GitHub). Where a snippet was ambiguous, the claim is tagged `[unverified]`.

---

## TL;DR

- The five comparable self-hostable Drives all converge on **server-side sessions for the web UI + OIDC for SSO**, not JWT-in-localStorage. oCIS is the outlier: OIDC is mandatory and it ships an embedded IdP (LibreGraph Connect).
- **WOPI's `access_token` is *not* user auth.** It is a per-file, per-session capability token the editor echoes back on every WOPI call; the Drive mints it after authorising the user. Microsoft recommends ~10 h TTL with `access_token_ttl` advertising expiry in ms-since-epoch.
- **OWASP 2024 baseline:** Argon2id at `m=19 MiB, t=2, p=1` minimum (or `m=47 MiB, t=1, p=1`); cookies `__Host-`, `HttpOnly; Secure; SameSite=Strict`; never put session tokens in `localStorage`.
- **Share-links done right:** 128-bit random token (not UUIDv4 — it's 122 bits), optional bcrypt/argon2-hashed password, expiry default, scope = view-only / edit / file-drop, server-side revocation row.
- **For a Rust/Axum stack the boring stack works:** `argon2`, `tower-sessions` + `axum-login`, `openidconnect` (+ optionally `axum-oidc`) when OIDC is wanted, `tower_governor` for rate limits.
- **Recommendation for v0: option (b) — single-tenant self-host.** One admin account, server-side session cookie, share-links for the anonymous-editor handoff. It defends `/admin` without inventing a multi-user permission model we'll regret. Migration to (c) is additive — no URL breakage if file IDs and share-link IDs are stable from day one.

---

## 1. Survey: how comparable self-hostable Drives do identity

### Nextcloud (PHP / Apache or FPM, MySQL/MariaDB/Postgres)

- **Auth methods:** built-in local users (PHP-hashed), **LDAP/AD** via the bundled `user_ldap` app, **OIDC** via `user_oidc` (can do provisioning or delegate to another backend), **SAML** via `user_saml` ("SSO & SAML" app). See docs: User auth OIDC, User auth LDAP. ([docs.nextcloud.com][nc-oidc], [docs.nextcloud.com][nc-ldap])
- **Sessions:** PHP server-side sessions, cookie-based; no JWT for the web UI.
- **Share-link model:** the gold standard. Per-link **password**, **expiration date** (admin can force a default and enforce a max), and per-folder **permissions**: *Read-only*, *Allow upload and editing*, *File-drop* (write-only), *Hide download*. Admins can enforce password policy globally. ([docs.nextcloud.com][nc-share])
- **Install footprint:** 128 MB RAM/PHP process minimum, 512 MB recommended; 64-bit OS+PHP; needs a webserver + DB. Comfortably runs on a $5 VPS at single-user scale but it's not lightweight. ([docs.nextcloud.com][nc-sys])
- **Lesson:** the share-link UI is the model to copy. Mixing "password" and "expiry" in the same dialog has historically had race-condition bugs (see issue 175968 [unverified]); apply both atomically server-side.

### Seafile (C/Python backend, MySQL/MariaDB)

- **Auth methods:** local accounts (django-style password hash) by default; **LDAP/AD** in CE and Pro; **OAuth** (`pro` and CE since 7.0); **Shibboleth/SAML** via Apache module; **Remote-User** header for fronting with anything else. Authentication backend is switchable. ([manual.seafile.com][sf-ldap-ce], [manual.seafile.com][sf-oauth], [manual.seafile.com][sf-shib])
- **Sessions:** Django session cookies; no first-class JWT for the browser UI.
- **Share-link model:** REST `POST /api/v2.1/share-links/` with `password`, `expire_days`, and `permissions` (`can_edit`, `can_download`). Three preset permission levels: *Preview and Download*, *Preview only*, *Edit on cloud and download*. Admins set `SHARE_LINK_PASSWORD_MIN_LENGTH`, `SHARE_LINK_PASSWORD_STRENGTH_LEVEL`, `SHARE_LINK_EXPIRE_DAYS_MIN/MAX`. ([plus.seafile.com][sf-api], [manual.seafile.com][sf-seahub])
- **Lesson:** policy knobs (min password length, max expiry) are admin-tunable and worth copying.

### Filerun (PHP / MySQL, commercial-but-self-hostable)

- **Auth methods:** local accounts + **LDAP** + **SAML 2.0 / OAuth / OpenID / WS-Federation via SimpleSAMLphp** (delegates the heavy lifting to a separate PHP IdP shim). ([docs.filerun.com][fr-auth], [docs.filerun.com][fr-saml])
- **Install:** PHP extensions (`mysqlnd`, `curl`, `zip`, `xml`, `mbstring`, `imagick`) plus the **ionCube Loader** (closed-source bytecode protection). ([docs.filerun.com][fr-php])
- **Lesson:** outsourcing federated auth to a sidecar (SimpleSAMLphp) keeps the core lean — same pattern as standing an external IdP in front of Casual Drive. We do **not** want ionCube-style closed-source dependencies.

### Pydio Cells (Go, MySQL/MariaDB required, optional MongoDB)

- **Auth methods:** Cells embeds an OIDC server (a fork of CoreOS Dex [unverified]); the browser auths against it and gets a **JWT** back. Enterprise edition can federate to external OIDC/SAML IdPs. The session API issues JWTs the JS client carries. ([docs.pydio.com][pyd-auth], [docs.pydio.com][pyd-idp])
- **Install footprint:** 4 GB RAM minimum, 8 GB recommended; MySQL/MariaDB required (5.7+/10.3+, *not* 8.0.22). Heavyweight — not a $5 VPS target. ([pydio.com][pyd-req])
- **Lesson:** "OIDC everywhere, even for our own UI" is architecturally clean but it's the reason Cells needs 4 GB. JWT-as-session means revocation requires either short TTL or a denylist; both add complexity.

### ownCloud Infinite Scale / oCIS (Go single binary)

- **Auth methods:** **OIDC is mandatory.** oCIS ships an embedded IdP (LibreGraph Connect / `lico`) on port 9130, backed by the IDM service. For real deployments the embedded IdP is meant to be replaced with Keycloak/Authelia/authentik. ([owncloud.dev][ocis-idp], [doc.owncloud.com][ocis-idp-doc])
- **Sessions:** OIDC tokens (proxy validates); single-binary Go process. ([github.com][ocis-readme])
- **Install:** single Go binary, Go 1.25+ to build; otherwise drop-in. ([github.com][ocis-readme])
- **Lesson:** "force OIDC, ship a built-in IdP for small installs" is a powerful design but it bakes a federated-auth dependency into every install. Too much for v0 of a Drive whose siblings have no accounts.

| Product | Lang | DB | Default web auth | OIDC | Share-link primitives |
|---|---|---|---|---|---|
| Nextcloud | PHP | MySQL/PG | Session cookie | Plugin | password, expiry, R/W/file-drop, hide-download |
| Seafile | C/Py | MySQL | Session cookie | OAuth/OIDC | password, expire_days, can_edit/can_download |
| Filerun | PHP | MySQL | Session cookie | via SimpleSAMLphp | password, expiry [unverified] |
| Pydio Cells | Go | MySQL+ | OIDC → JWT | Required | password, expiry, ACL-based |
| oCIS | Go | — | **OIDC required** | Embedded IdP | password, expiry, role |

---

## 2. The three v0 options for Casual Drive

**(a) Anonymous share-links only.** No accounts. Every file lives behind an unguessable URL. Matches sibling editors today.
- **Pros:** zero auth UI, zero state-shift for Sheets/Editor users, trivially deployable.
- **Cons:** no "my files" page, no per-user quota, no `/admin` protection, file enumeration risk if URLs leak (no second factor). Multi-user is impossible without re-keying every URL.

**(b) Single-tenant self-host (one admin per container).** One account (env-seeded), server-side session cookie protects the file listing and `/admin`. Files inside are user-owned (the one user). Share-links are still the public-handoff mechanism.
- **Pros:** smallest possible auth surface; defends the dashboard; matches the "personal $5 VPS" target; lets us write the session+cookie+CSRF code once.
- **Cons:** "users" is a list of length 1; multi-tenant comes later as a schema/UX expansion.

**(c) Multi-user accounts.** Real signup/login (or OIDC), per-user file ownership, ACLs, quotas.
- **Pros:** the actual long-term shape.
- **Cons:** the long pole — password reset, email, account recovery, OIDC plumbing, admin UI to manage users, RBAC. Substantial scope.

---

## 3. Industry-standard secure implementation, per layer

### Password hashing — Argon2id

OWASP Password Storage Cheat Sheet (current): use **Argon2id**. Recommended profiles:

- `m = 47 MiB, t = 1, p = 1` *or*
- `m = 19 MiB, t = 2, p = 1` (minimum). ([cheatsheetseries.owasp.org][owasp-pw], [github.com][owasp-pw-md])

Some practitioners cite RFC 9106's bumped values (`m = 64 MiB, t = 3, p = 1`); see the open OWASP issue tracking that update. ([github.com][owasp-pw-rfc])

**Rust:** [`argon2`][crate-argon2] (pure Rust, RustCrypto). Use `Argon2::default()` then tune `Params`; salt via `OsRng`; never roll your own.

### Sessions — cookies, not JWT in `localStorage`

OWASP Session Management Cheat Sheet is explicit:

> "Do not store authentication tokens, session IDs, JWTs, refresh tokens, or any credential in `localStorage` or `sessionStorage`. Instead, use `HttpOnly; Secure; SameSite=Strict` cookies (preferred) or a Backend-for-Frontend (BFF) pattern."
> Recommended canonical form: `Set-Cookie: __Host-SID=<token>; path=/; Secure; HttpOnly; SameSite=Strict`. ([cheatsheetseries.owasp.org][owasp-sess])

**Rust:** [`tower-sessions`][crate-tower-sess] (storage-pluggable) + [`axum-login`][crate-axum-login] for the `AuthSession`/`AuthnBackend` traits. axum-login uses tower-sessions under the hood; the `session_auth_hash` field is the documented way to auto-invalidate sessions on password change.

### OAuth / OIDC

- [`openidconnect`][crate-oidc] — the foundation crate, mirrors Go's `coreos/go-oidc` API surface.
- [`oauth2`][crate-oauth2] — lower-level OAuth 2.0 only.
- For Axum specifically, [`axum-oidc`][crate-axum-oidc] wraps `openidconnect` with extractors and a middleware layer. ([lib.rs][lib-axum-oidc])

When v0 doesn't need OIDC, **don't pull it in.** Keep the trait that backs `AuthnBackend` so it's a drop-in later.

### CSRF

OWASP CSRF Cheat Sheet:
- For stateful (cookie session) apps: **synchronizer-token pattern**.
- For stateless: **double-submit cookie**.
- **`SameSite=Strict` is defense in depth, not a substitute** — combine with a token. ([cheatsheetseries.owasp.org][owasp-csrf])

For an Axum SPA: `SameSite=Strict` on the session cookie + a CSRF token bound to the session, sent via custom header (`X-CSRF-Token`) on mutating requests. Reject requests missing the header for any non-`GET`/`HEAD` route.

### Rate limiting

[`tower_governor`][crate-gov] — GCRA (Generic Cell Rate Algorithm), keys by peer IP or custom extractor, plays with Axum/Tonic/Hyper. Emits `x-ratelimit-after`, `retry-after` headers. Latest is 0.8.0. ([crates.io][crate-gov])

Apply to: `/login`, `/share/<id>` (per-IP), `POST /share-links` (per-user), `/password-reset` (per-email + per-IP).

### Magic links / password reset

OWASP Forgot Password Cheat Sheet + Auth Cheat Sheet baseline:
- Token = CSPRNG, sufficiently long (treat as session-token-grade entropy).
- TTL **≤ 1 h** for reset; magic-login articles suggest 15–30 min typical, 5–10 min ideal. ([cheatsheetseries.owasp.org][owasp-auth], [supertokens.com][magic-link])
- **Single-use:** invalidate on consumption.
- Rate-limit reset endpoint like a login endpoint (per-email and per-IP).
- Always return the same response whether the email exists or not (no account enumeration).

---

## 4. WOPI auth model — what the access_token actually is

Critical separation:

> "The host uses [`access_token`] to determine whether the request is authorized." (CheckFileInfo, Microsoft Learn) ([learn.microsoft.com][wopi-cfi])

> "Access tokens should expire (become invalid) automatically after a period of time, and hosts can use the `access_token_ttl` property to specify when an access token expires. … hosts shouldn't revoke access tokens as a standard part of their operations." ([learn.microsoft.com][wopi-concepts])

> "`access_token_ttl` … is represented as the number of milliseconds since January 1, 1970 UTC … Microsoft recommends … 10 hours." ([learn.microsoft.com][wopi-concepts])

In practice for Casual Drive:

1. User opens `/files/<id>` in the Drive UI. The Drive validates their **session cookie**.
2. Drive constructs the editor launch URL with `WOPISrc=<our WOPI endpoint>&access_token=<mint>&access_token_ttl=<unix-ms+10h>`.
3. The mint is a **fresh, per-file, per-session capability** signed by the Drive (HMAC over `{file_id, user_id, perms, exp}` or a random row in a `wopi_tokens` table). It is **not** the user's session cookie and must not be reusable across files.
4. The editor sends `access_token` back on every WOPI call (`CheckFileInfo`, `GetFile`, `PutFile`). Drive validates it server-side, looks up `perms` (view-only blocks `PutFile`), and serves bytes.
5. Token is per-launch — close-and-reopen mints a new one. Revocation is by short TTL; only revoke mid-session if perms actually changed.

This decoupling is what lets share-link viewers (no user account) still open files in WOPI: the share-link consumer gets a WOPI token scoped `view-only` with a shorter TTL.

**Proof keys** (`X-WOPI-Proof` headers) are a *separate* defense: the editor signs requests with a key whose public half is in `/hosting/discovery`; the WOPI host verifies. ([collaboraonline.com][cool-sec], [learn.microsoft.com][wopi-proof])

---

## 5. Share-links done right

Cribbing from Nextcloud's share UI ([docs.nextcloud.com][nc-share]) and Seafile's API ([plus.seafile.com][sf-api]):

- **Token:** 128 random bits from a CSPRNG, base64url-encoded (22 chars). **Do not use UUIDv4** — it carries only 122 bits of entropy (6 are fixed for variant/version), which is below the NIST SP 800-90A bar. ([neilmadden.blog][nm-uuid])
- **Optional password:** stored Argon2id-hashed, same params as user passwords. Enforce a minimum length (Seafile defaults to admin-set, Nextcloud has a "force password" policy).
- **Expiry:** default ON with a sane default (7 d). Admin can enforce a maximum. Expiration evaluated server-side from a `expires_at TIMESTAMPTZ` column.
- **Permissions:** model as a small enum — `view`, `view_download`, `edit`, `file_drop` (matches Nextcloud's vocabulary). Anything edit-grade still mints a per-session WOPI token; view-only forbids `PutFile`.
- **Revocation:** a row in `share_links`; delete row → 404. Track `created_at`, `last_accessed_at`, `access_count` for the owner's "shared by me" UI.
- **No enumeration:** never disclose whether a token doesn't exist vs is wrong-password — return the same "Enter password" / 404 page either way.

---

## 6. Migration path between (a) → (b) → (c) without breaking URLs

The whole reason to think about this now is that a Drive's URLs **are** its contract. Get the IDs stable on day one.

- **Stable URL shape from v0:**
  - File handoff: `/files/<file_id>` (auth-gated when in mode b/c).
  - Public share: `/s/<share_token>` (never auth-gated; always 128-bit random).
  - WOPI bootstrap: `/wopi/files/<file_id>?access_token=...` (per-launch token, not a URL the user sees).
- **(a) → (b):** add a `users` table with exactly one seeded row, gate `/files/*` behind a session cookie. `/s/<token>` keeps working unchanged. Existing share-tokens are still valid because they're row-keyed, not user-keyed.
- **(b) → (c):** add a `user_id` FK on `files` and `share_links`. Backfill = "all rows → the single admin." Add signup/login routes. The OIDC trait was already there from §3, so flipping on `user_oidc` is a config change. Share-link URLs are unaffected.

The hard mistakes to avoid:
- Encoding the user in the file URL (`/u/sachin/files/123`) — locks you to (c)'s namespace forever.
- Reusing the WOPI `access_token` as a session token. Don't.
- Making share-tokens guessable now ("we'll regenerate them in v1") — you won't, and the old ones live forever in chat history.

---

## 7. Recommendation for v0

**Pick (b): single-tenant self-host.**

Why:
- **Defends `/admin` and the file list** — share-links-only (option a) means anyone who can `GET /` sees every file. Unacceptable past a demo.
- **No multi-user UX debt** — we don't need signup, password reset, account recovery, user-admin UI, RBAC, or OIDC for v0. We need *one* cookie-gated login form against one env-seeded Argon2id hash (`CASUAL_DRIVE_ADMIN_USER`, `CASUAL_DRIVE_ADMIN_PASSWORD_HASH`).
- **Preserves the anonymous-share UX** the sibling editors already have — `/s/<token>` works without a session, and the WOPI handoff stays decoupled.
- **Stays $5-VPS-shaped** — no external IdP, no Keycloak, no MySQL for an auth DB. SQLite is enough. Single Rust binary.
- **Migration to (c) is purely additive** per §6.

### Concrete v0 stack

| Concern | Choice |
|---|---|
| Password hash | `argon2` crate, `Params::new(19456, 2, 1, None)` (OWASP minimum) |
| Session store | `tower-sessions` + `axum-login`, SQLite-backed |
| Cookie | `__Host-cd_sid=...; Path=/; Secure; HttpOnly; SameSite=Strict` |
| CSRF | session-bound token; required `X-CSRF-Token` header on non-GET |
| Rate limit | `tower_governor` on `/login` (5/min/IP), `/s/*` (60/min/IP), `/wopi/*` (per-token) |
| WOPI tokens | HMAC-SHA256 over `{file_id, user_id|"share:<id>", perms, exp}`, 10 h TTL, key in env |
| Share-links | 128-bit token, optional Argon2id-hashed password, default 7 d expiry, perms enum |
| OIDC | **deferred**; keep the `AuthnBackend` trait so `openidconnect` slots in for v1 |
| Magic-link / reset | **deferred** to v1 — single admin uses env password rotation |

The single decision that makes this work: **WOPI access tokens are minted per launch from whatever identity opened the page** — a session user *or* a share-link consumer. Both flows produce the same token shape, so the editor never needs to know which one it is. That separation is what lets v0 stay tiny while v1 (multi-user) drops in cleanly.

---

## Sources

- Nextcloud — OIDC: <https://docs.nextcloud.com/server/stable/admin_manual/configuration_user/user_auth_oidc.html>
- Nextcloud — LDAP: <https://docs.nextcloud.com/server/stable/admin_manual/configuration_user/user_auth_ldap.html>
- Nextcloud — File sharing admin: <https://docs.nextcloud.com/server/23/admin_manual/configuration_files/file_sharing_configuration.html>
- Nextcloud — File sharing user manual: <https://docs.nextcloud.com/server/31/user_manual/en/files/sharing.html>
- Nextcloud — System requirements: <https://docs.nextcloud.com/server/stable/admin_manual/installation/system_requirements.html>
- Seafile — LDAP (CE): <https://manual.seafile.com/latest/config/ldap_in_ce/>
- Seafile — OAuth: <https://manual.seafile.com/11.0/deploy/oauth/>
- Seafile — Shibboleth: <https://manual.seafile.com/12.0/config/shibboleth_authentication/>
- Seafile — Share links API: <https://plus.seafile.com/published/web-api/v2.1/share-links.md>
- Seafile — seahub_settings: <https://haiwen.github.io/seafile-admin-docs/12.0/config/seahub_settings_py/>
- Filerun — Authentication integration: <https://docs.filerun.com/authentication_integration>
- Filerun — SimpleSAMLphp: <https://docs.filerun.com/simplesamlphp>
- Filerun — PHP requirements: <https://docs.filerun.com/php_configuration>
- Pydio — Authentication: <https://docs.pydio.com/latest/developer-guide/introduction/authentication/>
- Pydio — Cells as IdP: <https://docs.pydio.com/latest/admin-guide/connect-your-users/single-sign-on-features/cells-as-identity-provider/>
- Pydio — Cells requirements: <https://pydio.com/en/docs/cells/v4/requirements>
- oCIS — Project page: <https://owncloud.dev/ocis/>
- oCIS — IDP service: <https://owncloud.dev/services/idp/>
- oCIS — IDP service config: <https://doc.owncloud.com/ocis/next/deployment/services/s-list/idp.html>
- oCIS — README / build: <https://github.com/owncloud/ocis/blob/master/README.md>
- OWASP — Password Storage: <https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html>
- OWASP — Password Storage (md): <https://github.com/OWASP/CheatSheetSeries/blob/master/cheatsheets/Password_Storage_Cheat_Sheet.md>
- OWASP — Password Storage RFC 9106 issue: <https://github.com/OWASP/CheatSheetSeries/issues/1183>
- OWASP — Session Management: <https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html>
- OWASP — Authentication: <https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html>
- OWASP — Forgot Password: <https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html>
- OWASP — CSRF Prevention: <https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html>
- crates.io — `argon2`: <https://crates.io/crates/argon2>
- crates.io — `axum-login`: <https://crates.io/crates/axum-login>
- crates.io — `axum-oidc`: <https://crates.io/crates/axum-oidc>
- lib.rs — `axum-oidc`: <https://lib.rs/crates/axum-oidc>
- crates.io — `tower_governor`: <https://crates.io/crates/tower_governor>
- GitHub — `tower-governor`: <https://github.com/benwis/tower-governor>
- Microsoft Learn — WOPI CheckFileInfo: <https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/checkfileinfo>
- Microsoft Learn — WOPI key concepts: <https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/concepts>
- Microsoft Learn — WOPI proof keys: <https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/scenarios/proofkeys>
- Collabora — Security: <https://www.collaboraonline.com/security/>
- SuperTokens — Magic links: <https://supertokens.com/blog/magiclinks>
- Neil Madden — Moving away from UUIDs (entropy): <https://neilmadden.blog/2018/08/30/moving-away-from-uuids/>

<!-- link reference definitions used by shorthand citations above -->
[nc-oidc]: https://docs.nextcloud.com/server/stable/admin_manual/configuration_user/user_auth_oidc.html
[nc-ldap]: https://docs.nextcloud.com/server/stable/admin_manual/configuration_user/user_auth_ldap.html
[nc-share]: https://docs.nextcloud.com/server/23/admin_manual/configuration_files/file_sharing_configuration.html
[nc-sys]: https://docs.nextcloud.com/server/stable/admin_manual/installation/system_requirements.html
[sf-ldap-ce]: https://manual.seafile.com/latest/config/ldap_in_ce/
[sf-oauth]: https://manual.seafile.com/11.0/deploy/oauth/
[sf-shib]: https://manual.seafile.com/12.0/config/shibboleth_authentication/
[sf-api]: https://plus.seafile.com/published/web-api/v2.1/share-links.md
[sf-seahub]: https://haiwen.github.io/seafile-admin-docs/12.0/config/seahub_settings_py/
[fr-auth]: https://docs.filerun.com/authentication_integration
[fr-saml]: https://docs.filerun.com/simplesamlphp
[fr-php]: https://docs.filerun.com/php_configuration
[pyd-auth]: https://docs.pydio.com/latest/developer-guide/introduction/authentication/
[pyd-idp]: https://docs.pydio.com/latest/admin-guide/connect-your-users/single-sign-on-features/cells-as-identity-provider/
[pyd-req]: https://pydio.com/en/docs/cells/v4/requirements
[ocis-idp]: https://owncloud.dev/services/idp/
[ocis-idp-doc]: https://doc.owncloud.com/ocis/next/deployment/services/s-list/idp.html
[ocis-readme]: https://github.com/owncloud/ocis/blob/master/README.md
[owasp-pw]: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
[owasp-pw-md]: https://github.com/OWASP/CheatSheetSeries/blob/master/cheatsheets/Password_Storage_Cheat_Sheet.md
[owasp-pw-rfc]: https://github.com/OWASP/CheatSheetSeries/issues/1183
[owasp-sess]: https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html
[owasp-auth]: https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html
[owasp-csrf]: https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html
[crate-argon2]: https://crates.io/crates/argon2
[crate-tower-sess]: https://crates.io/crates/tower-sessions
[crate-axum-login]: https://crates.io/crates/axum-login
[crate-oidc]: https://crates.io/crates/openidconnect
[crate-oauth2]: https://crates.io/crates/oauth2
[crate-axum-oidc]: https://crates.io/crates/axum-oidc
[lib-axum-oidc]: https://lib.rs/crates/axum-oidc
[crate-gov]: https://crates.io/crates/tower_governor
[wopi-cfi]: https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/files/checkfileinfo
[wopi-concepts]: https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/concepts
[wopi-proof]: https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/online/scenarios/proofkeys
[cool-sec]: https://www.collaboraonline.com/security/
[magic-link]: https://supertokens.com/blog/magiclinks
[nm-uuid]: https://neilmadden.blog/2018/08/30/moving-away-from-uuids/
[plus.seafile.com]: https://plus.seafile.com/published/web-api/v2.1/share-links.md
[manual.seafile.com]: https://manual.seafile.com/latest/config/ldap_in_ce/
[docs.nextcloud.com]: https://docs.nextcloud.com/server/stable/admin_manual/configuration_user/user_auth_oidc.html
[docs.filerun.com]: https://docs.filerun.com/authentication_integration
[docs.pydio.com]: https://docs.pydio.com/latest/developer-guide/introduction/authentication/
[pydio.com]: https://pydio.com/en/docs/cells/v4/requirements
[owncloud.dev]: https://owncloud.dev/services/idp/
[doc.owncloud.com]: https://doc.owncloud.com/ocis/next/deployment/services/s-list/idp.html
[cheatsheetseries.owasp.org]: https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html
[github.com]: https://github.com/OWASP/CheatSheetSeries/blob/master/cheatsheets/Password_Storage_Cheat_Sheet.md
[supertokens.com]: https://supertokens.com/blog/magiclinks
[neilmadden.blog]: https://neilmadden.blog/2018/08/30/moving-away-from-uuids/
[collaboraonline.com]: https://www.collaboraonline.com/security/
[lib.rs]: https://lib.rs/crates/axum-oidc
[learn.microsoft.com]: https://learn.microsoft.com/en-us/microsoft-365/cloud-storage-partner-program/rest/concepts
[crates.io]: https://crates.io/crates/tower_governor
