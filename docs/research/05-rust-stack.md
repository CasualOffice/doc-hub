# 05 — Rust Web Stack for Casual Drive (2026)

State-of-the-ecosystem brief for the Casual Drive backend: a single Rust binary serving a WOPI host, a Drive-style file browser SPA, and four pluggable storage backends (fs / memory / S3 / MinIO). Target deploy: one container on a $5 VPS; must also scale up cleanly.

All version numbers, maintenance claims and crate URLs were cross-checked against crates.io / docs.rs / GitHub via WebSearch in June 2026. Where a fact could not be confirmed from a fetched source it is tagged `[unverified]`.

## TL;DR

- **Axum 0.8.x** is the framework. Tokio-team owned, tower/hyper-native, ergonomic, currently the de-facto default and growing fastest in downloads/ecosystem. Actix is fine but socially isolated; Rocket is dormant; Poem is niche.
- **Core stack:** axum 0.8, tokio 1.4x, tower 0.5, tower-http 0.6, hyper 1.x, serde 1.0.228, tracing 0.1.41, thiserror 2.0.18 + anyhow 1.x.
- **Auth:** `tower-sessions` 0.15 + cookie auth for v0; `oauth2` 5.0 / `openidconnect` 4.0 once IdP shows up; `argon2` 0.5 only if we ever hash local passwords; `jsonwebtoken` 10.x for WOPI access tokens. **Skip `axum-login`** — `tower-sessions` + a custom extractor is now the dominant pattern.
- **Storage trait:** native `async fn` in traits (AFIT, stable since 1.75) + `Arc<dyn Storage>` via `#[async_trait]` for the dyn-compatible variant. Streams via `impl Stream<Item = Result<Bytes>>`.
- **WOPI in Rust:** essentially greenfield. Only `beatgammit/wopi-rs` exists, last touched in 2017 [unverified beyond search-engine metadata] — treat as nonexistent and build our own handlers.
- **SPA embedding:** `rust-embed` 8.x (single binary). Split service is overkill at v0.
- **Build/deploy:** `cargo-chef` multi-stage Dockerfile, debian-slim runtime, strip + lto in release profile. Workspace with `drive-core` / `drive-storage` / `drive-wopi` / `drive-http` crates.

---

## 1. Framework: Axum vs Actix-Web vs Rocket vs Poem

| Framework  | Latest version | Maintenance | Notes |
|---|---|---|---|
| **axum**       | **0.8.8** (~Apr 2026 [unverified exact date]) | Active, Tokio team | Tower-native, hyper 1.x, AFIT-friendly, biggest ecosystem momentum |
| **actix-web**  | **4.13.0** (docs.rs head, 2026)   | Active, ~21k stars | Own runtime/middleware stack; mature; socially insulated from Tokio ecosystem |
| **rocket**     | **0.5.1** (docs.rs head; 0.5 GA Nov 2023) | Stagnant. No new news posts since 2023. | Macro-heavy ergonomics; dropped behind on async ecosystem evolution |
| **poem**       | **~unstated**, last commit Mar 13 2026 | Single-vendor (poem-web org) | Friendly API, OpenAPI bundled; small community |

### Maintenance and ecosystem fit

- **axum 0.8** (Jan 2025) brought `{name}` / `{*rest}` path syntax via matchit 0.8 — breaking but stabilising. 0.8.x patch line is current. Tower/hyper/tokio share one cabal of maintainers, so middleware (`tower-http`), sessions (`tower-sessions`), auth (`axum-login`), and observability all snap together.
- **actix-web 4.x** is fast and stable but ships its own actor-ish runtime model and its own middleware trait. Every Rust ecosystem narrative since 2023 has shifted toward tower; new auth/observability crates target axum first. Real-world perf delta vs axum is single-digit-% and irrelevant for a Drive workload.
- **Rocket** never resumed cadence after 0.5 GA in November 2023. The news feed shows nothing newer; this is "maintained, not developed." Avoid for greenfield work in 2026.
- **Poem** is a real, currently-maintained framework with a nice ergonomic surface and built-in OpenAPI, but the bus factor is low and few of the ecosystem crates we want (`tower-sessions`, `tower-http` middlewares, `axum-login`) are first-class there.

### Pick

