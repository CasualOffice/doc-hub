//! Thin HTTP-side wrapper over [`dochub_authz`]: a single [`gate`] that runs
//! `dochub_authz::require` and, on denial, emits an `authz.deny` audit event
//! before returning `Forbidden`. Handlers convert [`dochub_authz::AuthzError`]
//! into their own error type via the `From` impls in their modules.

use dochub_auth::AuthSession;
use dochub_authz::{require, AuthzError, Permission, ResourceRef};
use dochub_db::{AuditRepo, NewAuditEvent};

use crate::HttpState;

/// `(resource_kind, resource_id)` for audit metadata.
fn parts(resource: &ResourceRef) -> (&'static str, &str) {
    match resource {
        ResourceRef::Workspace(id) => ("workspace", id),
        ResourceRef::Project(id) => ("project", id),
        ResourceRef::Folder(id) => ("folder", id),
        ResourceRef::File(id) => ("file", id),
    }
}

/// Stable lowercase label for a permission, used in the deny audit metadata.
fn perm_label(perm: Permission) -> &'static str {
    match perm {
        Permission::View => "view",
        Permission::Download => "download",
        Permission::Comment => "comment",
        Permission::Edit => "edit",
        Permission::Create => "create",
        Permission::Delete => "delete",
        Permission::Share => "share",
        Permission::ManageMembers => "manage_members",
        Permission::ManageSettings => "manage_settings",
        Permission::ManageRetention => "manage_retention",
        Permission::ManageKeys => "manage_keys",
    }
}

/// Enforce `perm` on `resource` for the session's user. On denial, audit
/// (`authz.deny`) and return `AuthzError::Forbidden`; DB errors pass through.
pub(crate) async fn gate(
    s: &HttpState,
    session: &AuthSession,
    resource: ResourceRef,
    perm: Permission,
) -> Result<(), AuthzError> {
    match require(&s.db, &session.user_id, &resource, perm).await {
        Ok(()) => Ok(()),
        Err(AuthzError::Forbidden) => {
            let (kind, id) = parts(&resource);
            AuditRepo::emit(
                &s.db,
                NewAuditEvent {
                    actor_id: Some(session.user_id.clone()),
                    actor_username: Some(session.username.clone()),
                    action: "authz.deny".into(),
                    target_kind: Some(kind.into()),
                    target_id: Some(id.to_string()),
                    target_name: None,
                    ip_address: None,
                    metadata: Some(format!(
                        r#"{{"permission":"{}","resource_kind":"{kind}"}}"#,
                        perm_label(perm)
                    )),
                },
            );
            Err(AuthzError::Forbidden)
        }
        Err(other) => Err(other),
    }
}
