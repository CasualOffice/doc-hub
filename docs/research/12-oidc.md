# 12 — OIDC sign-in (Phase 3)

Pipeline §1.7. Sketch + decisions for the multi-tenant identity layer that lets Casual Drive accept sign-ins from an external Identity Provider (IdP). Phase 3 work — this brief is the design contract, not the implementation.

## Why now

The Phase 1/2 model (single admin, then admin-creates-users) is great for a one-person VPS but hits a wall the moment a team wants Drive. Adding OIDC unlocks:

1. **Bring-your-own SSO.** Okta / Entra / Google Workspace / Authentik / Authelia / Keycloak — pick one, Drive accepts the assertion.
2. **Account lifecycle.** Joiners get auto-provisioned; leavers lose access when the IdP says so. No more orphaned admin accounts after someone leaves the team.
3. **Audit alignment.** Sign-in events match the IdP's logs — security teams can correlate Drive activity with the broader SSO audit trail.
4. **Compliance.** SOC 2 / ISO 27001 conversations get a lot easier when Drive defers identity to a system the auditor already trusts.

## What this brief is NOT

- A timeline. Phase 3 starts when v0 has been dogfooded at scale.
- A commitment to support every IdP. v0.3 ships against Authentik + Google Workspace + Entra; everything else is reachable via standard OIDC discovery but unverified.
- A SCIM / Directory Sync implementation. That's a follow-up; v0.3 provisions on first sign-in and de-provisions via a session cleanup endpoint.

## Locked decisions

### Authorization Code Flow with PKCE — not implicit, not password, not device

- **PKCE (RFC 7636)** even though Drive is a confidential client (it can keep a secret). PKCE costs nothing extra and protects against authorization-code interception if the redirect ever leaks.
- **No implicit flow.** Deprecated by OAuth 2.1.
- **No resource-owner password flow.** Defeats the purpose of SSO.
- **No device flow.** Drive is a browser-first app; there's no terminal client to authorise.

### ID token validation in-process — no token introspection round-trip

- Validate the ID token's signature against the IdP's JWKS (cached, refreshed every hour or on `kid` mismatch).
- Verify `iss`, `aud`, `exp`, `iat`, `nonce` per OIDC core.
- Do NOT call the IdP's `/userinfo` endpoint unless the ID token is missing claims we need — adds a network round-trip on every sign-in.

### Sessions stay Drive-side — not IdP token forwarding

- After successful OIDC sign-in, Drive mints its own `__Host-cd_sid` cookie pointing at a `sessions` row, same as today.
- The IdP refresh token is **discarded** post-auth. Drive's own session TTL controls how long the user stays signed in. Re-auth round-trips to the IdP when Drive's session expires.
- Why: keeps the session model uniform between password and OIDC sign-ins. No "is this an IdP-backed session or a local one" branching in every middleware.

### `pkce-redirect` token, not `state` for CSRF

- `state` is for caller-supplied opaque data the client wants echoed.
- CSRF protection on the redirect is the PKCE verifier itself (the IdP only accepts the code from the holder of the original challenge).
- Drive still uses `state` to carry an internal nonce so we can match the callback to the in-flight sign-in attempt and reject replays — but the security-critical CSRF property is from PKCE, not from `state`.

### One IdP per Drive instance — not federation

- v0.3 supports exactly one configured OIDC provider per Drive deployment.
- Multi-IdP / "sign in with X or Y" federation is v0.4+. It's not hard to add (just a list of providers in `Config`), but the UX (which provider to pick? show all on the sign-in card?) is a separate research item.

## Locked-out decisions

- **SAML.** Not in scope. Modern IdPs all speak OIDC. Adding SAML would double the implementation surface for a vanishing share of deployments.
- **OAuth2-only (no OIDC).** OIDC is the ID-token-bearing superset. Without OIDC we can't get `sub` / `email` / `name` claims reliably.
- **Just-in-time admin elevation via group claim.** v0.3 maps a single configured group claim to `is_admin`. More elaborate RBAC (per-group workspace assignment, etc.) lands with §8.5 RBAC role tiers, not here.

## Schema (Phase 3 migration draft)

```sql
-- Per-IdP-subject anchor row. `provider_id` lets us distinguish two
-- users with the same email under different IdPs (rare but possible
-- in the multi-IdP v0.4 future).
ALTER TABLE users ADD COLUMN oidc_provider_id TEXT;
ALTER TABLE users ADD COLUMN oidc_subject TEXT;
ALTER TABLE users ADD COLUMN oidc_email_verified INTEGER NOT NULL DEFAULT 0;
CREATE UNIQUE INDEX users_oidc_subject_idx
  ON users(oidc_provider_id, oidc_subject)
  WHERE oidc_subject IS NOT NULL;

-- Optional table for short-lived sign-in flow state. Could also live
-- in-memory + lose state across restarts; SQL is simpler.
CREATE TABLE oidc_flow_state (
  state          TEXT PRIMARY KEY,
  pkce_verifier  TEXT NOT NULL,
  nonce          TEXT NOT NULL,
  created_at     TEXT NOT NULL,
  expires_at     TEXT NOT NULL
);
```

Existing local-password rows keep working; OIDC is additive. Users can have both auth methods on the same `users.id`.

## Endpoints

