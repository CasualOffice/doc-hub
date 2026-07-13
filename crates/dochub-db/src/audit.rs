//! Append-only, hash-chained audit log (build spec §6). Powers the
//! `/activity` feed and (later) the compliance JSON export.
//!
//! Writes are fire-and-forget from the handler's perspective — callers
//! `tokio::spawn` the `insert` so the request returns without waiting on
//! the DB. The append-only invariant lives in the schema (no UPDATE,
//! no DELETE statements anywhere); we don't enforce it with a trigger
//! to keep migrations portable across SQLite + Postgres.
//!
//! # Tamper-evidence (§6)
//!
//! Every appended row joins a single **global** hash chain (scope decision D1;
//! `audit_log` has no `workspace_id`, so a global chain is the simplest correct
//! option — per-workspace chains are a later refinement). Each row records the
//! previous chained row's `entry_hash` in `prev_hash` (NULL at the head) and its
//! own `entry_hash` over a canonical serialization of its stable fields:
//!
//! ```text
//! entry_hash = dochub_crypto::entry_hash(prev, canonical)
//!            = SHA-256( prev_hex ‖ 0x00 ‖ canonical )
//! ```
//!
//! where `canonical` is [`canonical`]'s length-prefixed encoding of, in fixed
//! order, `(id, created_at, actor_id, action, target_kind, target_id,
//! ip_address, metadata)`. [`AuditRepo::verify_audit_chain`] recomputes the
//! whole chain and reports the first break; committed rows are never
//! `UPDATE`/`DELETE`d.
//!
//! ## Single-writer assumption (Phase 0)
//!
//! Appends read the current chain head and insert the successor in one
//! transaction. Phase 0 assumes a single audit writer: SQLite is capped at one
//! connection (`pool.rs`), so appends serialize there; a concurrent second
//! writer under Postgres could read the same head and fork the chain. Verified
//! single-writer serialization (advisory lock / dedicated writer task) is a
//! Phase 1 hardening — the read-head-then-append transaction is the seam.

use serde::{Deserialize, Serialize};
use sqlx::Row;

use dochub_crypto::{entry_hash, Sha256Hex};

use crate::{
    users::{parse_ts, ts},
    Db, DbError,
};

/// The audit action vocabulary. Actions are dotted, namespaced strings
/// (`"auth.sign_in"`, `"files.upload"`, …); most are still written as string
/// literals at their handler call sites. The Phase-0 registry / boot actions
/// (build spec §6) are named here so the immutable-history events that gate the
/// compliance story share one source of truth.
pub mod action {
    /// A new immutable version was committed (`registry::commit_version`).
    pub const VERSION_COMMIT: &str = "version.commit";
    /// An older version was restored as a new head (`registry::restore_version`).
    pub const VERSION_RESTORE: &str = "version.restore";
    /// A file was tombstoned (soft-deleted; bytes retained per hold).
    pub const FILE_TOMBSTONE: &str = "file.tombstone";
    /// An ingest was rejected by the allowlist / magic-byte guard.
    pub const INGEST_REJECT: &str = "ingest.reject";
    /// A per-workspace DEK was generated + wrapped on first write.
    pub const KEY_WORKSPACE_CREATED: &str = "key.workspace_created";
    /// A boot invariant failed; the process is refusing to start.
    pub const BOOT_INVARIANT_FAILED: &str = "boot.invariant_failed";
    /// A legal hold was placed on a file / project / workspace (P1.2 compliance).
    pub const HOLD_PLACED: &str = "hold.placed";
    /// A legal hold was released (`released_at` stamped).
    pub const HOLD_RELEASED: &str = "hold.released";
    /// A workspace retention policy was set (P1.2 compliance).
    pub const RETENTION_SET: &str = "retention.set";
    /// A personal access token was issued.
    pub const TOKEN_CREATED: &str = "token.created";
    /// A personal access token was revoked.
    pub const TOKEN_REVOKED: &str = "token.revoked";
    /// A document was scanned for PII (read-only; records what was flagged).
    pub const PII_SCAN: &str = "pii.scan";
    /// A document summary was generated (read-only AI suggestion).
    pub const AI_SUMMARY: &str = "ai.summary";
    /// An admin exported the audit log (the export is itself audited).
    pub const AUDIT_EXPORT: &str = "audit.export";
}

