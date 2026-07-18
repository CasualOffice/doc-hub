# Doc-Hub

**Open-source, self-hosted document hub — an encrypted, tamper-evident registry for the documents your team can't afford to lose or leak.**

Documents go in; a permanent, hash-chained, content-searchable history comes out. Every save appends a new version — old versions are never overwritten or hard-deleted. Every file is encrypted at rest (AES-256-GCM, per-workspace keys) and served over a strict two-origin model. `.docx` · `.xlsx` · `.pdf` · `.md` · `.txt` · `.csv` · `.json` · `.yaml` only — the narrow scope is what lets it encrypt, index, and version everything.

A single Rust binary. SQLite by default, Postgres for production. Runs as a non-root user.

- **Source & docs:** https://github.com/CasualOffice/doc-hub
- **Architecture:** https://github.com/CasualOffice/doc-hub/blob/main/docs/ARCHITECTURE.md
- **License:** Apache-2.0

---

## Supported tags

| Tag | Meaning |
| --- | --- |
| `0.0.1` | Exact immutable version. |
| `0.0` | Latest patch on the `0.0` line. |
| `latest` | Latest published release. |
| `sha-<short>` | The exact commit the image was built from. |

Multi-arch: `linux/amd64` and `linux/arm64`.

## Quick start

```bash
docker run -d --name dochub \
  -p 8080:8080 \
  -v dochub-data:/data \
  -e DOCHUB_BIND=0.0.0.0:8080 \
  -e DOCHUB_APP_ORIGIN=https://hub.example.com \
  -e DOCHUB_USERCONTENT_ORIGIN=https://usercontent.example.com \
  -e DOCHUB_BACKEND=fs \
  -e DOCHUB_FS_ROOT=/data \
  -e DOCHUB_DB_URL=sqlite:///data/dochub.db \
  -e DOCHUB_MASTER_KEY="$(openssl rand -base64 32)" \
  -e DOCHUB_SESSION_SECRET="$(openssl rand -hex 32)" \
  -e DOCHUB_WOPI_HMAC_SECRET="$(openssl rand -hex 32)" \
  -e DOCHUB_SIGNED_URL_HMAC_SECRET="$(openssl rand -hex 32)" \
  -e DOCHUB_ADMIN_USER=admin \
  -e DOCHUB_ADMIN_PASSWORD_HASH='<argon2id hash — see below>' \
  casualoffice/dochub:latest
```

> Keep `DOCHUB_MASTER_KEY` safe and stable. It wraps every per-workspace data key; lose it and the encrypted documents are unrecoverable. Boot **refuses to start** without it — encryption at rest is not optional.

Then open `https://hub.example.com`, sign in as the admin, create a workspace, and upload a document.

### Generating the admin password hash

The admin password is supplied as an Argon2id hash, never in plaintext:

```bash
printf '%s' 'your-admin-password' | argon2 "$(openssl rand -hex 8)" -id -t 2 -m 14 -p 1 -e
```

Paste the resulting `$argon2id$...` string into `DOCHUB_ADMIN_PASSWORD_HASH`.

## Configuration

**Required:**

| Variable | Purpose |
| --- | --- |
| `DOCHUB_APP_ORIGIN` | Public URL of the app (SPA, JSON API, editor byte streams). |
| `DOCHUB_USERCONTENT_ORIGIN` | Separate origin serving share-link bytes. **Must differ** from the app origin in production. |
| `DOCHUB_MASTER_KEY` | Base64 32-byte key-encryption-key. Wraps every per-workspace DEK. No boot without it. |
| `DOCHUB_ADMIN_PASSWORD_HASH` | Argon2id hash of the initial admin password. |
| `DOCHUB_SESSION_SECRET` | ≥32-byte session-signing secret. |
| `DOCHUB_WOPI_HMAC_SECRET` | ≥32-byte HMAC secret for editor access tokens. |
| `DOCHUB_SIGNED_URL_HMAC_SECRET` | ≥32-byte HMAC secret for signed download URLs. |

**Common (with defaults):**

| Variable | Default | Purpose |
| --- | --- | --- |
| `DOCHUB_BIND` | `127.0.0.1:8080` | Listen address. Set `0.0.0.0:8080` in a container. |
| `DOCHUB_BACKEND` | `fs` | Storage backend: `fs`, `s3`, `minio`, `memory`. |
| `DOCHUB_FS_ROOT` | — | Root dir for the `fs` backend (required when `fs`). |
| `DOCHUB_DB_URL` | `sqlite::memory:` | `sqlite:///data/dochub.db` to persist, or a `postgres://…` URL. |
| `DOCHUB_ADMIN_USER` | `admin` | Initial admin username. |
| `DOCHUB_BODY_LIMIT_MB` | `100` | Max upload/request body size. |
| `DOCHUB_PROD` | `false` | Enables production invariants (distinct origins, no dev-default secrets, JSON logs). |

S3/MinIO backends additionally take `DOCHUB_S3_BUCKET`, `DOCHUB_S3_REGION`, `DOCHUB_S3_ENDPOINT`, and `AWS_ACCESS_KEY_ID` / `AWS_SECRET_ACCESS_KEY`. OIDC sign-in and the optional AI layer have their own `DOCHUB_OIDC_*` / `DOCHUB_AI_*` variables — see the repository docs.

## Volumes & ports

- **`/data`** — the SQLite database and (with the `fs` backend) the encrypted document blobs. Mount a named volume to persist. Owned by the non-root `dochub` user (uid 1000).
- **`8080`** — HTTP. Put a TLS-terminating reverse proxy in front in production.

## Health

- `GET /healthz` → `ok` (liveness).
- `GET /readyz` → JSON readiness with `db` and `storage` checks (use for readiness probes).

## Security notes

- **Encryption at rest is mandatory** — no config flag disables it; boot fails without a master key.
- **Two-origin isolation** — the app origin and the user-content origin must differ in production (boot refuses otherwise), so share-link bytes can never run in the app's security context.
- **Documents only** — every upload is checked against a MIME allowlist by both extension and magic-byte sniff; anything else is rejected.
- **Runs as non-root** (uid 1000).
- This is not zero-knowledge E2E: the server holds keys by design so it can index and reason over content. Encryption defends a stolen disk or database dump, not a compromised running server.
