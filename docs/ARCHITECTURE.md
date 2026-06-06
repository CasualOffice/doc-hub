# Architecture

How Casual Drive fits together: workspace, the storage adapter trait, the WOPI handoff, the two-origin security model, the three-token identity model. Distillation of [`research/00-synthesis.md`](./research/00-synthesis.md); the deeper rationale lives in the numbered research briefs alongside.

## At a glance

```
                   ┌─────────────────────────────┐
                   │  Browser — Drive SPA        │
                   │  (React + Radix + shadcn)   │
                   └─────────────┬───────────────┘
                                 │ HTTPS, cookies, JSON
                                 ▼
       ┌─────────────────────────────────────────────────────┐
       │  drive-bin  (single Rust binary, Axum)              │
       │  ┌─────────┬──────────┬──────────┬──────────┐       │
       │  │ http    │ wopi     │ auth     │ storage  │       │
       │  │ routes  │ host     │ session  │ facade   │       │
       │  └─────────┴──────────┴──────────┴────┬─────┘       │
       │                                       │              │
       │  app_origin            usercontent_origin            │
       │  drive.<host>          usercontent-drive.<host>      │
       └────────┬────────────────────┬─────────┬──────────────┘
                │                    │         │
       ┌────────▼─────────┐  ┌───────▼────┐  ┌─▼────────────────┐
       │  sheet/  (Node)  │  │  document/ │  │  OpenDAL backend │
       │  WOPI client     │  │  WOPI client│ │  fs|mem|S3|MinIO │
       │  port 3066 et al │  │  Go gateway │ │                  │
       └──────────────────┘  └────────────┘  └──────────────────┘
```

Single Rust binary in a Docker container. Two HTTP origins served from the same process (distinguished by `Host:` header at request time). Storage and identity are pluggable but the binary ships with sensible defaults for solo self-host.

## Workspace layout

```
drive/
├─ Cargo.toml              # [workspace] members = ["crates/*"]
├─ crates/
│  ├─ drive-core/          # domain types, errors, config, IDs
│  ├─ drive-storage/       # Storage facade over opendal::Operator
│  ├─ drive-wopi/          # WOPI types + host handlers (axum::Router fragment)
│  ├─ drive-auth/          # tower-sessions glue + share-link tokens
│  ├─ drive-http/          # router assembly, two-origin middleware, SPA mount
│  └─ drive-bin/           # main.rs, CLI, settings loading
├─ web/                    # Drive SPA (React + Vite)
├─ migrations/             # SQL (portable across SQLite + Postgres)
├─ docs/
└─ Dockerfile
```

**Why a workspace.** Lets `drive-wopi` be re-used by tests and the (future) `drive-wopi-validator` test harness without dragging in the whole HTTP layer. Lets `drive-storage` ship as a standalone library for anything else that wants the same OpenDAL facade later.

**Crate boundaries (rules).** `drive-core` depends on nothing in the workspace. `drive-storage`, `drive-wopi`, `drive-auth` depend on `drive-core` only. `drive-http` depends on all three. `drive-bin` depends on `drive-http`. No reverse edges.

## Storage facade

The single most important trait in the codebase. Wraps `opendal::Operator` + a small HMAC-token mint for filesystem/memory presigning.