/// Outcome of [`AuditRepo::verify_audit_chain`].
///
/// A dedicated status (rather than reusing [`dochub_crypto::ChainStatus`])
/// because the audit chain hashes an `entry_hash` preimage rather than raw
/// content bytes; the break location is all a caller needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditChainStatus {
    /// Every chained row's `entry_hash` recomputed and every `prev_hash` linked.
    Intact,
    /// The first failing row, by zero-based position in chain order.
    Broken {
        /// Zero-based index (chain order) of the first tampered / mislinked row.
        at_index: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub created_at: time::OffsetDateTime,
    pub actor_id: Option<String>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub ip_address: Option<String>,
    /// Verbatim JSON payload from `NewAuditEvent::metadata`, if any.
    pub metadata: Option<String>,
}

/// One row of an audit export — every stored field **including the raw
/// `created_at` string and both hash-chain columns**, so a recipient can
/// recompute `entry_hash` and re-walk the linkage entirely offline (see
/// [`verify_exported_chain`]). Distinct from [`AuditEvent`], whose `created_at`
/// is a parsed timestamp: the export must carry the byte-exact stored string the
/// hash was computed over.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedAuditRow {
    pub id: String,
    /// The stored RFC-3339 UTC string, verbatim (the hash preimage byte-for-byte).
    pub created_at: String,
    pub actor_id: Option<String>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub ip_address: Option<String>,
    pub metadata: Option<String>,
    /// Previous chained row's `entry_hash` (lowercase hex); `None` at the head.
    pub prev_hash: Option<String>,
    /// This row's `entry_hash` (lowercase hex).
    pub entry_hash: String,
}

/// A complete, self-verifiable audit export: the full chain in append order plus
/// the server's own verification verdict at export time. Serialized as the body
/// of `GET /api/admin/audit/export` and consumed by `dochub verify-audit`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExport {
    /// RFC-3339 UTC stamp of when the export was produced (set by the caller).
    pub generated_at: String,
    /// Number of chained rows in `events`.
    pub count: usize,
    /// The server's verdict when it produced the export: `"intact"` or
    /// `"broken_at_<index>"`. A recipient re-derives the same from `events`.
    pub chain_status: String,
    /// The full chain, seq/append order (`created_at ASC, id ASC`).
    pub events: Vec<ExportedAuditRow>,
}

/// Render an [`AuditChainStatus`] as the stable `chain_status` string.
#[must_use]
pub fn chain_status_str(status: &AuditChainStatus) -> String {
    match status {
        AuditChainStatus::Intact => "intact".to_string(),
        AuditChainStatus::Broken { at_index } => format!("broken_at_{at_index}"),
    }
}

/// Re-verify an exported chain **offline** — no database. Recomputes each row's
/// `entry_hash` from its own fields + stored `prev_hash` (catching field tamper)
/// and checks each `prev_hash` links to the previous row's `entry_hash`
/// (catching reordering / splicing), exactly as [`AuditRepo::verify_audit_chain`]
/// does against live rows. A malformed stored hash counts as a break at that
/// row. This is the check `dochub verify-audit` runs on an export file.
#[must_use]
pub fn verify_exported_chain(events: &[ExportedAuditRow]) -> AuditChainStatus {
    let mut prev: Option<Sha256Hex> = None;
    for (i, row) in events.iter().enumerate() {
        let stored_prev: Option<Sha256Hex> = match &row.prev_hash {
            Some(h) => match h.parse() {
                Ok(v) => Some(v),
                Err(_) => return AuditChainStatus::Broken { at_index: i },
            },
            None => None,
        };
        let Ok(stored_entry) = row.entry_hash.parse::<Sha256Hex>() else {
            return AuditChainStatus::Broken { at_index: i };
        };

        let link_ok = match (&prev, &stored_prev) {
            (None, None) => true,
            (Some(expected), Some(claimed)) => expected == claimed,
            _ => false,
        };
        if !link_ok {
            return AuditChainStatus::Broken { at_index: i };
        }

        let canonical = canonical(&CanonicalFields {
            id: &row.id,
            created_at: &row.created_at,
            actor_id: row.actor_id.as_deref(),
            action: &row.action,
            target_kind: row.target_kind.as_deref(),
            target_id: row.target_id.as_deref(),
            ip_address: row.ip_address.as_deref(),
            metadata: row.metadata.as_deref(),
        });
        if entry_hash(stored_prev.as_ref(), &canonical) != stored_entry {
            return AuditChainStatus::Broken { at_index: i };
        }
        prev = Some(stored_entry);
    }
    AuditChainStatus::Intact
}

