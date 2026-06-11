//! MU1 Phase 1a — workspace invitations HTTP surface.
//!
//! Spec: [[workspace-invitations]] memory entry. Endpoints:
//!
//!   - `POST   /api/workspaces/{id}/invitations`         — create
//!   - `GET    /api/workspaces/{id}/invitations`         — list pending
//!   - `DELETE /api/workspaces/{id}/invitations/{inv_id}` — revoke
//!   - `GET    /api/invitations/{token}`                  — public peek
//!   - `POST   /api/invitations/{token}/accept`           — signed-in accept
//!
//! Auth model:
//!   - Create / list / revoke: caller must be a member of the
//!     workspace (`WorkspaceMemberRepo::role_of`). Role gate matches
//!     the existing "members invite members" rule until MU2 adds
//!     Admin / Owner-only invite tiers.
//!   - Peek: anonymous-safe. Returns only the workspace name,
//!     inviter username, role, expires_at, and remaining-uses count
//!     — never the token itself.
//!   - Accept: signed-in only in Phase 1a. Magic-link auto-create
//!     (anonymous → mint user + session) ships in Phase 1d.
//!
//! Token generation: 32 random bytes → URL-safe base64. Constant-
//! time compare happens implicitly in the indexed SQL WHERE — the
//! attacker has no oracle to brute against in any case.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use base64::Engine;
use drive_auth::AuthSession;
use drive_db::{
    AuditRepo, NewAuditEvent, NewWorkspaceInvitation, UserRepo, WorkspaceInvitation,
    WorkspaceInvitationRepo, WorkspaceMemberRepo, WorkspaceRepo, WorkspaceRole,
};
use serde::{Deserialize, Serialize};

use crate::state::HttpState;

/// Newly-created invitation row, returned to the inviter so they can
/// share the URL. The `token` is in plaintext here ON THIS SINGLE
/// RESPONSE only — list endpoints return a token-less DTO.
#[derive(Debug, Serialize)]
struct InvitationCreatedDto {
    id: String,
    token: String,
    role: String,
    expires_at: Option<String>,
    max_uses: i64,
}

/// List-endpoint shape. Token is omitted to keep the wire shape
/// minimal — owners who want to re-share generate a fresh invite.
#[derive(Debug, Serialize)]
struct InvitationDto {
    id: String,
    role: String,
    created_at: String,
    created_by: String,
    expires_at: Option<String>,
    max_uses: i64,
    used_count: i64,
    revoked: bool,
}

/// Public peek payload — what the anonymous `/invite/<token>` page
/// renders. No PII beyond what the inviter chose to share by sending
/// the link.
#[derive(Debug, Serialize)]
struct InvitationPeekDto {
    workspace_name: String,
    inviter_username: String,
    role: String,
    expires_at: Option<String>,
    remaining_uses: i64,
}

#[derive(Debug, Deserialize)]
struct CreateBody {
    /// "member" only in Phase 1a. Admin role gets unlocked when MU2
    /// ships role tiers; "owner" is never grantable via invite.
    #[serde(default = "default_role")]
    role: String,
    /// Hours from now until expiry. `null` = never expires.
    #[serde(default)]
    expires_in_hours: Option<i64>,
    /// 1 = single-use (default); higher = multi-use cap.
    #[serde(default = "default_max_uses")]
    max_uses: i64,
}

fn default_role() -> String {
    "member".into()
}

fn default_max_uses() -> i64 {
    1
}

#[derive(Debug, Serialize)]
struct ErrBody<'a> {
    error: &'a str,
}

// ── Helpers ──────────────────────────────────────────────────────

async fn require_member(
    s: &HttpState,
    workspace_id: &str,
    user_id: &str,
) -> Result<WorkspaceRole, (StatusCode, Json<ErrBody<'static>>)> {
    let members = WorkspaceMemberRepo::new(&s.db);
    match members.role_of(workspace_id, user_id).await {
        Ok(Some(role)) => Ok(role),
        Ok(None) => Err((
            StatusCode::FORBIDDEN,
            Json(ErrBody {
                error: "not a member of this workspace",
            }),
        )),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrBody {
                error: "membership lookup failed",
            }),
        )),
    }
}

