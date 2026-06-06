# 06 — Activity / audit-log surface

Companion to `02-surface-v2.md`. Defines the in-app `/activity` feed and the underlying `audit_log` event catalogue.

## Pattern reference

**GitHub / Linear / Vercel** all use a chronological, day-grouped timeline with one-line entries. We pick the same shape because:

1. *Scannability* — admins flick through the feed during incidents; one-line entries beat collapsed cards for that.
2. *Per-event extensibility* — each entry can carry a small badge + metadata blob without breaking the rhythm.
3. *Export-friendliness* — the wire shape maps 1:1 to a JSONL audit export (deferred to v0.2 but already informs the table schema).

## Layout

```
┌─ Activity (centered pane, 760 px max, scrolls vertically) ───────────────┐
│                                                                          │
│   # Activity                                                             │
│   Everything that happens in your Drive, newest first.                   │
│   ────────────────────────────────────────────────────                  │
│                                                                          │
│   Today                                                                  │
│   ────────                                                              │
│   • 14:32   sign-in       admin signed in              from 198.51.…   │
│   • 14:31   files.upload  admin uploaded Q2.xlsx       28.4 KB         │
│   • 14:29   share.create  admin shared Q2.xlsx         expires in 7 d  │
│   • 14:28   files.rename  admin renamed README.md       "README" → …   │
│                                                                          │
│   Yesterday                                                              │
│   ────────────                                                          │
│   • 18:11   share.access  someone opened Architecture.pdf  via Z3kQ… │
│   • 11:02   sign-in       admin signed in                              │
│                                                                          │
│   [ Load older ]                                                         │
└──────────────────────────────────────────────────────────────────────────┘
```

Notes:

- Day-group header is `Today` / `Yesterday` / `Last 7 days` / a relative date like `Wed, Jun 4`. Falls back to absolute `MMM D, YYYY` past 30 days.
- Each row is one line at default density: `[hh:mm]  [action-pill]  [sentence]  [metadata]`.
- Time renders in the **user's local timezone** (no UTC), matching the timezone-rule baked into `[[feedback_every_flow_tested_timezones]]`.
- Action pill uses category color: `auth.*` → ink, `files.*` → blue tint, `share.*` → gold tint, `system.*` → muted.
- Metadata column is grey, right-aligned at wider widths; collapses under the sentence on narrow.
- Row hover reveals a small ⋯ that exposes "Copy event JSON" for forensic use.
- Load-older button paginates against the cursor returned by `/api/activity`.

## Event catalogue (v0)

| Action | Actor | Target kind | Display sentence |
|---|---|---|---|
| `auth.sign_in` | user | session | *admin* signed in |
| `auth.sign_in_failed` | — | user | sign-in failed for *username* |
| `auth.sign_out` | user | session | *admin* signed out |
| `auth.password_changed` | user | user | *admin* changed their password |
| `setup.admin_created` | — | user | first-run setup completed — *admin* created |
| `files.upload` | user | file | *admin* uploaded *Q2.xlsx* |
| `files.rename` | user | file | *admin* renamed *Q2.xlsx* |
| `files.trash` | user | file | *admin* moved *Q2.xlsx* to trash |
| `files.restore` | user | file | *admin* restored *Q2.xlsx* |
| `files.download` | user | file | *admin* downloaded *Q2.xlsx* |
| `folders.create` | user | folder | *admin* created folder *Projects* |
| `folders.rename` | user | folder | *admin* renamed folder *Projects* |
| `share.create` | user | share_link | *admin* shared *Q2.xlsx* |
| `share.revoke` | user | share_link | *admin* revoked a share for *Q2.xlsx* |
| `share.access` | — | share_link | someone opened *Q2.xlsx* |

`share.access` deliberately has no actor (the recipient is anonymous). The metadata blob carries the share-link token.

## Backend contract

### `GET /api/activity?before={iso8601}&limit={n}` (authed)

```json
{
  "events": [
    {
      "id": "01HK…",
      "created_at": "2026-06-06T14:32:11Z",
      "actor_id": "usr_…",
      "actor_username": "admin",
      "action": "files.upload",
      "target_kind": "file",
      "target_id": "f_…",
      "target_name": "Q2.xlsx",
      "ip_address": null,
      "metadata": "{\"size\": 28400}"
    }
  ],
  "next_before": "2026-06-06T11:02:09Z"
}
```

- `limit` is server-clamped to `[1, 200]`, default 50.
- `next_before` is `null` when the page is short of `limit` (end of data).
- The endpoint is **authed only**. v0 returns the whole feed to every authed user; the per-user filter ships in v0.2 alongside RBAC.

## State checklist

| | Required | Notes |
|---|---|---|
| Default (≥1 event) | yes | day-grouped timeline |
| Empty (zero events) | yes | "Nothing here yet" + helper text |
| Loading | yes | 4 skeleton rows |
| Error | yes | inline `aria-live` band |
| Load older button | yes | spinner-on-click; hides when `next_before` is null |

## Out of scope (v0)

- Filter by event type / actor / date (10.5) — v0.2.
- JSONL export — v0.2.
- Tamper-evident chaining (hash-of-prev row) — v0.2. The table is append-only by convention today; cryptographic hardening comes with compliance work.
- Real-time push (SSE) — v0.2.
