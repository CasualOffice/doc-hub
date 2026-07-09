//! Effective-permission resolution + central enforcement.
//! Spec: docs/design/foundation-access-rag-mcp.md §3.
//!
//! `effective_perms` walks a resource up its tree (file → folder(s) → project →
//! workspace) and unions the user's role permissions at the project/workspace
//! scope with every matching `acl_grants` row on the resource or any ancestor.
//! **Superadmins bypass** (all permissions); with no membership and no grant the
//! effective set is empty — deny-by-default. `require` is the single gate every
//! `dochub-http` handler calls; `readable_scope` backs list/search filtering.

use std::collections::HashSet;

use dochub_db::{
    resource_kind, subject_kind, AclRepo, Db, DbError, File, FileRepo, Folder, FolderRepo,
    ProjectMemberRepo, ProjectRepo, UserRepo, WorkspaceMemberRepo, WorkspaceRepo,
};

use crate::perms::{PermSet, Permission, Role};

/// How deep the folder-ancestor walk will follow `parent_id` before giving up.
/// Guards against a pathological or cyclic tree; real trees are shallow.
const MAX_FOLDER_DEPTH: usize = 64;

/// A reference to a resource the caller wants to act on.
#[derive(Debug, Clone)]
pub enum ResourceRef {
    Workspace(String),
    Project(String),
    Folder(String),
    File(String),
}

/// Enforcement / resolution error.
#[derive(Debug, thiserror::Error)]
pub enum AuthzError {
    /// The user lacks the required permission on the resource.
    #[error("forbidden")]
    Forbidden,
    /// A database error while resolving permissions.
    #[error(transparent)]
    Db(#[from] DbError),
}

/// The chain of (resource_kind, resource_id) nodes plus the scope ids resolved
/// for base-role lookup.
#[derive(Debug, Default)]
struct Chain {
    nodes: Vec<(String, String)>,
    project_id: Option<String>,
    workspace_id: Option<String>,
}

/// True when the user is a system superadmin (`users.is_admin`).
async fn is_superadmin(db: &Db, user_id: &str) -> Result<bool, DbError> {
    match UserRepo::new(db).find_by_id(user_id).await {
        Ok(u) => Ok(u.is_admin),
        Err(DbError::NotFound) => Ok(false),
        Err(e) => Err(e),
    }
}

/// Walk `folder_id` and its ancestors, pushing `(folder, id)` nodes. Tolerant of
/// a dangling parent (stops rather than erroring) so a broken link never denies
/// a resource the rest of whose chain is intact.
async fn push_folder_chain(db: &Db, folder_id: &str, nodes: &mut Vec<(String, String)>) {
    let repo = FolderRepo::new(db);
    let mut current = Some(folder_id.to_string());
    let mut depth = 0;
    while let Some(id) = current {
        if depth >= MAX_FOLDER_DEPTH {
            break;
        }
        match repo.find_by_id(&id).await {
            Ok(folder) => {
                nodes.push((resource_kind::FOLDER.to_string(), folder.id.clone()));
                current = folder.parent_id;
            }
            Err(_) => break,
        }
        depth += 1;
    }
}

/// Build the resolution chain for a resource: the resource node, its folder
/// ancestors, its project, and its workspace — most specific first.
async fn build_chain(db: &Db, resource: &ResourceRef) -> Result<Chain, AuthzError> {
    let mut chain = Chain::default();
    match resource {
        ResourceRef::File(id) => {
            let file = FileRepo::new(db).find_by_id(id).await?;
            chain
                .nodes
                .push((resource_kind::FILE.to_string(), id.clone()));
            if let Some(parent) = file.parent_id.as_deref() {
                push_folder_chain(db, parent, &mut chain.nodes).await;
            }
            chain.project_id = file.project_id.clone();
            chain.workspace_id = file.workspace_id.clone();
        }
        ResourceRef::Folder(id) => {
            let folder = FolderRepo::new(db).find_by_id(id).await?;
            push_folder_chain(db, id, &mut chain.nodes).await;
            chain.project_id = folder.project_id.clone();
            chain.workspace_id = folder.workspace_id.clone();
        }
        ResourceRef::Project(id) => {
            let project = ProjectRepo::new(db).find_by_id(id).await?;
            chain
                .nodes
                .push((resource_kind::PROJECT.to_string(), id.clone()));
            chain.project_id = Some(id.clone());
            chain.workspace_id = Some(project.workspace_id);
        }
        ResourceRef::Workspace(id) => {
            chain
                .nodes
                .push((resource_kind::WORKSPACE.to_string(), id.clone()));
            chain.workspace_id = Some(id.clone());
        }
    }
    if let Some(pid) = &chain.project_id {
        chain
            .nodes
            .push((resource_kind::PROJECT.to_string(), pid.clone()));
    }
    if let Some(wid) = &chain.workspace_id {
        chain
            .nodes
            .push((resource_kind::WORKSPACE.to_string(), wid.clone()));
    }
    Ok(chain)
}

/// Resolve the user's **effective** permission set on `resource`: the union of
/// their project/workspace role permissions and every matching ACL grant on the
/// resource or an ancestor. Superadmin ⇒ all; no membership + no grant ⇒ empty.
pub async fn effective_perms(
    db: &Db,
    user_id: &str,
    resource: &ResourceRef,
) -> Result<PermSet, AuthzError> {
    if is_superadmin(db, user_id).await? {
        return Ok(PermSet::all());
    }

    let chain = build_chain(db, resource).await?;
    let mut acc = PermSet::EMPTY;

    // Base role: the user's project role if they are a project member, else
    // their workspace role. This role's permissions apply across the scope.
    let mut base_role: Option<Role> = None;
    if let Some(pid) = &chain.project_id {
        if let Some(r) = ProjectMemberRepo::new(db).role_of(pid, user_id).await? {
            base_role = Role::from_db(&r);
        }
    }
    if base_role.is_none() {
        if let Some(wid) = &chain.workspace_id {
            if let Some(r) = WorkspaceMemberRepo::new(db).role_name(wid, user_id).await? {
                base_role = Role::from_db(&r);
            }
        }
    }
    let mut held_roles: Vec<Role> = Vec::new();
    if let Some(role) = base_role {
        acc = acc.union(role.permissions());
        held_roles.push(role);
    }

    // ACL grants on any node in the chain, matched to the user directly or to a
    // role the user holds.
    let acl = AclRepo::new(db);
    for (kind, id) in &chain.nodes {
        for grant in acl.list_for_resource(kind, id).await? {
            let matches = match grant.subject_kind.as_str() {
                subject_kind::USER => grant.subject_id == user_id,
                subject_kind::ROLE => {
                    Role::from_db(&grant.subject_id).is_some_and(|gr| held_roles.contains(&gr))
                }
                _ => false,
            };
            if matches {
                if let Some(role) = Role::from_db(&grant.role) {
                    acc = acc.union(role.permissions());
                }
            }
        }
    }

    Ok(acc)
}

/// Deny-by-default gate. `Ok(())` when the user has `perm` on `resource`,
/// `Err(AuthzError::Forbidden)` otherwise.
pub async fn require(
    db: &Db,
    user_id: &str,
    resource: &ResourceRef,
    perm: Permission,
) -> Result<(), AuthzError> {
    if effective_perms(db, user_id, resource).await?.contains(perm) {
        Ok(())
    } else {
        Err(AuthzError::Forbidden)
    }
}

/// Non-erroring convenience for list filtering: `true` iff the user has `perm`.
/// A resolution error resolves to `false` (fail closed).
pub async fn can(db: &Db, user_id: &str, resource: &ResourceRef, perm: Permission) -> bool {
    matches!(effective_perms(db, user_id, resource).await, Ok(p) if p.contains(perm))
}

/// A snapshot of everything a user can read, for filtering list/search queries
/// without a per-row ancestor walk. Precise for project/workspace membership and
/// direct grants; folder-level grants to non-members are still caught by the
/// full [`require`] path on single-resource access (defense in depth).
#[derive(Debug, Default, Clone)]
pub struct ReadableScope {
    pub superadmin: bool,
    pub workspaces: HashSet<String>,
    pub projects: HashSet<String>,
    pub granted_files: HashSet<String>,
    pub granted_folders: HashSet<String>,
    pub granted_projects: HashSet<String>,
    pub granted_workspaces: HashSet<String>,
}

impl ReadableScope {
    /// True when the user may view `file`.
    #[must_use]
    pub fn can_view_file(&self, file: &File) -> bool {
        if self.superadmin || self.granted_files.contains(&file.id) {
            return true;
        }
        if let Some(pid) = file.project_id.as_deref() {
            if self.projects.contains(pid) || self.granted_projects.contains(pid) {
                return true;
            }
        }
        if let Some(wid) = file.workspace_id.as_deref() {
            if self.workspaces.contains(wid) || self.granted_workspaces.contains(wid) {
                return true;
            }
        }
        false
    }