```rust
// crates/drive-storage/src/lib.rs

use std::{ops::Range, sync::Arc, time::Duration};
use bytes::Bytes;
use futures::stream::BoxStream;
use thiserror::Error;

pub type ByteStream = BoxStream<'static, Result<Bytes, StorageError>>;

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub etag: Option<String>,
    pub modified: time::OffsetDateTime,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListPage {
    pub entries: Vec<ObjectMeta>,
    pub next_token: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SignedUrl {
    /// Backend issued a native presigned URL (S3/MinIO).
    Native { url: url::Url, expires_at: time::OffsetDateTime },
    /// We minted an HMAC token; serve via /raw/{token} on the user-content origin.
    Token { url: url::Url, expires_at: time::OffsetDateTime },
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("already exists: {0}")]
    AlreadyExists(String),
    #[error("invalid key: {0}")]
    InvalidKey(String),
    #[error("backend error")]
    Backend(#[from] opendal::Error),
    #[error("io error")]
    Io(#[from] std::io::Error),
}

#[derive(Clone)]
pub struct Storage {
    op: opendal::Operator,
    sign_key: Arc<[u8; 32]>,    // HMAC secret for self-minted tokens
    raw_base: Arc<url::Url>,    // base URL of /raw/{token} on user-content origin
}

impl Storage {
    pub fn from_env() -> anyhow::Result<Self> { /* DRIVE_BACKEND=fs|memory|s3|minio */ }

    pub async fn put_stream(&self, key: &str, body: ByteStream,
                            content_type: Option<&str>) -> Result<ObjectMeta, StorageError>;
    pub async fn get_stream(&self, key: &str, range: Option<Range<u64>>)
                            -> Result<(ObjectMeta, ByteStream), StorageError>;
    pub async fn stat(&self, key: &str)                  -> Result<ObjectMeta, StorageError>;
    pub async fn delete(&self, key: &str)                -> Result<(), StorageError>;
    pub async fn copy(&self, src: &str, dst: &str)       -> Result<(), StorageError>;
    pub async fn rename(&self, src: &str, dst: &str)     -> Result<(), StorageError>;
    pub async fn list(&self, prefix: &str, page_token: Option<&str>)
                            -> Result<ListPage, StorageError>;

    pub async fn signed_get(&self, key: &str, ttl: Duration) -> Result<SignedUrl, StorageError>;
    pub async fn signed_put(&self, key: &str, ttl: Duration) -> Result<SignedUrl, StorageError>;
}
```

**Capability gate.** `signed_get`/`signed_put` branch on `self.op.info().full_capability().presign_read`. S3/MinIO go down the `Native` path; fs/memory go down the `Token` path. Handlers always get a `SignedUrl` and never need to know.

**Path/key model.** OpenDAL's RFC-0112 normalisation: Unix-style, leading slash optional, `//` collapsed, no `..`, no `\`. Drive's `Key` is just `String` validated at the API boundary.

**The `/raw/{token}` route.** Mounted only on the user-content origin (see §"Two-origin model"). Decodes the HMAC token, verifies in constant time (`subtle::ConstantTimeEq`), checks expiry, then streams via `op.reader(key).into_futures_async_read(..)`. Same handler accepts uploads when method is `PUT`.

