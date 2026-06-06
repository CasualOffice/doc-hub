# 04 — First-run admin-setup wizard

Companion to `02-surface-v2.md` + `03-settings-surface.md`. Closes the install gap: a freshly-deployed Drive with zero users in the DB shouldn't show a sign-in card the operator has no credentials for.

## When it triggers

- App boot calls `GET /api/setup/status` *before* `/api/me`.
- If the response is `{needs_setup: true}` (zero users in DB), render the wizard.
- Once the wizard finishes a successful `POST /api/setup/admin`, the backend
  also mints a session for the new admin in the same response — the SPA goes
  straight from wizard to shell, never via the sign-in card.
- `needs_setup` is a one-way switch: as soon as one user exists, it stays
  `false` forever. Re-runs are a fresh install, not a recovery flow.

## Layout

```
┌─ Setup wizard (centered card, max-width 460px) ──────────────────────┐
│                                                                       │
│                       [Cloud logo · 56px]                             │
│                                                                       │
│                   Welcome to Casual Drive                             │
│              Self-hosted Drive for the Casual Office suite.           │
│                                                                       │
│   Step ●——○——○                                                       │
│   1 / 3                                                               │
│                                                                       │
│   ┌─ Step body ───────────────────────────────────────────────────┐  │
│   │                                                               │  │
│   │  (per-step content — paragraph + form fields + CTA)           │  │
│   │                                                               │  │
│   └───────────────────────────────────────────────────────────────┘  │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

Three steps:

1. **Welcome** — one paragraph about what Casual Drive is + a "Get started" button. No form. Sets expectations that this is a one-time flow.
2. **Create admin** — username + password + confirm. Inline validation: username ≥ 3 chars, password ≥ 12 chars, confirm matches. Saves on submit → calls `POST /api/setup/admin` → on success the cookie is set and CSRF is stashed.
3. **Ready** — single line "Welcome, *username*. Taking you to your Drive…" + a 600 ms beat → routes into the shell. Mostly an acknowledgement state; never gates the user.

## Component / token reuse

- Card uses the same surface tokens as the SignIn card (`--card` bg, `--line` border, `--radius-xl` corner, `--shadow`).
- Title in Fraunces 24 px / 500. Helper text Hanken 14 px / `--muted`.
- Step indicator: 3 dots, active in `--ink`, completed in `--accent`, pending in `--line-strong`. 8 px each, 6 px gap, animated transition on advance.
- Inputs reuse the SignIn `<Input>` style (12 px padding, 12 px radius, focus ring `0 0 0 4px rgba(26,26,30,.04)`).
- Primary button: full-width, `--ink` fill, `--paper` text, 12 px radius — same as SignIn's "Sign in" button.

## State checklist per step

| | Welcome | Create admin | Ready |
|---|---|---|---|
| Default | static copy + CTA | empty form, primary disabled | spinner → toast → redirect |
| Loading | n/a | submit spinner inline | n/a |
| Error | n/a | aria-live band above the form | n/a (errors come back to Step 2) |
| Empty | n/a | n/a | n/a |
| Success | n/a | clear form, advance step | redirect to shell after 600 ms |

## Backend contract

### `GET /api/setup/status` (public)

```json
{ "needs_setup": true }
```

- 200 only. No auth required. Safe to call before sign-in.
- Returns `false` once at least one row exists in `users`.

### `POST /api/setup/admin` (public, gated by zero-users invariant)

Body:

```json
{ "username": "…", "password": "…" }
```

Responses:

- **204** + `Set-Cookie: __Host-cd_sid=…` and `{csrf_token: "…"}` in body — admin created, session minted.
- **409** if a user already exists (post-race or replay).
- **422** if username < 3 or password < 12 chars.

Race protection: count + insert wrapped in a transaction. The `UNIQUE(username)` constraint backs us up if two operators race.

## Security notes

- Wizard endpoints are mounted **on the app origin only** — never on user-content. Host-dispatch middleware enforces this.
- Wizard endpoints **bypass CSRF** because there is no session to bind a CSRF token to yet. The zero-users invariant is the only access control they get; once a user exists, both endpoints turn into 409s permanently.
- The newly-minted password is Argon2id-hashed via the same `drive_auth::hash_password` used by the regular sign-in flow — no separate path, no separate parameters.

## Out of scope

- Workspace name / branding (Phase 2 — extends the wizard with a Step 2.5).
- OIDC bootstrap (Phase 3 — gates this whole wizard behind a "use SSO" option).
- Storage backend picker (configured via env vars, not the UI — security-sensitive).