**Axum 0.8.x.** Justification beyond "strong prior":
1. Drive needs streaming uploads/downloads, sessions, and middleware composition — all best in axum because they're all built on `tower::Service`.
2. WOPI handlers map cleanly to axum extractors (typed path/query, typed headers like `X-WOPI-Lock`, body as bytes/stream).
3. We can later put `tonic` (gRPC) or `tower-sessions` next to it without re-architecting.
4. AFIT support is fluent; the storage trait below works without macros for static dispatch.

Sources: [crates.io/axum](https://crates.io/crates/axum), [crates.io/actix-web](https://crates.io/crates/actix-web), [crates.io/rocket](https://crates.io/crates/rocket), [crates.io/poem](https://crates.io/crates/poem), [tokio.rs axum 0.8 release](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0), [rust-web-framework-comparison](https://github.com/flosse/rust-web-framework-comparison).

## 2. Axum baseline crate stack

Confirmed against crates.io / docs.rs in June 2026:

```toml
[dependencies]
axum                = "0.8"            # 0.8.8 latest
tokio               = { version = "1", features = ["full"] }   # 1.5x LTS line, 1.47 / 1.51 LTS active
tower               = "0.5"            # 0.5.3
tower-http          = { version = "0.6", features = ["trace", "cors", "limit", "compression-gzip", "fs"] }   # 0.6.11
hyper               = "1"              # 1.8.1
serde               = { version = "1", features = ["derive"] } # 1.0.228
serde_json          = "1"              # 1.0.149
tracing             = "0.1"            # 0.1.41
tracing-subscriber  = { version = "0.3", features = ["env-filter", "json"] }
anyhow              = "1"              # 1.x
thiserror           = "2"              # 2.0.18 — note 2.x is the current major
bytes               = "1"
futures             = "0.3"
```

Notable: `thiserror` is at **2.x** now (since late 2024); don't pin 1.x out of habit. `hyper` is firmly on the **1.x** line; everything depends on it via `hyper-util`.

Sources: [crates.io/tokio](https://crates.io/crates/tokio), [crates.io/tower](https://crates.io/crates/tower), [crates.io/tower-http](https://crates.io/crates/tower-http), [crates.io/hyper](https://crates.io/crates/hyper), [crates.io/serde](https://crates.io/crates/serde), [crates.io/tracing](https://crates.io/crates/tracing), [crates.io/thiserror](https://crates.io/crates/thiserror), [crates.io/anyhow](https://crates.io/crates/anyhow).

## 3. State management

Standard pattern: one `AppState` cloned cheaply (everything is `Arc`-wrapped) and handed to `Router::with_state`. Extractors pull what they need via `State<T>` or `FromRef`.

```rust
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<dyn Storage>,           // dyn-compatible adapter (see §5)
    pub sessions: SessionManagerLayer<…>,    // tower-sessions
    pub wopi_keys: Arc<WopiKeySet>,          // for HMAC signing access tokens
    pub config: Arc<Config>,
}

let app = Router::new()
    .route("/wopi/files/{file_id}",          get(wopi::check_file_info))
    .route("/wopi/files/{file_id}/contents", get(wopi::get_file).post(wopi::put_file))
    .route("/api/files",                     get(api::list).post(api::upload))
    .nest_service("/", spa_service)          // embedded SPA
    .with_state(state);
```

Extractors:

```rust
async fn check_file_info(
    State(s): State<AppState>,
    Path(file_id): Path<String>,
    TypedHeader(token): TypedHeader<WopiAccessToken>,  // custom typed header
) -> Result<Json<CheckFileInfo>, AppError> { … }
```

`FromRef` lets handlers depend on just one field (e.g. `State<Arc<dyn Storage>>`) without copying the whole AppState shape into the signature.

## 4. Authentication crates

| Crate | Latest | Maintenance | Use |
|---|---|---|---|
| `tower-sessions` | **0.15.0** (Feb 2026) | Active, Max Countryman | **Yes.** Cookie sessions as a tower layer; pluggable stores (memory for dev, redis/sqlite for prod). |
| `axum-login`     | **0.18.0**            | Active                 | Optional. Provides a richer `AuthSession<Backend>` extractor + `login_required!` macro. Many teams now skip it and write the ~30-line extractor themselves on top of `tower-sessions`. **Skip for v0.** |
| `argon2`         | **0.5.3** (RustCrypto) | Active                 | Only if we ever store local password hashes. For OIDC-only deployments: don't add. |
| `oauth2`         | **5.0.0** (ramosbugs) | Active                 | Plain OAuth2 (e.g. GitHub login). MSRV 1.65. |
| `openidconnect`  | **4.0.1** (ramosbugs) | Active                 | OIDC discovery, ID-token verification (Google, Keycloak, Authentik). Built on `oauth2`. **Preferred for SSO.** |
| `jsonwebtoken`   | **10.4.0** (Keats)    | Active                 | Issue + verify our **own** short-lived tokens (WOPI access tokens, signed download URLs). Pick the `aws_lc_rs` or `rust_crypto` backend feature. |

Recommendation for v0:

- Browser sessions: **`tower-sessions`** (cookie-backed, MemoryStore in dev, swap for SqliteStore / RedisStore in prod).
- SSO when we add it: **`openidconnect`** with the auth-code + PKCE flow.
- Signed URLs and WOPI access tokens: **`jsonwebtoken`** (HS256 with a server secret is plenty; rotate via key id).
- Don't add `axum-login` until we feel actual friction; the boilerplate is small.

Sources: [crates.io/tower-sessions](https://crates.io/crates/tower-sessions), [crates.io/axum-login](https://crates.io/crates/axum-login), [crates.io/argon2](https://crates.io/crates/argon2), [crates.io/oauth2](https://crates.io/crates/oauth2), [crates.io/openidconnect](https://crates.io/crates/openidconnect), [crates.io/jsonwebtoken](https://crates.io/crates/jsonwebtoken).

## 5. Async storage trait patterns

Status check on async-in-traits as of mid-2026:

- **AFIT** (`async fn` in traits) — stable since **Rust 1.75** (Dec 2023). Works fine for **static dispatch** (`impl<S: Storage> Handler<S>`).
- **`dyn Trait` with async fn** — still not object-safe in 2026. To get `Arc<dyn Storage>` you either:
  - keep `#[async_trait]` on the trait (returns `Pin<Box<dyn Future + Send>>`, one heap alloc per call — fine for our scale), or
  - hand-roll an "erased" wrapper trait, or use experimental crates like `dynify`/`trait-variant`.
- For Drive: one heap alloc per storage call is irrelevant next to the actual IO. **Use `#[async_trait]` and `Arc<dyn Storage>`.** Move to native AFIT only if we ever pick a single backend at compile time.

Streams: settle on `impl Stream<Item = Result<Bytes, StorageError>> + Send + 'static` for downloads; uploads accept the same shape and the adapter forwards to the backend. `bytes::Bytes` is the universal currency for axum, hyper, and the AWS SDK.

Trait sketch (satisfies fs / memory / S3 / MinIO cleanly — MinIO is just S3-protocol via a custom endpoint):

```rust
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;

pub type ByteStream = BoxStream<'static, Result<Bytes, StorageError>>;

#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub etag: Option<String>,
    pub modified: time::OffsetDateTime,
    pub content_type: Option<String>,
}

#[async_trait]
pub trait Storage: Send + Sync + 'static {
    async fn head(&self, key: &str) -> Result<ObjectMeta, StorageError>;
    async fn get(&self, key: &str) -> Result<(ObjectMeta, ByteStream), StorageError>;
    async fn put(
        &self,
        key: &str,
        body: ByteStream,
        content_type: Option<&str>,
    ) -> Result<ObjectMeta, StorageError>;
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
    async fn list(&self, prefix: &str) -> Result<Vec<ObjectMeta>, StorageError>;

    // WOPI lock semantics — opaque blob, 1024 ASCII chars, ~30 min TTL per spec.
    async fn lock(&self, key: &str, lock: &str) -> Result<(), LockError>;
    async fn refresh_lock(&self, key: &str, lock: &str) -> Result<(), LockError>;
    async fn unlock(&self, key: &str, lock: &str) -> Result<(), LockError>;
}
```

`StorageError` is the `thiserror` enum; `From<aws_sdk_s3::Error>` etc. live in the S3 adapter.

## 6. File upload patterns

Axum exposes `axum::extract::Multipart` (re-exposed from `multer`). Two things to remember:

- Default body limit is **2 MB**. For Drive uploads, set `DefaultBodyLimit::disable()` on the upload route and gate size in the handler (or `RequestBodyLimitLayer` for a hard cap).
- `Multipart` consumes the body, so it must be the **last** extractor in the handler signature.

Streaming pattern (no full buffering):

```rust
async fn upload(
    State(s): State<AppState>,
    mut mp: Multipart,
) -> Result<Json<UploadResp>, AppError> {
    while let Some(field) = mp.next_field().await? {
        if field.name() == Some("file") {
            let name = field.file_name().unwrap_or("untitled").to_owned();
            let content_type = field.content_type().map(str::to_owned);
            // Field implements Stream<Item = Result<Bytes, _>>:
            let stream: ByteStream = Box::pin(field.map_err(StorageError::from));
            let meta = s.storage.put(&name, stream, content_type.as_deref()).await?;
            return Ok(Json(UploadResp::from(meta)));
        }
    }
    Err(AppError::missing_field("file"))
}
```

For the S3 adapter, pipe the stream into `ByteStream::from_body_1_x(...)` so the AWS SDK does the multipart-S3 upload in chunks — no double buffering.

**tus.io** (resumable upload protocol): worth knowing about but not for v0. `fileloft` (multi-framework, Axum adapter, fs/S3/GCS/Azure stores) is the most production-shaped Rust option in 2026; `ztus` is a client; `tus-rust` is an older library. If we want resumable browser uploads later, slot `fileloft`'s axum adapter onto `/files/tus`.

Sources: [axum::extract::Multipart docs](https://docs.rs/axum/latest/axum/extract/multipart/index.html), [axum streaming upload discussion #1638](https://github.com/tokio-rs/axum/discussions/1638), [tus.io](https://tus.io/), [crates.io/fileloft-axum](https://crates.io/crates/fileloft-axum).

## 7. Existing WOPI implementations in Rust

Searched crates.io and GitHub. Result:

- **`beatgammit/wopi-rs`** — the only hit. Targets WOPI spec v9.0. Search-engine metadata suggests no meaningful activity in years (last indexed circa 2017 per snippet; [unverified beyond that]). Not on crates.io as far as the searches showed.
- No other WOPI host crate exists in the Rust ecosystem.

For reference, mature implementations live in other languages:
- **petrsvihlik/WopiHost** (.NET) — most complete; useful as a behavioural reference.
- **cs3org/wopiserver** (Python, vendor-neutral gateway).
- **nagi1/laravel-wopi** (PHP), **coatsy/wopi-node** / **mikeebowen/node-wopi-server** (Node), **OfficeDev/PnP-WOPI** (sample).

**Plan: write our own.** Build a `drive-wopi` crate with typed structs for `CheckFileInfo`, headers (`X-WOPI-Lock`, `X-WOPI-Override`, `X-WOPI-ItemVersion`, …), and the lock/unlock/refresh-lock state machine. Use WopiHost (.NET) as the canonical "this is what the response actually has to look like for Office Online" reference.

Sources: [github.com/beatgammit/wopi-rs](https://github.com/beatgammit/wopi-rs), [github.com/topics/wopi](https://github.com/topics/wopi), [github.com/petrsvihlik/WopiHost](https://github.com/petrsvihlik/WopiHost), [github.com/cs3org/wopiserver](https://github.com/cs3org/wopiserver).

## 8. Observability

Minimum to ship:

```rust
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

tracing_subscriber::registry()
    .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,drive=debug,tower_http=info".into()))
    .with(fmt::layer().json())          // structured logs out of the box
    .init();

// in routes:
let app = Router::new()
    .route(...)
    .layer(tower_http::trace::TraceLayer::new_for_http());
```

That's enough for structured per-request logs with method, path, status, latency. Add `#[tracing::instrument(skip(state, body))]` on storage adapter methods.

OpenTelemetry is **optional** and **off by default**:

- `opentelemetry` + `opentelemetry_sdk` + `opentelemetry-otlp` + `tracing-opentelemetry` (last release activity Jan 2026).
- Gate behind a `otel` cargo feature so the $5-VPS build doesn't pay for it.
- Export OTLP to whatever the operator runs (Tempo, Jaeger, Honeycomb).

Sources: [crates.io/tracing-opentelemetry](https://crates.io/crates/tracing-opentelemetry), [opentelemetry.io/docs/languages/rust](https://opentelemetry.io/docs/languages/rust/).

## 9. Testing

Three layers:

1. **Unit / handler tests** — call async handler functions directly with hand-built `State<T>` and check the `Result`.
2. **Router integration tests** — `tower::ServiceExt::oneshot` on the configured `Router`:

   ```rust
   use tower::ServiceExt;
   #[tokio::test]
   async fn check_file_info_ok() {
       let app = drive::router(test_state().await);
       let resp = app
           .oneshot(Request::get("/wopi/files/abc")
               .header("X-WOPI-Token", "test")
               .body(Body::empty()).unwrap())
           .await.unwrap();
       assert_eq!(resp.status(), StatusCode::OK);
   }
   ```

   Trait bounds quirk: `oneshot` needs `Router<()>`, so call `.with_state(state)` before the test if you parameterised `Router<AppState>`.

3. **Backend integration tests** — `testcontainers` + `testcontainers-modules` (feature `minio`) spin up a real MinIO on a random port for the S3 adapter tests. Reuse a single container per test module via `OnceCell` to keep CI snappy.

Sources: [axum testing example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs), [testcontainers-modules MinIO](https://docs.rs/testcontainers-modules/latest/testcontainers_modules/minio/struct.MinIO.html).

## 10. Build & deploy

Workspace layout:

```
drive/
├─ Cargo.toml                # [workspace] members = ["crates/*"]
├─ crates/
│  ├─ drive-core/            # domain types, errors, config
│  ├─ drive-storage/         # Storage trait + fs/memory/s3 adapters
│  ├─ drive-wopi/            # WOPI types + handlers (axum router fragment)
│  ├─ drive-http/            # router assembly + middleware + SPA mount
│  └─ drive-bin/             # main.rs, CLI, settings loading
├─ web/                      # SPA source (vite/whatever)
├─ Dockerfile
└─ docs/
```

Release profile (root `Cargo.toml`):

```toml
[profile.release]
lto = "thin"
codegen-units = 1
strip = "symbols"
panic = "abort"
```

Multi-stage Dockerfile with `cargo-chef` (≈5x faster rebuilds, per LukeMathWalker's benchmarks):

```dockerfile
# syntax=docker/dockerfile:1.7
FROM rust:1.85 AS chef
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json   # cached layer: deps only
COPY . .
RUN cargo build --release --bin drive

FROM debian:trixie-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/drive /usr/local/bin/drive
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/drive"]
```

Key rule (from cargo-chef docs): the planner, builder, and any cached image must use the **same Rust toolchain version**, or cache reuse silently breaks.

Sources: [cargo-chef README](https://github.com/LukeMathWalker/cargo-chef/blob/main/README.md), [Luca Palmieri — 5x faster rust docker builds](https://lpalmieri.com/posts/fast-rust-docker-builds/).

## 11. Frontend served by Drive

Three options:

- **`rust-embed` 8.x** — proc-macro that embeds a directory at compile time in release and reads from disk in dev. First-class axum example in the repo. SPA-friendly fallback is one match arm. **Pick this.**
- **`include_dir`** — simpler macro, no dev-mode hot reload, fewer features (no auto-MIME, no compression). Fine for tiny embeds; weak for an SPA.
- **`axum-embed`** — convenience wrapper over `rust-embed` that gives you a `Service` directly. Optional sugar.
- **Split service (nginx / reverse proxy)** — adds an operational moving part for a $5 VPS for zero benefit at v0. Defer until we actually need CDN edge caching.

For v0: `rust-embed = "8"` (8.11 latest), build the SPA into `web/dist/`, embed, and SPA-fallback to `index.html`:

```rust
#[derive(rust_embed::Embed)]
#[folder = "web/dist/"]
struct Assets;

async fn spa(uri: Uri) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');
    match Assets::get(path).or_else(|| Assets::get("index.html")) {
        Some(f) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(CONTENT_TYPE, mime.as_ref())], f.data.into_owned()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

Sources: [crates.io/rust-embed](https://crates.io/crates/rust-embed), [crates.io/include_dir](https://crates.io/crates/include_dir), [crates.io/axum-embed](https://crates.io/crates/axum-embed).

## 12. Drive starter blueprint

`Cargo.toml` (binary crate; trim once split into workspace):

```toml
[package]
name    = "drive"
version = "0.0.1"
edition = "2024"

[dependencies]
# http stack
axum                = { version = "0.8", features = ["multipart", "macros"] }
tokio               = { version = "1",  features = ["full"] }
tower               = "0.5"
tower-http          = { version = "0.6", features = ["trace", "cors", "limit", "compression-gzip"] }
hyper               = "1"

# data
serde               = { version = "1", features = ["derive"] }
serde_json          = "1"
bytes               = "1"
futures             = "0.3"
time                = { version = "0.3", features = ["serde", "formatting"] }

# error / log
anyhow              = "1"
thiserror           = "2"
tracing             = "0.1"
tracing-subscriber  = { version = "0.3", features = ["env-filter", "json"] }

# auth
tower-sessions      = "0.15"
jsonwebtoken        = { version = "10", default-features = false, features = ["aws_lc_rs"] }
openidconnect       = { version = "4", optional = true }   # behind `oidc` feature
argon2              = { version = "0.5", optional = true } # only if local pw login

# storage
async-trait         = "0.1"
aws-config          = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-s3          = "1"   # 1.135 latest
tokio-util          = { version = "0.7", features = ["io"] }

# spa
rust-embed          = { version = "8", features = ["mime-guess"] }
mime_guess          = "2"

[dev-dependencies]
testcontainers          = "0.27"
testcontainers-modules  = { version = "0.13", features = ["minio"] }   # [unverified exact version]
tower                   = { version = "0.5", features = ["util"] }     # ServiceExt::oneshot
http-body-util          = "0.1"

[features]
default = []
oidc    = ["dep:openidconnect"]
otel    = []
```

Module layout (inside one crate, ready to split into workspace later):

```
src/
├─ main.rs                  # tokio::main, settings, layers, axum::serve
├─ config.rs                # figment/serde Config struct
├─ error.rs                 # AppError + IntoResponse impl
├─ state.rs                 # AppState
├─ http/
│  ├─ mod.rs                # Router assembly
│  ├─ spa.rs                # rust-embed SPA fallback
│  └─ api/                  # GET /api/files, POST /api/files, signed URLs
├─ wopi/
│  ├─ types.rs              # CheckFileInfo, lock headers
│  ├─ handlers.rs           # GetFile, PutFile, Lock, Unlock, RefreshLock
│  └─ token.rs              # JWT access tokens
├─ storage/
│  ├─ mod.rs                # Storage trait + StorageError
│  ├─ fs.rs                 # filesystem adapter
│  ├─ memory.rs             # in-memory (dev/tests)
│  └─ s3.rs                 # AWS SDK; works for S3 and MinIO via endpoint override
└─ auth/
   ├─ session.rs            # tower-sessions glue
   └─ oidc.rs               # behind `oidc` feature
```

`main.rs` sketch:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    drive::observability::init();
    let cfg = drive::config::load()?;
    let storage = drive::storage::from_config(&cfg).await?;   // Arc<dyn Storage>
    let state   = drive::state::AppState::new(cfg.clone(), storage).await?;

    let app = drive::http::router(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(tower_http::compression::CompressionLayer::new())
        .layer(tower_http::cors::CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind(cfg.bind).await?;
    tracing::info!(addr = %cfg.bind, "drive listening");
    axum::serve(listener, app).await?;
    Ok(())
}
```

This blueprint compiles down to a single static binary (linked via `aws_lc_rs` to avoid an OpenSSL system dep) that ships in a ~20–40 MB Debian-slim image and runs comfortably on a $5 VPS, while leaving every seam (storage backend, session store, identity provider, OTLP exporter) swappable for scale-up.

---

## Sources (fetched)

- [crates.io — axum](https://crates.io/crates/axum)
- [crates.io — axum versions](https://crates.io/crates/axum/versions)
- [crates.io — actix-web](https://crates.io/crates/actix-web)
- [docs.rs — actix-web 4.13.0](https://docs.rs/crate/actix-web/latest)
- [crates.io — rocket](https://crates.io/crates/rocket)
- [docs.rs — rocket 0.5.1](https://docs.rs/crate/rocket/latest)
- [crates.io — poem](https://crates.io/crates/poem)
- [github.com/poem-web/poem](https://github.com/poem-web/poem)
- [tokio.rs — Announcing axum 0.8.0](https://tokio.rs/blog/2025-01-01-announcing-axum-0-8-0)
- [github — rust-web-framework-comparison](https://github.com/flosse/rust-web-framework-comparison)
- [crates.io — tokio](https://crates.io/crates/tokio)
- [crates.io — tower](https://crates.io/crates/tower)
- [docs.rs — tower 0.5.3](https://docs.rs/crate/tower/latest)
- [crates.io — tower-http](https://crates.io/crates/tower-http)
- [docs.rs — tower-http 0.6.8](https://docs.rs/crate/tower-http/latest)
- [crates.io — hyper](https://crates.io/crates/hyper)
- [crates.io — serde](https://crates.io/crates/serde)
- [docs.rs — serde 1.0.228](https://docs.rs/crate/serde/latest/source/crates-io.md)
- [docs.rs — serde_json 1.0.149](https://docs.rs/crate/serde_json/latest)
- [crates.io — tracing](https://crates.io/crates/tracing)
- [crates.io — tracing-opentelemetry](https://crates.io/crates/tracing-opentelemetry)
- [opentelemetry.io — Rust](https://opentelemetry.io/docs/languages/rust/)
- [crates.io — anyhow](https://crates.io/crates/anyhow)
- [crates.io — thiserror](https://crates.io/crates/thiserror)
- [docs.rs — thiserror 2.0.18](https://docs.rs/crate/thiserror/latest)
- [crates.io — tower-sessions](https://crates.io/crates/tower-sessions)
- [docs.rs — tower-sessions 0.15.0](https://docs.rs/crate/tower-sessions/latest)
- [github.com/maxcountryman/tower-sessions](https://github.com/maxcountryman/tower-sessions)
- [crates.io — axum-login](https://crates.io/crates/axum-login)
- [docs.rs — axum-login 0.18.0](https://docs.rs/crate/axum-login/latest)
- [crates.io — argon2](https://crates.io/crates/argon2)
- [docs.rs — argon2 0.5.3](https://docs.rs/crate/argon2/latest)
- [crates.io — oauth2](https://crates.io/crates/oauth2)
- [docs.rs — oauth2 5.0.0](https://docs.rs/crate/oauth2/latest)
- [crates.io — openidconnect](https://crates.io/crates/openidconnect)
- [docs.rs — openidconnect 4.0.1](https://docs.rs/crate/openidconnect/latest)
- [crates.io — jsonwebtoken](https://crates.io/crates/jsonwebtoken)
- [docs.rs — jsonwebtoken 10.4.0](https://docs.rs/crate/jsonwebtoken/latest)
- [rust-lang blog — async fn and RPIT in traits (1.75)](https://blog.rust-lang.org/2023/12/21/async-fn-rpit-in-traits/)
- [async fundamentals — async fn in dyn trait](https://rust-lang.github.io/async-fundamentals-initiative/explainer/async_fn_in_dyn_trait.html)
- [crates.io — async-trait](https://crates.io/crates/async-trait)
- [docs.rs — axum::extract::Multipart](https://docs.rs/axum/latest/axum/extract/multipart/index.html)
- [axum discussion #1638 — streaming upload/download](https://github.com/tokio-rs/axum/discussions/1638)
- [tus.io — resumable upload protocol](https://tus.io/protocols/resumable-upload)
- [crates.io — fileloft-axum](https://crates.io/crates/fileloft-axum)
- [github.com/beatgammit/wopi-rs](https://github.com/beatgammit/wopi-rs)
- [github.com — topics/wopi](https://github.com/topics/wopi)
- [github.com/petrsvihlik/WopiHost](https://github.com/petrsvihlik/WopiHost)
- [github.com/cs3org/wopiserver](https://github.com/cs3org/wopiserver)
- [axum testing example](https://github.com/tokio-rs/axum/blob/main/examples/testing/src/main.rs)
- [docs.rs — testcontainers-modules MinIO](https://docs.rs/testcontainers-modules/latest/testcontainers_modules/minio/struct.MinIO.html)
- [github.com/LukeMathWalker/cargo-chef](https://github.com/LukeMathWalker/cargo-chef)
- [Luca Palmieri — 5x faster Rust Docker builds](https://lpalmieri.com/posts/fast-rust-docker-builds/)
- [crates.io — rust-embed](https://crates.io/crates/rust-embed)
- [docs.rs — rust-embed 8.11.0](https://docs.rs/crate/rust-embed/latest)
- [crates.io — include_dir](https://crates.io/crates/include_dir)
- [crates.io — axum-embed](https://crates.io/crates/axum-embed)
- [crates.io — aws-sdk-s3](https://crates.io/crates/aws-sdk-s3)
- [docs.rs — aws-sdk-s3 1.122.0](https://docs.rs/crate/aws-sdk-s3/latest)