/// Mint a 32-byte URL-safe random token. Same OsRng channel as
/// drive-auth's session / CSRF tokens and share.rs's link tokens.
/// 32 bytes = 256 bits of entropy → 43 base64-no-pad chars.
fn mint_token() -> String {
    use argon2::password_hash::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

fn iso(t: time::OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

fn invitation_to_list_dto(inv: &WorkspaceInvitation) -> InvitationDto {
    InvitationDto {
        id: inv.id.clone(),
        role: inv.role.clone(),
        created_at: iso(inv.created_at),
        created_by: inv.created_by.clone(),
        expires_at: inv.expires_at.map(iso),
        max_uses: inv.max_uses,
        used_count: inv.used_count,
        revoked: inv.revoked_at.is_some(),
    }
}

// ── Handlers ─────────────────────────────────────────────────────

/// `POST /api/workspaces/{id}/invitations` — mint a fresh invite for
/// the workspace. The plaintext token is in the response ONCE; later
/// reads through the list endpoint omit it. Caller must be a member.
async fn create_invitation(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<InvitationCreatedDto>), (StatusCode, Json<ErrBody<'static>>)> {
    require_member(&s, &workspace_id, &session.user_id).await?;

    let role = match body.role.as_str() {
        "member" => "member",
        "admin" => {
            // MU2 unlocks Admin invites. Until then, reject explicitly
            // rather than silently downgrading — keeps the API honest
            // about what's wired.
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrBody {
                    error: "admin role invitations are not enabled yet (see MU2)",
                }),
            ));
        }
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrBody {
                    error: "role must be 'member'",
                }),
            ));
        }
    };

    if body.max_uses < 1 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrBody {
                error: "max_uses must be >= 1",
            }),
        ));
    }
    // 1000 is a generous ceiling — anyone needing more is doing
    // organisational onboarding, which probably wants per-domain
    // policies anyway.
    if body.max_uses > 1000 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrBody {
                error: "max_uses must be <= 1000",
            }),
        ));
    }

    let expires_at = body
        .expires_in_hours
        .filter(|h| *h > 0)
        .map(|h| time::OffsetDateTime::now_utc() + time::Duration::hours(h));

    let token = mint_token();
    let repo = WorkspaceInvitationRepo::new(&s.db);
    let inv = repo
        .insert(&NewWorkspaceInvitation {
            workspace_id: workspace_id.clone(),
            token: token.clone(),
            role: role.into(),
            created_by: session.user_id.clone(),
            expires_at,
            max_uses: body.max_uses,
        })
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrBody {
                    error: "insert failed",
                }),
            )
        })?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "workspace.invited".into(),
            target_kind: Some("workspace".into()),
            target_id: Some(workspace_id.clone()),
            target_name: None,
            ip_address: None,
            metadata: Some(format!(
                r#"{{"invitation_id":"{}","role":"{}","max_uses":{}}}"#,
                inv.id, inv.role, inv.max_uses
            )),
        },
    );

    Ok((
        StatusCode::CREATED,
        Json(InvitationCreatedDto {
            id: inv.id,
            token,
            role: inv.role,
            expires_at: inv.expires_at.map(iso),
            max_uses: inv.max_uses,
        }),
    ))
}

/// `GET /api/workspaces/{id}/invitations` — list invitations for the
/// Settings → Members tab. Member-gated.
async fn list_invitations(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(workspace_id): Path<String>,
) -> Result<Json<Vec<InvitationDto>>, (StatusCode, Json<ErrBody<'static>>)> {
    require_member(&s, &workspace_id, &session.user_id).await?;
    let repo = WorkspaceInvitationRepo::new(&s.db);
    let invs = repo.list_for_workspace(&workspace_id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrBody {
                error: "list failed",
            }),
        )
    })?;
    Ok(Json(invs.iter().map(invitation_to_list_dto).collect()))
}