    /// True when the user may view `folder`.
    #[must_use]
    pub fn can_view_folder(&self, folder: &Folder) -> bool {
        if self.superadmin || self.granted_folders.contains(&folder.id) {
            return true;
        }
        if let Some(pid) = folder.project_id.as_deref() {
            if self.projects.contains(pid) || self.granted_projects.contains(pid) {
                return true;
            }
        }
        if let Some(wid) = folder.workspace_id.as_deref() {
            if self.workspaces.contains(wid) || self.granted_workspaces.contains(wid) {
                return true;
            }
        }
        false
    }
}

/// Compute the caller's readable scope (membership + direct grants). Used by
/// list/search handlers to ACL-filter candidate rows.
pub async fn readable_scope(db: &Db, user_id: &str) -> Result<ReadableScope, AuthzError> {
    let mut scope = ReadableScope {
        superadmin: is_superadmin(db, user_id).await?,
        ..ReadableScope::default()
    };

    for w in WorkspaceRepo::new(db).list_for_user(user_id).await? {
        scope.workspaces.insert(w.id);
    }
    for p in ProjectMemberRepo::new(db)
        .projects_for_user(user_id)
        .await?
    {
        scope.projects.insert(p);
    }
    for g in AclRepo::new(db).list_for_user_subject(user_id).await? {
        match g.resource_kind.as_str() {
            resource_kind::FILE => scope.granted_files.insert(g.resource_id),
            resource_kind::FOLDER => scope.granted_folders.insert(g.resource_id),
            resource_kind::PROJECT => scope.granted_projects.insert(g.resource_id),
            resource_kind::WORKSPACE => scope.granted_workspaces.insert(g.resource_id),
            _ => false,
        };
    }
    Ok(scope)
}