**Object-safety.** Trait dispatch uses `Arc<Storage>` directly (not `Arc<dyn StorageTrait>`). One concrete type, multiple OpenDAL services behind it. Keeps the call sites simple and lets us pass `Storage` by value cheaply (everything's an `Arc` internally).

**Conformance.** `tests/storage_conformance.rs` runs the same property suite across `Backend::Fs`, `Backend::Mem`, `Backend::Minio` (via `testcontainers-modules` `minio` feature). `Backend::S3` opt-in via `AWS_TEST_BUCKET`. Suite verifies put/get roundtrip, range read, list pagination, copy/rename atomicity, delete+absence, signed-URL variant selection, signed-URL round-trip.

## WOPI host + clients

Drive is the **WOPI host**. Sheet and Document are the **WOPI clients**. Sheet already ships ~500 LOC of WOPI-as-self-host scaffolding ([`research/01-wopi.md`](./research/01-wopi.md) §6) — the retrofit is `/hosting/discovery` + iframe entry route + lock loop, not a from-zero build. Document's `host.Integration` has an unwritten `wopi` impl slot already enumerated.

### Open-in-editor sequence

```
 Drive SPA (drive.<host>)         Drive backend         Editor (sheet.<host>)
 ───────────────────────         ─────────────         ─────────────────────
 user double-clicks row
        │
        │ GET /api/files/<id>/open
        ├──────────────────────────►│
        │                           │  ── mint per-launch access_token
        │                           │     (HMAC over {user_id, file_id, perms, exp, jti})
        │  { editor_app: "sheet",   │
        │    entry_url, token, ttl }│
        │◄──────────────────────────┤
        │
        │ window.open(entry_url
        │   + ?access_token=...&
        │     WOPISrc=https://drive.<host>/wopi/files/<id>)
        │ ─────────────────────────────────────────────────►│
        │                                                   │  ── sheet's entry route
        │                                                   │     dynamically creates iframe,
        │                                                   │     POSTs token into it
        │                                                   │     (defeats bfcache)
        │                                                   │
        │ Within iframe:                                    │
        │   GET /wopi/files/<id>?access_token=...           │  ─► Drive: CheckFileInfo
        │                                                   │     { BaseFileName, OwnerId, Size,
        │                                                   │       UserId, Version, UserCanWrite,
        │                                                   │       SupportsUpdate, SupportsLocks,
        │                                                   │       SupportsExtendedLockLength }
        │   GET /wopi/files/<id>/contents?access_token=...  │  ─► Drive: GetFile (stream bytes)
        │
        │   (sheet renders, user starts editing)
        │
        │   POST /wopi/files/<id>                           │
        │     X-WOPI-Override: LOCK                         │
        │     X-WOPI-Lock: <uuid v4>                        │  ─► Drive: Lock (30-min TTL)
        │
        │   Every ~10 min while editing:                    │
        │   POST /wopi/files/<id>                           │
        │     X-WOPI-Override: REFRESH_LOCK                 │
        │     X-WOPI-Lock: <uuid v4>                        │  ─► Drive: RefreshLock
        │
        │   On save (autosave or explicit):                 │
        │   POST /wopi/files/<id>/contents                  │
        │     X-WOPI-Override: PUT                          │
        │     X-WOPI-Lock: <uuid v4>                        │
        │     body: <xlsx bytes>                            │  ─► Drive: PutFile
        │                                                   │     → Storage::put_stream
        │                                                   │     → bump version, emit X-WOPI-ItemVersion
        │
        │   On beforeunload (navigator.sendBeacon):         │
        │   POST /wopi/files/<id>                           │
        │     X-WOPI-Override: UNLOCK                       │
        │     X-WOPI-Lock: <uuid v4>                        │  ─► Drive: Unlock
        │                                                   │
        │                                                   │  (lock released; row badge in Drive
        │                                                   │   clears within ~30 s on next list refresh)
```

### Endpoint contract

Drive (host) implements the seven endpoints from [`research/01-wopi.md`](./research/01-wopi.md) §1:

| Operation | Verb + path | Override | Required for edit |
|---|---|---|---|
| CheckFileInfo | `GET /wopi/files/{id}` | — | yes |
| GetFile | `GET /wopi/files/{id}/contents` | — | yes |
| PutFile | `POST /wopi/files/{id}/contents` | `PUT` | yes |
| Lock | `POST /wopi/files/{id}` | `LOCK` (no `X-WOPI-OldLock`) | yes |
| Unlock | `POST /wopi/files/{id}` | `UNLOCK` | yes |
| RefreshLock | `POST /wopi/files/{id}` | `REFRESH_LOCK` | yes |
| UnlockAndRelock | `POST /wopi/files/{id}` | `LOCK` + `X-WOPI-OldLock` present | yes |

`PutRelativeFile` (Save-As) deferred to a later phase; the route returns 501. `GetLock` deferred (we don't advertise `SupportsGetLock`).

### Lock storage

In Drive: a `wopi_locks` table keyed by `file_id` with columns `lock_id TEXT, acquired_at TIMESTAMPTZ, expires_at TIMESTAMPTZ`. One lock per file, 30-min TTL per spec, expiry checked on every Lock/Unlock/Refresh request. Stale locks (`expires_at < now()`) are treated as absent — the next Lock request succeeds and overwrites.

### Discovery (for sheet/document)

Sheet exposes `GET /hosting/discovery` returning the XML:

```xml
<wopi-discovery>
  <net-zone name="internal-https">
    <app name="Casual Sheets" favIconUrl="https://sheet.<host>/favicon.ico">
      <action name="edit" ext="xlsx" requires="locks,update"
        urlsrc="https://sheet.<host>/wopi/editor?<ui=UI_LLCC&><WOPI_SOURCE=WOPI_SOURCE&>"/>
      <action name="view" ext="xlsx"
        urlsrc="https://sheet.<host>/wopi/editor?readonly=1&<WOPI_SOURCE=WOPI_SOURCE&>"/>
      <!-- ods/csv/tsv: same shape -->
    </app>
  </net-zone>
  <proof-key value="" modulus="" exponent="" oldvalue="" oldmodulus="" oldexponent=""/>
</wopi-discovery>
```

Drive fetches and caches this for 12 h, re-fetches on proof-key validation failure (when MS365 federation is enabled — deferred for v0).

Document does the same at `/hosting/discovery` with `app name="Casual Editor"` advertising `docx`.

### Why no proof-key crypto in v0

Decided 2026-06-06: Drive doesn't federate to MS365 in v0. Only our own editors (sheet/document) are clients. Proof-key RSA validation is the defense against forged requests from Office's servers using a leaked token — irrelevant when the request comes from sheet's own server-side WOPI client. We still validate access tokens (HMAC), still enforce TTL, still check file-id scope. The proof-key hook is in the design; we just don't ship the RSA validation code until/unless federation goes on.

## Two-origin security model

Non-negotiable per [`research/06-security.md`](./research/06-security.md). Drive serves two HTTPS origins from the same binary, distinguished by the `Host:` header.

| Origin | Example | Serves | CSP |
|---|---|---|---|
| **App** | `drive.casualoffice.org` | SPA assets, JSON API, WOPI endpoints | `default-src 'self'; script-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'` (WOPI routes override `frame-ancestors` to allow sheet/document origins) |
| **User-content** | `usercontent-drive.casualoffice.org` | `/raw/{token}` — raw file bytes | `sandbox; default-src 'none'`. `Cross-Origin-Resource-Policy: same-site`. Forces opaque origin → in-document JS in uploaded HTML/SVG/PDF can't reach the app origin's cookies, can't navigate it, can't postMessage to it. |

Boot-time check: `assert!(app_origin != usercontent_origin)` in production builds. Dev defaults to `127.0.0.1:8080` + `127.0.0.1:8081`.

### Routing within the binary

```rust
// crates/drive-http/src/lib.rs
let app = axum::Router::new()
    .merge(app_origin_router(state.clone()))      // SPA + API + WOPI
    .merge(usercontent_router(state.clone()))     // /raw/{token}
    .layer(host_dispatch_layer(&cfg));            // rejects cross-origin requests
```

`host_dispatch_layer` reads `Host` and returns 421 (Misdirected Request) if the route doesn't match the expected origin for that host. Defence-in-depth against misconfigured reverse proxies.

### The signed-URL handoff

```
   Drive SPA              Drive backend (app)        Drive backend (usercontent)
   ─────────              ───────────────────        ───────────────────────────
   "Download" click
        │
        │ GET /api/files/<id>/download
        ├────────────────────────────►│
        │                             │  Storage::signed_get(key, 5min) →
        │                             │    fs/mem: SignedUrl::Token { url, exp }
        │                             │    S3/MinIO: SignedUrl::Native { url, exp }
        │  302 Location: <url>        │
        │◄────────────────────────────┤
        │                             │
        │ GET <url>  (usercontent origin OR S3 presign URL)
        │ ──────────────────────────────────────────►│
        │                                            │  /raw/{token}:
        │                                            │    HMAC-verify token in constant time
        │                                            │    Storage::get_stream(key)
        │                                            │    headers: Content-Type=sniffed,
        │                                            │             Content-Disposition=attachment;filename*=...
        │                                            │             X-Content-Type-Options=nosniff
        │                                            │             CSP: sandbox; default-src 'none'
        │  <bytes>                                   │
        │◄───────────────────────────────────────────┤
```

The browser ends up with a file from a different origin from the app it was using — so even if it executes (e.g. an uploaded HTML page), it can't reach app-origin state.

## Three-token identity model

Three concurrent token types, each with a distinct purpose. They never overlap.

| Token | Issued by | Carried in | TTL | Scope | Verifier |
|---|---|---|---|---|---|
| **Session cookie** | Drive `/sign-in` | `__Host-cd_sid` (HttpOnly Secure SameSite=Lax) | rolling 7 days | the one admin user | `tower-sessions` server-side store |
| **WOPI access token** | Drive `/api/files/<id>/open` | query `?access_token=` (and re-sent on every WOPI call) | 10 min, refreshed by editor via CheckFileInfo | (user_id, file_id, perms) | `jsonwebtoken` HMAC-SHA256 |
| **Share-link token** | Drive `/api/share-links` | path segment `/s/<token>` | unlimited or until expiry/revoke | a single share-link row | `subtle::ConstantTimeEq` against DB row |
| **Signed-URL token** (fs/mem only) | Drive `Storage::signed_get` | path/query on user-content origin | 5 min | (storage key, method) | HMAC-SHA256 + constant-time compare |

WOPI access tokens are minted *after* the requester is authenticated by one of the first two. A logged-in admin requesting `/api/files/<id>/open` gets a token. A share-link consumer reaching `/s/<token>/open` gets a token scoped to the share-link's perms (read-only, or read+write). The editor doesn't care which path produced the token.

## Data flow — typical edit session

```
1. Admin signs in (cookie set)
2. Admin double-clicks Budget Q2.xlsx
3. Drive mints WOPI token (10 min, write perms), returns sheet entry URL
4. Sheet opens in new tab, posts token into its WOPI iframe
5. Iframe → Drive WOPI: CheckFileInfo → GetFile → Lock → (user edits) → PutFile (every 30s on autosave) → RefreshLock (every 10min) → Unlock (on close)
6. Drive Storage::put_stream → OpenDAL Operator → backend (fs/S3/MinIO/memory)
7. Drive bumps version row, emits X-WOPI-ItemVersion on PutFile response
8. Admin returns to Drive tab; row "Modified" column refreshes on focus
```

## Error handling

- **Storage errors** (`StorageError`) → mapped to HTTP via `impl IntoResponse for AppError`. `NotFound → 404`, `AlreadyExists → 409`, `InvalidKey → 400`, anything else → `500` with redacted body.
- **WOPI errors** follow the spec contract: `400` missing lock header, `401` bad token, `404` not-found, `409` lock mismatch (with `X-WOPI-Lock: <current>` response header — mandatory + asymmetric per [`research/01-wopi.md`](./research/01-wopi.md) §4), `412` GetFile over `X-WOPI-MaxExpectedSize`, `413` PutFile over cap, `500` server error / proof-key failure.
- **Auth errors** → `401` for unauthenticated, `403` for authenticated-but-no-permission, `429` for rate-limited.
- **Validation errors** → `400` with a JSON body `{ "error": "...", "field": "..." }`.

## Observability

- `tracing` + `tracing-subscriber` JSON layer.
- Per-request `TraceLayer::new_for_http()` from `tower-http`.
- `#[tracing::instrument(skip(state, body))]` on storage adapter methods and WOPI handlers.
- Redaction: `Authorization`, `Cookie`, `X-WOPI-*`, `?access_token=` query — all stripped via `tower-http SetSensitiveHeadersLayer` + custom URL redactor at the trace layer.
- Body logging disabled by hard code path that refuses to start in prod when set.
- OpenTelemetry export gated behind cargo feature `otel`; off by default.

## Configuration

```bash
# Required
DRIVE_APP_ORIGIN=https://drive.casualoffice.org
DRIVE_USERCONTENT_ORIGIN=https://usercontent-drive.casualoffice.org
DRIVE_SESSION_SECRET=<32B base64>
DRIVE_WOPI_HMAC_SECRET=<32B base64>
DRIVE_SIGNED_URL_HMAC_SECRET=<32B base64>
DRIVE_ADMIN_USER=admin
DRIVE_ADMIN_PASSWORD_HASH=<argon2id $argon2id$...>

# Storage backend (one of)
DRIVE_BACKEND=fs           DRIVE_FS_ROOT=/var/lib/drive/data
DRIVE_BACKEND=memory       # tests only
DRIVE_BACKEND=s3           DRIVE_S3_BUCKET=...  DRIVE_S3_REGION=...
                           AWS_ACCESS_KEY_ID=...  AWS_SECRET_ACCESS_KEY=...
DRIVE_BACKEND=minio        DRIVE_S3_BUCKET=...  DRIVE_S3_ENDPOINT=http://minio:9000
                           AWS_ACCESS_KEY_ID=minioadmin  AWS_SECRET_ACCESS_KEY=minioadmin

# Database
DRIVE_DB_URL=sqlite:///var/lib/drive/drive.db   # or postgres://...

# Optional
DRIVE_BIND=0.0.0.0:8080
DRIVE_LOG_LEVEL=info,drive=debug
DRIVE_BODY_LIMIT_MB=100
DRIVE_ANTIVIRUS=clamd      DRIVE_CLAMD_SOCKET=/var/run/clamav/clamd.ctl
DRIVE_OIDC_ISSUER=...      # only when single-tenant mode → multi-user
DRIVE_RECIPIENT_FOOTER=true
```

Boot refuses to start in prod when:
- Either origin is missing or both match
- Any required secret is shorter than 32 bytes
- Any required secret equals a known dev default (`"changeme"`)
- `DRIVE_BACKEND=fs` and `DRIVE_FS_ROOT` doesn't exist or isn't `chmod 0700`
- `DRIVE_DB_URL` is unset

## Testing strategy

- **Unit tests** in each crate; standard `cargo test`.
- **Handler integration tests** in `drive-http`: build the `Router` with a `Backend::Mem` storage, `MemoryStore` sessions, hit it via `tower::ServiceExt::oneshot`.
- **Storage conformance suite** in `drive-storage/tests/`: same suite runs against `Fs`, `Mem`, `Minio` (via testcontainers).
- **WOPI conformance** in `drive-wopi/tests/`: use the proof-key fixtures from Microsoft's `Office-Online-Test-Tools-and-Documentation` repo as a unit test even though we don't validate proof in v0 (so we're ready when we do).
- **End-to-end** in `tests/e2e/` (Playwright): drives the SPA from sign-in through upload, open-in-editor (against a stub sheet/document or the real ones if available on `$DRIVE_E2E_SHEET_URL`), download, trash, restore.
- **WOPI cross-repo e2e** is a separate harness mounted in sheet/'s playwright.wopi.config.ts (which already exists at port 3066) — extended to point at this Drive binary instead of sheet-as-self-host.

## Build & deploy

- Cargo workspace, release profile: `lto = "thin", codegen-units = 1, strip = "symbols", panic = "abort"`.
- Multi-stage Dockerfile with `cargo-chef` for dep-layer caching.
- Runtime: `debian:trixie-slim` + `ca-certificates`. No OpenSSL system dep (link `aws-lc-rs`).
- SPA bundle (`web/dist/`) embedded via `rust-embed`. Single static binary, single .sqlite file, single Docker image.
- Reverse proxy example: Caddy (Caddyfile in `docs/deploy/Caddyfile.example`).