| Method | Path | Purpose |
|---|---|---|
| `GET`  | `/api/auth/oidc/login`       | Redirect to the IdP authorization endpoint with PKCE challenge + state + nonce. |
| `GET`  | `/api/auth/oidc/callback`    | Receive `code` + `state`. Exchange for tokens, validate ID token, find-or-create user, mint Drive session, redirect to `/`. |
| `GET`  | `/api/auth/oidc/metadata`    | Returns the configured IdP discovery URL + provider label, so the sign-in card knows whether to show "Sign in with X" button. Public — no session required. |
| `POST` | `/api/auth/oidc/revoke`      | Optional — calls the IdP's `revocation_endpoint` with the refresh token (if we ever start storing it). v0.3 ships this as a no-op since we discard the refresh token. |

The existing password sign-in routes (`POST /api/auth/sign-in`, `POST /api/setup/admin`) keep working — `Config` toggles whether the password path is gated behind a `DRIVE_ALLOW_PASSWORD_AUTH=true` flag (default `true` for migration safety; operators can flip to `false` once OIDC works).

## Config

```
DRIVE_OIDC_ISSUER=https://auth.example.com
DRIVE_OIDC_CLIENT_ID=casual-drive
DRIVE_OIDC_CLIENT_SECRET=<32+ bytes>
DRIVE_OIDC_REDIRECT_URL=https://drive.example.com/api/auth/oidc/callback
DRIVE_OIDC_SCOPES=openid email profile           # default
DRIVE_OIDC_ADMIN_GROUP=drive-admins              # optional; maps group → is_admin
DRIVE_OIDC_AUTO_CREATE_USERS=true                # default true
DRIVE_OIDC_PROVIDER_LABEL=Authentik              # shown on the sign-in card
DRIVE_ALLOW_PASSWORD_AUTH=true                   # set false once OIDC works
```

Discovery URL is derived from `<issuer>/.well-known/openid-configuration` — no need to also configure `authorization_endpoint`, `token_endpoint`, etc.

## Sign-in card UX (v0.3)

```
┌─ Sign in ────────────────────────────────────────┐
│                                                  │
│  [ ┌─ logo ─┐  Sign in with Authentik   →     ]  │
│                                                  │
│  ── or ──                                        │
│                                                  │
│  Username  [                                  ]  │
│  Password  [                                  ]  │
│  [ Sign in ]                                     │
│                                                  │
│  Forgot password?                                │
└──────────────────────────────────────────────────┘
```

- IdP button shows only when `oidc_metadata` returns a configured provider.
- "or" divider disappears when `DRIVE_ALLOW_PASSWORD_AUTH=false`.
- The IdP button is the **primary** action when configured (filled).

## Threat model

| Risk | Mitigation |
|---|---|
| **Stolen ID token replay** | `nonce` claim verified against `oidc_flow_state.nonce`. Flow state rows expire after 10 min. |
| **Authorization code interception** | PKCE verifier required. IdP only accepts the code from the original challenger. |
| **IdP key rotation** | JWKS cached with `kid`-aware lookup; refresh on `kid` miss + every hour. |
| **Open redirect via post-login redirect** | The post-login redirect URL is hardcoded to `/` server-side. We don't accept a `?return_to` parameter at all in v0.3 — too easy to leak into a phishing chain. |
| **Group-claim spoofing** | The admin-group mapping is a static string compare against the IdP-asserted `groups` claim. The claim is inside the signed ID token, so the user can't forge it client-side. |
| **Compromised client secret** | Stored as `DRIVE_OIDC_CLIENT_SECRET` env var — same model as our other secrets. PKCE means even a leaked secret + leaked authorization code still can't be exchanged without the verifier. |
| **De-provisioning lag** | The IdP knows the user is gone; Drive's `sessions` row keeps them in for up to `DRIVE_SESSION_TTL_HOURS`. Mitigation: short session TTL (4h default) for OIDC-backed sessions specifically; force re-auth more often than for local-password users. |

## Out of scope for v0.3

- **SCIM** (System for Cross-domain Identity Management). The complete user-lifecycle protocol. Lands with §8.3 invitations.
- **Multi-IdP federation.** v0.3 = one provider per deployment.
- **Per-workspace IdP.** Workspaces inherit the deployment-wide IdP.
- **Bring-your-own-claim-mapper.** v0.3 hardcodes the claim-name conventions (`sub`, `email`, `name`, `groups`). A configurable mapper expression language lands in v0.4 if it becomes a pain point.
- **Stepup auth.** OIDC's `acr_values` for "require MFA for this action" — relevant for the admin surface but defer until §11.x admin gets a security-sensitive lane.
- **WebAuthn / passkeys as a parallel local-auth path.** Useful but orthogonal to OIDC; covered separately in a future brief.

## Crate landscape

- **`openidconnect` 5.x** — the canonical Rust client. Solid, audited, but heavy (drags in `oauth2`, several `serde` dependencies). Worth it for not hand-rolling JWS verification.
- Don't roll our own: ID token signature verification is a security-critical loop that's caused several CVEs in the wild (alg=none, etc.).

## Test plan (Phase 3 contract)

- Round-trip against a containerised IdP (Authentik test image) in CI: login → callback → session → sign-out → re-login.
- ID token tamper detection: flip a single byte, reject.
- Nonce mismatch: rejected.
- State mismatch: rejected.
- Expired `oidc_flow_state` row: rejected with a fresh-flow redirect.
- Provider unreachable (timeout): redirect back to sign-in with a "couldn't reach your identity provider" banner.
- Local password sign-in still works alongside OIDC when both are configured.
- `DRIVE_ALLOW_PASSWORD_AUTH=false` hides the password form server-side (not just CSS).
