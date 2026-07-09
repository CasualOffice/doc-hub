//! Workspace member management — change a member's role, remove a member.
//! Spec: docs/design/foundation-access-rag-mcp.md §2–§3 (F2a).
//!
//! Endpoints (gated by `manage_members`, audited):
//!   - `PUT    /api/workspaces/{id}/members/{userId}/role`  — change role
//!   - `DELETE /api/workspaces/{id}/members/{userId}`       — remove member
//!
//! Roles are the F1 model (`viewer|editor|admin|owner`). Writing a member's
//! role through [`WorkspaceMemberRepo::add`] stores the literal string, so
//! [`dochub_authz`] resolves the real effective permissions on the next call
//! (promote → can edit, demote → can't). Guards: can't demote the last owner,
//! can't remove the workspace owner.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, put},
    Json, Router,
};
use dochub_auth::AuthSession;
use dochub_authz::{Permission, ResourceRef, Role};
use dochub_db::{AuditRepo, NewAuditEvent, WorkspaceMemberRepo, WorkspaceRepo};
use serde::{Deserialize, Serialize};

use crate::authz::gate;
use crate::HttpState;

#[derive(Debug, thiserror::Error)]
pub(crate) enum MemberError {
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("validation: {0}")]
    Validation(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrBody<'a> {
    error: &'a str,
}

impl From<dochub_authz::AuthzError> for MemberError {
    fn from(e: dochub_authz::AuthzError) -> Self {
        match e {
            dochub_authz::AuthzError::Forbidden => Self::Forbidden,
            dochub_authz::AuthzError::Db(err) => Self::Internal(err.to_string()),
        }
    }
}

impl IntoResponse for MemberError {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => {
                (StatusCode::NOT_FOUND, Json(ErrBody { error: "not found" })).into_response()
            }
            Self::Forbidden => {
                (StatusCode::FORBIDDEN, Json(ErrBody { error: "forbidden" })).into_response()
            }
            Self::Validation(m) => {
                (StatusCode::BAD_REQUEST, Json(ErrBody { error: &m })).into_response()
            }
            Self::Conflict(m) => {
                (StatusCode::CONFLICT, Json(ErrBody { error: &m })).into_response()
            }
            Self::Internal(m) => {
                tracing::error!(error = %m, "members handler error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrBody {
                        error: "internal error",
                    }),
                )
                    .into_response()
            }
        }
    }
}

#[derive(Deserialize)]
struct RoleBody {
    /// `viewer` | `editor` | `admin` | `owner`.
    role: String,
}

/// `PUT /api/workspaces/{id}/members/{userId}/role` — change a member's
/// workspace role. Gated by `manage_members`; refuses to demote the last
/// owner. Audited `workspace.member_role_changed`.
async fn change_role(
    State(s): State<HttpState>,
    session: AuthSession,
    Path((workspace_id, user_id)): Path<(String, String)>,
    Json(body): Json<RoleBody>,
) -> Result<StatusCode, MemberError> {
    gate(
        &s,
        &session,
        ResourceRef::Workspace(workspace_id.clone()),
        Permission::ManageMembers,
    )
    .await?;

    let new_role = Role::from_db(body.role.trim()).ok_or_else(|| {
        MemberError::Validation("role must be one of: viewer, editor, admin, owner".into())
    })?;

    let members = WorkspaceMemberRepo::new(&s.db);
    let current = members
        .role_name(&workspace_id, &user_id)
        .await
        .map_err(|e| MemberError::Internal(e.to_string()))?
        .ok_or(MemberError::NotFound)?;
    let current_role = Role::from_db(&current);

    // Last-owner guard: demoting the sole owner would orphan the workspace.
    if current_role == Some(Role::Owner) && new_role != Role::Owner {
        let owners = members
            .count_with_role(&workspace_id, Role::Owner.as_str())
            .await
            .map_err(|e| MemberError::Internal(e.to_string()))?;
        if owners <= 1 {
            return Err(MemberError::Conflict("cannot demote the last owner".into()));
        }
    }

    members
        .add(&workspace_id, &user_id, new_role.as_str())
        .await
        .map_err(|e| MemberError::Internal(e.to_string()))?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "workspace.member_role_changed".into(),
            target_kind: Some("workspace".into()),
            target_id: Some(workspace_id.clone()),
            target_name: None,
            ip_address: None,
            metadata: Some(format!(
                r#"{{"user_id":"{}","from":"{}","to":"{}"}}"#,
                user_id,
                current,
                new_role.as_str()
            )),
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

/// `DELETE /api/workspaces/{id}/members/{userId}` — remove a member. Gated by
/// `manage_members`; refuses to remove the workspace owner. Audited.
async fn remove_member(
    State(s): State<HttpState>,
    session: AuthSession,
    Path((workspace_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, MemberError> {
    gate(
        &s,
        &session,
        ResourceRef::Workspace(workspace_id.clone()),
        Permission::ManageMembers,
    )
    .await?;

    let ws = WorkspaceRepo::new(&s.db)
        .find_by_id(&workspace_id)
        .await
        .map_err(|_| MemberError::NotFound)?;
    if ws.owner_id == user_id {
        return Err(MemberError::Conflict(
            "cannot remove the workspace owner".into(),
        ));
    }

    let members = WorkspaceMemberRepo::new(&s.db);
    if members
        .role_name(&workspace_id, &user_id)
        .await
        .map_err(|e| MemberError::Internal(e.to_string()))?
        .is_none()
    {
        return Err(MemberError::NotFound);
    }

    members
        .delete(&workspace_id, &user_id)
        .await
        .map_err(|e| MemberError::Internal(e.to_string()))?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "workspace.member_removed".into(),
            target_kind: Some("workspace".into()),
            target_id: Some(workspace_id.clone()),
            target_name: None,
            ip_address: None,
            metadata: Some(format!(r#"{{"user_id":"{user_id}"}}"#)),
        },
    );
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) fn router(state: HttpState) -> Router {
    Router::new()
        .route(
            "/api/workspaces/{workspace_id}/members/{user_id}/role",
            put(change_role),
        )
        .route(
            "/api/workspaces/{workspace_id}/members/{user_id}",
            delete(remove_member),
        )
        .with_state(state)
}