#[derive(Debug, Clone)]
pub struct NewAuditEvent {
    pub actor_id: Option<String>,
    pub actor_username: Option<String>,
    pub action: String,
    pub target_kind: Option<String>,
    pub target_id: Option<String>,
    pub target_name: Option<String>,
    pub ip_address: Option<String>,
    /// Caller-supplied JSON object string. We don't parse it — callers
    /// build it with `serde_json::json!`.
    pub metadata: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditRepo<'a> {
    db: &'a Db,
}

impl<'a> AuditRepo<'a> {
    #[must_use]
    pub fn new(db: &'a Db) -> Self {
        Self { db }
    }

    /// Append one audit event, extending the global hash chain.
    ///
    /// In a single transaction: read the current chain head's `entry_hash`
    /// (deterministic order `created_at DESC, id DESC`, skipping pre-migration
    /// rows whose `entry_hash` is NULL), set it as this row's `prev_hash`
    /// (`None` at the head), compute `entry_hash = SHA-256(prev_hex ‖ 0x00 ‖
    /// canonical)`, and insert with both columns set.
    pub async fn insert(&self, new: NewAuditEvent) -> Result<AuditEvent, DbError> {
        let id = ulid::Ulid::new().to_string();
        let created_at = time::OffsetDateTime::now_utc();
        let created_s = ts(created_at);

        let mut tx = self.db.pool().begin().await?;

        // Chain head: the most recent chained row's entry_hash. Rows predating
        // migration 0017 have a NULL entry_hash and sit outside the chain.
        let head_row = sqlx::query(&self.db.sql(
            "SELECT entry_hash FROM audit_log \
             WHERE entry_hash IS NOT NULL \
             ORDER BY created_at DESC, id DESC LIMIT 1",
        ))
        .fetch_optional(&mut *tx)
        .await?;
        let prev: Option<Sha256Hex> = match head_row {
            Some(row) => {
                let hex: String = row.get("entry_hash");
                Some(
                    hex.parse()
                        .map_err(|_| DbError::Corrupt("audit entry_hash"))?,
                )
            }
            None => None,
        };

        let canonical = canonical(&CanonicalFields {
            id: &id,
            created_at: &created_s,
            actor_id: new.actor_id.as_deref(),
            action: &new.action,
            target_kind: new.target_kind.as_deref(),
            target_id: new.target_id.as_deref(),
            ip_address: new.ip_address.as_deref(),
            metadata: new.metadata.as_deref(),
        });
        let entry = entry_hash(prev.as_ref(), &canonical);

        sqlx::query(&self.db.sql(
            "INSERT INTO audit_log \
             (id, created_at, actor_id, actor_username, action, target_kind, \
              target_id, target_name, ip_address, metadata, prev_hash, entry_hash) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        ))
        .bind(&id)
        .bind(&created_s)
        .bind(&new.actor_id)
        .bind(&new.actor_username)
        .bind(&new.action)
        .bind(&new.target_kind)
        .bind(&new.target_id)
        .bind(&new.target_name)
        .bind(&new.ip_address)
        .bind(&new.metadata)
        .bind(prev.map(|p| p.to_hex()))
        .bind(entry.to_hex())
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(AuditEvent {
            id,
            created_at,
            actor_id: new.actor_id,
            actor_username: new.actor_username,
            action: new.action,
            target_kind: new.target_kind,
            target_id: new.target_id,
            target_name: new.target_name,
            ip_address: new.ip_address,
            metadata: new.metadata,
        })
    }

    /// Verify the global audit hash chain end-to-end.
    ///
    /// Walks chained rows in append order (`created_at ASC, id ASC`, skipping
    /// pre-migration NULL-`entry_hash` rows). For each row it recomputes
    /// `entry_hash` from the row's own fields + stored `prev_hash` and checks it
    /// against the stored `entry_hash` (catches any field / pointer tamper), and
    /// checks the `prev_hash` links to the previous row's `entry_hash` (catches
    /// reordering / splicing). Returns [`AuditChainStatus::Broken`] at the first
    /// failing index, else [`AuditChainStatus::Intact`] (including the empty and
    /// single-row chains). Never mutates anything.
    pub async fn verify_audit_chain(&self) -> Result<AuditChainStatus, DbError> {
        let rows = sqlx::query(&self.db.sql(
            "SELECT id, created_at, actor_id, action, target_kind, target_id, \
             ip_address, metadata, prev_hash, entry_hash \
             FROM audit_log WHERE entry_hash IS NOT NULL \
             ORDER BY created_at ASC, id ASC",
        ))
        .fetch_all(self.db.pool())
        .await?;

        let mut prev: Option<Sha256Hex> = None;
        for (i, row) in rows.iter().enumerate() {
            let id: String = row.get("id");
            let created_s: String = row.get("created_at");
            let actor_id: Option<String> = row.get("actor_id");
            let act: String = row.get("action");
            let target_kind: Option<String> = row.get("target_kind");
            let target_id: Option<String> = row.get("target_id");
            let ip_address: Option<String> = row.get("ip_address");
            let metadata: Option<String> = row.get("metadata");
            let stored_prev_hex: Option<String> = row.get("prev_hash");
            let stored_entry_hex: String = row.get("entry_hash");

            let stored_prev: Option<Sha256Hex> = match &stored_prev_hex {
                Some(h) => Some(h.parse().map_err(|_| DbError::Corrupt("audit prev_hash"))?),
                None => None,
            };
            let stored_entry: Sha256Hex = stored_entry_hex
                .parse()
                .map_err(|_| DbError::Corrupt("audit entry_hash"))?;

            // Linkage: this row's prev_hash must point at the previous row's
            // entry_hash (None only at the head).
            let link_ok = match (&prev, &stored_prev) {
                (None, None) => true,
                (Some(expected), Some(claimed)) => expected == claimed,
                _ => false,
            };
            if !link_ok {
                return Ok(AuditChainStatus::Broken { at_index: i });
            }

            // Recompute from this row's fields + its own prev_hash.
            let canonical = canonical(&CanonicalFields {
                id: &id,
                created_at: &created_s,
                actor_id: actor_id.as_deref(),
                action: &act,
                target_kind: target_kind.as_deref(),
                target_id: target_id.as_deref(),
                ip_address: ip_address.as_deref(),
                metadata: metadata.as_deref(),
            });
            if entry_hash(stored_prev.as_ref(), &canonical) != stored_entry {
                return Ok(AuditChainStatus::Broken { at_index: i });
            }

            prev = Some(stored_entry);
        }

        Ok(AuditChainStatus::Intact)
    }

    /// Fire-and-forget insert. Used by handlers that don't want to block
    /// the response on an audit write. Errors are logged, never returned.
    pub fn emit(db: &Db, event: NewAuditEvent) {
        let db = db.clone();
        tokio::spawn(async move {
            if let Err(e) = AuditRepo::new(&db).insert(event).await {
                tracing::warn!(error = %e, action = %"audit_emit_failed", "audit insert failed");
            }
        });
    }

    /// Page latest-first, filtered to one or more action strings. Used by
    /// the Admin → Recent sign-ins card. Empty `actions` returns nothing.
    pub async fn list_filtered(
        &self,
        actions: &[&str],
        limit: i64,
    ) -> Result<Vec<AuditEvent>, DbError> {
        if actions.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = vec!["?"; actions.len()].join(", ");
        let sql = format!(
            "SELECT id, created_at, actor_id, actor_username, action, \
             target_kind, target_id, target_name, ip_address, metadata \
             FROM audit_log WHERE action IN ({placeholders}) \
             ORDER BY created_at DESC LIMIT ?",
        );
        let sql = self.db.sql(&sql);
        let mut q = sqlx::query(&sql);
        for a in actions {
            q = q.bind(*a);
        }
        let rows = q
            .bind(limit.clamp(1, 200))
            .fetch_all(self.db.pool())
            .await?;
        rows.iter().map(row_to_event).collect()
    }

    /// Page latest-first. `before` is an opaque cursor (the previous
    /// page's last `created_at`); omit for the first page.
    pub async fn list(&self, before: Option<&str>, limit: i64) -> Result<Vec<AuditEvent>, DbError> {
        let rows = if let Some(before) = before {
            sqlx::query(&self.db.sql(
                "SELECT id, created_at, actor_id, actor_username, action, \
                 target_kind, target_id, target_name, ip_address, metadata \
                 FROM audit_log WHERE created_at < ? ORDER BY created_at DESC LIMIT ?",
            ))
            .bind(before)
            .bind(limit.clamp(1, 500))
            .fetch_all(self.db.pool())
            .await?
        } else {
            sqlx::query(&self.db.sql(
                "SELECT id, created_at, actor_id, actor_username, action, \
                 target_kind, target_id, target_name, ip_address, metadata \
                 FROM audit_log ORDER BY created_at DESC LIMIT ?",
            ))
            .bind(limit.clamp(1, 500))
            .fetch_all(self.db.pool())
            .await?
        };
        rows.iter().map(row_to_event).collect()
    }

    /// The complete chained audit log in append order (`created_at ASC, id ASC`),
    /// every field including both hash-chain columns — the payload of an audit
    /// export. Pre-migration rows with a NULL `entry_hash` sit outside the chain
    /// and are excluded, so [`verify_exported_chain`] can walk the result whole.
    pub async fn export_chain(&self) -> Result<Vec<ExportedAuditRow>, DbError> {
        let rows = sqlx::query(&self.db.sql(
            "SELECT id, created_at, actor_id, actor_username, action, target_kind, \
             target_id, target_name, ip_address, metadata, prev_hash, entry_hash \
             FROM audit_log WHERE entry_hash IS NOT NULL \
             ORDER BY created_at ASC, id ASC",
        ))
        .fetch_all(self.db.pool())
        .await?;
        Ok(rows
            .iter()
            .map(|row| ExportedAuditRow {
                id: row.get("id"),
                created_at: row.get("created_at"),
                actor_id: row.get("actor_id"),
                actor_username: row.get("actor_username"),
                action: row.get("action"),
                target_kind: row.get("target_kind"),
                target_id: row.get("target_id"),
                target_name: row.get("target_name"),
                ip_address: row.get("ip_address"),
                metadata: row.get("metadata"),
                prev_hash: row.get("prev_hash"),
                entry_hash: row.get("entry_hash"),
            })
            .collect())
    }
}

/// Deterministic byte serialization of an audit event's **stable** fields — the
/// `canonical` preimage fed to [`dochub_crypto::entry_hash`].
///
/// Fields, in this exact fixed order: `id`, `created_at` (the stored RFC-3339
/// UTC string), `actor_id`, `action`, `target_kind`, `target_id`, `ip_address`,
/// `metadata`. Deliberately **excluded**: the denormalized display strings
/// `actor_username` / `target_name`, which can legitimately change without the
/// event's meaning changing.
///
/// Each field is length-prefixed so field boundaries are unforgeable even when
/// a value is empty or contains the separator byte:
///
/// ```text
/// canonical  = F(id) ‖ F(created_at) ‖ F(actor_id) ‖ F(action)
///            ‖ F(target_kind) ‖ F(target_id) ‖ F(ip_address) ‖ F(metadata)
/// F(None)    = 0x00
/// F(Some(s)) = 0x01 ‖ be_u64(len(utf8(s))) ‖ utf8(s)
/// ```
#[derive(Debug)]
struct CanonicalFields<'a> {
    id: &'a str,
    created_at: &'a str,
    actor_id: Option<&'a str>,
    action: &'a str,
    target_kind: Option<&'a str>,
    target_id: Option<&'a str>,
    ip_address: Option<&'a str>,
    metadata: Option<&'a str>,
}

fn canonical(f: &CanonicalFields) -> Vec<u8> {
    let mut buf = Vec::new();
    push_field(&mut buf, Some(f.id));
    push_field(&mut buf, Some(f.created_at));
    push_field(&mut buf, f.actor_id);
    push_field(&mut buf, Some(f.action));
    push_field(&mut buf, f.target_kind);
    push_field(&mut buf, f.target_id);
    push_field(&mut buf, f.ip_address);
    push_field(&mut buf, f.metadata);
    buf
}

/// Append one length-prefixed field to the canonical preimage. `None` is a lone
/// `0x00`; `Some(s)` is `0x01`, the big-endian u64 UTF-8 byte length, then the
/// bytes — so `None`, `Some("")`, and any content are mutually unambiguous.
fn push_field(buf: &mut Vec<u8>, field: Option<&str>) {
    match field {
        None => buf.push(0x00),
        Some(s) => {
            buf.push(0x01);
            buf.extend_from_slice(&(s.len() as u64).to_be_bytes());
            buf.extend_from_slice(s.as_bytes());
        }
    }
}

fn row_to_event(row: &sqlx::any::AnyRow) -> Result<AuditEvent, DbError> {
    Ok(AuditEvent {
        id: row.get("id"),
        created_at: parse_ts(row.get::<String, _>("created_at"))?,
        actor_id: row.get("actor_id"),
        actor_username: row.get("actor_username"),
        action: row.get("action"),
        target_kind: row.get("target_kind"),
        target_id: row.get("target_id"),
        target_name: row.get("target_name"),
        ip_address: row.get("ip_address"),
        metadata: row.get("metadata"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn fresh_db() -> Db {
        Db::connect("sqlite::memory:").await.expect("connect")
    }

    fn event(action: &str) -> NewAuditEvent {
        NewAuditEvent {
            actor_id: Some("u_actor".into()),
            actor_username: Some("actor".into()),
            action: action.into(),
            target_kind: Some("file".into()),
            target_id: Some("F_1".into()),
            target_name: Some("Q3.xlsx".into()),
            ip_address: Some("10.0.0.1".into()),
            metadata: Some(r#"{"k":"v"}"#.into()),
        }
    }

    #[tokio::test]
    async fn empty_chain_is_intact() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);
        assert_eq!(
            repo.verify_audit_chain().await.expect("verify"),
            AuditChainStatus::Intact
        );
    }

    #[tokio::test]
    async fn single_entry_chain_is_intact() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);
        repo.insert(event(action::VERSION_COMMIT))
            .await
            .expect("insert");
        assert_eq!(
            repo.verify_audit_chain().await.expect("verify"),
            AuditChainStatus::Intact
        );
    }

    #[tokio::test]
    async fn export_round_trips_and_verifies_offline() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);
        for a in [
            action::VERSION_COMMIT,
            action::FILE_TOMBSTONE,
            action::PII_SCAN,
        ] {
            repo.insert(event(a)).await.expect("insert");
        }
        let export = repo.export_chain().await.expect("export");
        assert_eq!(export.len(), 3);
        // Exported in append order, fully linked — verifies with no database.
        assert_eq!(verify_exported_chain(&export), AuditChainStatus::Intact);