/// `DELETE /api/workspaces/{id}/invitations/{inv_id}` — revoke.
/// Member-gated; idempotent (re-revoking a revoked invite returns 204
/// without error).
async fn revoke_invitation(
    State(s): State<HttpState>,
    session: AuthSession,
    Path((workspace_id, inv_id)): Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<ErrBody<'static>>)> {
    require_member(&s, &workspace_id, &session.user_id).await?;
    let repo = WorkspaceInvitationRepo::new(&s.db);

    // Verify the invitation actually belongs to this workspace — a
    // member of workspace A can't revoke an invite for workspace B.
    let inv = repo.find_by_id(&inv_id).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrBody {
                error: "invitation not found",
            }),
        )
    })?;
    if inv.workspace_id != workspace_id {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrBody {
                error: "invitation not found",
            }),
        ));
    }

    repo.revoke(&inv_id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrBody {
                error: "revoke failed",
            }),
        )
    })?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "workspace.invitation_revoked".into(),
            target_kind: Some("workspace_invitation".into()),
            target_id: Some(inv_id),
            target_name: None,
            ip_address: None,
            metadata: None,
        },
    );

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /api/invitations/{token}` — anonymous-safe peek. Renders the
/// accept card. Returns 404 (not 410 / 401) for revoked / expired /
/// exhausted invites — keeps the wire shape one-dimensional and
/// doesn't leak whether the token existed at all.
async fn peek_invitation(
    State(s): State<HttpState>,
    Path(token): Path<String>,
) -> Result<Json<InvitationPeekDto>, (StatusCode, Json<ErrBody<'static>>)> {
    let repo = WorkspaceInvitationRepo::new(&s.db);
    let inv = match repo.find_by_token(&token).await {
        Ok(inv) if inv.is_consumable() => inv,
        _ => {
            return Err((
                StatusCode::NOT_FOUND,
                Json(ErrBody {
                    error: "invitation not found",
                }),
            ));
        }
    };

    let workspace = WorkspaceRepo::new(&s.db)
        .find_by_id(&inv.workspace_id)
        .await
        .map_err(|_| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrBody {
                    error: "workspace not found",
                }),
            )
        })?;
    let inviter = match UserRepo::new(&s.db).find_by_id(&inv.created_by).await {
        Ok(u) => u.username,
        Err(_) => "someone".into(),
    };

    Ok(Json(InvitationPeekDto {
        workspace_name: workspace.name,
        inviter_username: inviter,
        role: inv.role,
        expires_at: inv.expires_at.map(iso),
        remaining_uses: inv.max_uses - inv.used_count,
    }))
}

/// `POST /api/invitations/{token}/accept` — accept an invitation as
/// the signed-in caller. Phase 1a is signed-in-only; anonymous
/// callers get 401 and the SPA's accept page bounces them to sign-in
/// with a return URL. Magic-link auto-create (anonymous → new user)
/// ships in MU1 Phase 1d.
async fn accept_invitation(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(token): Path<String>,
) -> Result<(StatusCode, Json<AcceptResp>), (StatusCode, Json<ErrBody<'static>>)> {
    let repo = WorkspaceInvitationRepo::new(&s.db);
    let inv = repo.find_by_token(&token).await.map_err(|_| {
        (
            StatusCode::NOT_FOUND,
            Json(ErrBody {
                error: "invitation not found",
            }),
        )
    })?;

    // Already a member? Idempotent — return 200 BEFORE the
    // consumable check, so a user clicking their own already-used
    // invite link from email history gets the friendly path rather
    // than a 409 about exhaustion that's true but unhelpful.
    let members = WorkspaceMemberRepo::new(&s.db);
    if let Ok(Some(_)) = members.role_of(&inv.workspace_id, &session.user_id).await {
        return Ok((
            StatusCode::OK,
            Json(AcceptResp {
                workspace_id: inv.workspace_id,
                already_member: true,
            }),
        ));
    }

    // Not yet a member — token must still have capacity.
    if !inv.is_consumable() {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrBody {
                error: "invitation expired or exhausted",
            }),
        ));
    }

    // Atomic consume. If two clients race the same single-use token,
    // exactly one wins; the loser gets 409.
    let consumed = repo.try_consume(&inv.id).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrBody {
                error: "consume failed",
            }),
        )
    })?;
    if consumed == 0 {
        return Err((
            StatusCode::CONFLICT,
            Json(ErrBody {
                error: "invitation expired or exhausted",
            }),
        ));
    }

    let role = match inv.role.as_str() {
        "admin" => WorkspaceRole::Owner, // future-proofing; admin maps to elevated for now
        _ => WorkspaceRole::Member,
    };
    members
        .insert(&inv.workspace_id, &session.user_id, role)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrBody {
                    error: "membership insert failed",
                }),
            )
        })?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "workspace.joined".into(),
            target_kind: Some("workspace".into()),
            target_id: Some(inv.workspace_id.clone()),
            target_name: None,
            ip_address: None,
            metadata: Some(format!(r#"{{"invitation_id":"{}"}}"#, inv.id)),
        },
    );

    // RT1 1c — quiet broadcast so existing members' RT4 toast can
    // announce the new arrival. Uses the "+ joined" wording on the
    // SPA side.
    s.presence
        .broadcast_action(
            &inv.workspace_id,
            &session.user_id,
            "workspace.joined",
            Some(&inv.workspace_id),
            Some(&session.username),
        )
        .await;

    Ok((
        StatusCode::OK,
        Json(AcceptResp {
            workspace_id: inv.workspace_id,
            already_member: false,
        }),
    ))
}

#[derive(Debug, Serialize)]
struct AcceptResp {
    workspace_id: String,
    already_member: bool,
}

/// Mount the five endpoints under the app origin. Workspace-scoped
/// + token-scoped routes share the same `HttpState`.
pub(crate) fn router(state: HttpState) -> Router {
    Router::new()
        .route(
            "/api/workspaces/{workspace_id}/invitations",
            post(create_invitation).get(list_invitations),
        )
        .route(
            "/api/workspaces/{workspace_id}/invitations/{invitation_id}",
            delete(revoke_invitation),
        )
        .route("/api/invitations/{token}", get(peek_invitation))
        .route("/api/invitations/{token}/accept", post(accept_invitation))
        .with_state(state)
}