        // Tamper a field on the middle row → its recomputed entry_hash no longer
        // matches, so offline verification breaks exactly there.
        let mut tampered = export.clone();
        tampered[1].action = "version.forged".into();
        assert_eq!(
            verify_exported_chain(&tampered),
            AuditChainStatus::Broken { at_index: 1 }
        );

        // Reordering (splicing) breaks linkage at the first out-of-place row.
        let mut reordered = export.clone();
        reordered.swap(0, 1);
        assert!(matches!(
            verify_exported_chain(&reordered),
            AuditChainStatus::Broken { .. }
        ));
    }

    #[tokio::test]
    async fn appends_chain_and_link_prev_to_entry() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);

        for i in 0..8 {
            // Distinct actions from the Phase-0 vocabulary + a legacy literal.
            let a = if i % 2 == 0 {
                action::VERSION_COMMIT
            } else {
                "auth.sign_in"
            };
            repo.insert(event(a)).await.expect("insert");
        }

        // Read the chain in append order and assert prev_hash links.
        let rows = sqlx::query(&db.sql(
            "SELECT prev_hash, entry_hash FROM audit_log \
             ORDER BY created_at ASC, id ASC",
        ))
        .fetch_all(db.pool())
        .await
        .expect("rows");
        assert_eq!(rows.len(), 8);

        let mut prev: Option<String> = None;
        for row in &rows {
            let ph: Option<String> = row.get("prev_hash");
            let eh: String = row.get("entry_hash");
            assert_eq!(ph, prev, "each prev_hash equals the prior entry_hash");
            prev = Some(eh);
        }

        assert_eq!(
            repo.verify_audit_chain().await.expect("verify"),
            AuditChainStatus::Intact
        );
    }

    #[tokio::test]
    async fn tampered_field_breaks_at_that_row() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);
        for _ in 0..5 {
            repo.insert(event(action::FILE_TOMBSTONE))
                .await
                .expect("insert");
        }

        // The id of the 3rd row (index 2) in chain order.
        let ordered =
            sqlx::query(&db.sql("SELECT id FROM audit_log ORDER BY created_at ASC, id ASC"))
                .fetch_all(db.pool())
                .await
                .expect("ids");
        let victim: String = ordered[2].get("id");

        // Simulate tampering a committed row's field (never done in real code).
        sqlx::query(&db.sql("UPDATE audit_log SET action = ? WHERE id = ?"))
            .bind("tampered.action")
            .bind(&victim)
            .execute(db.pool())
            .await
            .expect("tamper");

        assert_eq!(
            repo.verify_audit_chain().await.expect("verify"),
            AuditChainStatus::Broken { at_index: 2 }
        );
    }

    #[tokio::test]
    async fn tampered_metadata_breaks_at_that_row() {
        let db = fresh_db().await;
        let repo = AuditRepo::new(&db);
        for _ in 0..3 {
            repo.insert(event(action::INGEST_REJECT))
                .await
                .expect("insert");
        }
        let ordered =
            sqlx::query(&db.sql("SELECT id FROM audit_log ORDER BY created_at ASC, id ASC"))
                .fetch_all(db.pool())
                .await
                .expect("ids");
        let victim: String = ordered[0].get("id");

        sqlx::query(&db.sql("UPDATE audit_log SET metadata = ? WHERE id = ?"))
            .bind(r#"{"k":"tampered"}"#)
            .bind(&victim)
            .execute(db.pool())
            .await
            .expect("tamper");

        assert_eq!(
            repo.verify_audit_chain().await.expect("verify"),
            AuditChainStatus::Broken { at_index: 0 }
        );
    }

    #[tokio::test]
    async fn canonical_is_length_prefixed_unambiguous() {
        // None, Some(""), and content never collide across the field boundary.
        let base = CanonicalFields {
            id: "id",
            created_at: "t",
            actor_id: None,
            action: "act",
            target_kind: None,
            target_id: None,
            ip_address: None,
            metadata: None,
        };
        let a = canonical(&base);
        let b = canonical(&CanonicalFields {
            actor_id: Some(""),
            ..base
        });
        assert_ne!(a, b);
    }
}
