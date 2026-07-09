//! DB-backed resolution tests for `dochub-authz`: effective-permission
//! inheritance, most-specific/union, deny-by-default, and superadmin bypass.
//! Spec: docs/design/foundation-access-rag-mcp.md §3; test contract
//! docs/TESTING.md ("authz property tests: no privilege escalation,
//! deny-by-default, tenant isolation").

use dochub_authz::{effective_perms, readable_scope, require, Permission, ResourceRef};
use dochub_db::{
    resource_kind, subject_kind, AclRepo, Db, FileRepo, NewAclGrant, NewFile, NewUser,
    ProjectMemberRepo, ProjectRepo, UserRepo, WorkspaceKind, WorkspaceMemberRepo, WorkspaceRepo,
};

async fn db() -> Db {
    Db::connect("sqlite::memory:").await.expect("connect")
}

async fn make_user(db: &Db, name: &str, is_admin: bool) -> String {
    UserRepo::new(db)
        .insert(&NewUser {
            username: name.into(),
            password_hash: "$argon2id$x".into(),
            is_admin,
        })
        .await
        .expect("user")
        .id
}

/// A team workspace owned by `owner_id`, with its default project + a file.
/// Returns (workspace_id, project_id, file_id).
async fn seed_ws_project_file(db: &Db, owner_id: &str) -> (String, String, String) {
    let ws = WorkspaceRepo::new(db)
        .insert("Team", WorkspaceKind::Team, owner_id)
        .await
        .expect("ws")
        .id;
    let project = ProjectRepo::new(db)
        .ensure_default(&ws)
        .await
        .expect("proj");
    let file_id = ulid::Ulid::new().to_string();
    FileRepo::new(db)
        .insert(&NewFile {
            id: file_id.clone(),
            name: "doc.txt".into(),
            owner_id: owner_id.to_string(),
            workspace_id: ws.clone(),
            project_id: Some(project.clone()),
            ..NewFile::default()
        })
        .await
        .expect("file");
    (ws, project, file_id)
}

#[tokio::test]
async fn deny_by_default_for_a_stranger() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let stranger = make_user(&db, "stranger", false).await;
    let (_ws, _p, file) = seed_ws_project_file(&db, &owner).await;

    let perms = effective_perms(&db, &stranger, &ResourceRef::File(file.clone()))
        .await
        .unwrap();
    assert!(perms.is_empty(), "a non-member gets no permissions");
    assert!(matches!(
        require(&db, &stranger, &ResourceRef::File(file), Permission::View).await,
        Err(dochub_authz::AuthzError::Forbidden)
    ));
}

#[tokio::test]
async fn owner_has_full_permissions_on_own_documents() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let (_ws, _p, file) = seed_ws_project_file(&db, &owner).await;

    let perms = effective_perms(&db, &owner, &ResourceRef::File(file))
        .await
        .unwrap();
    for p in [
        Permission::View,
        Permission::Edit,
        Permission::Delete,
        Permission::Share,
        Permission::ManageRetention,
    ] {
        assert!(perms.contains(p), "owner should have {p:?}");
    }
}

#[tokio::test]
async fn workspace_viewer_can_view_but_not_edit() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let viewer = make_user(&db, "viewer", false).await;
    let (ws, _p, file) = seed_ws_project_file(&db, &owner).await;
    WorkspaceMemberRepo::new(&db)
        .add(&ws, &viewer, "viewer")
        .await
        .unwrap();

    let perms = effective_perms(&db, &viewer, &ResourceRef::File(file))
        .await
        .unwrap();
    assert!(perms.contains(Permission::View));
    assert!(!perms.contains(Permission::Edit));
    assert!(!perms.contains(Permission::Delete));
}

#[tokio::test]
async fn project_editor_can_edit() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let editor = make_user(&db, "editor", false).await;
    let (_ws, project, file) = seed_ws_project_file(&db, &owner).await;
    ProjectMemberRepo::new(&db)
        .set_role(&project, &editor, "editor")
        .await
        .unwrap();

    let perms = effective_perms(&db, &editor, &ResourceRef::File(file))
        .await
        .unwrap();
    assert!(perms.contains(Permission::Edit));
    assert!(perms.contains(Permission::Delete));
    assert!(!perms.contains(Permission::ManageMembers));
}

#[tokio::test]
async fn grant_at_project_is_inherited_by_child_file() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let guest = make_user(&db, "guest", false).await;
    let (_ws, project, file) = seed_ws_project_file(&db, &owner).await;

    // A viewer grant on the *project* must be visible on its child file.
    AclRepo::new(&db)
        .grant(&NewAclGrant {
            resource_kind: resource_kind::PROJECT.into(),
            resource_id: project,
            subject_kind: subject_kind::USER.into(),
            subject_id: guest.clone(),
            role: "viewer".into(),
            created_by: owner,
        })
        .await
        .unwrap();

    let perms = effective_perms(&db, &guest, &ResourceRef::File(file))
        .await
        .unwrap();
    assert!(perms.contains(Permission::View), "inherited from project");
    assert!(!perms.contains(Permission::Edit));
}

#[tokio::test]
async fn union_of_membership_and_file_grant_is_most_permissive() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let user = make_user(&db, "user", false).await;
    let (ws, _p, file) = seed_ws_project_file(&db, &owner).await;
    // Workspace viewer …
    WorkspaceMemberRepo::new(&db)
        .add(&ws, &user, "viewer")
        .await
        .unwrap();
    // … plus an editor grant on this one file.
    AclRepo::new(&db)
        .grant(&NewAclGrant {
            resource_kind: resource_kind::FILE.into(),
            resource_id: file.clone(),
            subject_kind: subject_kind::USER.into(),
            subject_id: user.clone(),
            role: "editor".into(),
            created_by: owner,
        })
        .await
        .unwrap();

    let perms = effective_perms(&db, &user, &ResourceRef::File(file))
        .await
        .unwrap();
    assert!(perms.contains(Permission::View));
    assert!(perms.contains(Permission::Edit), "union grants edit");
}

#[tokio::test]
async fn superadmin_bypasses_everything() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let root = make_user(&db, "root", true).await; // is_admin
    let (ws, _p, file) = seed_ws_project_file(&db, &owner).await;

    let perms = effective_perms(&db, &root, &ResourceRef::File(file))
        .await
        .unwrap();
    assert_eq!(perms, dochub_authz::PermSet::all());
    // Superadmin also passes require on a workspace they're not a member of.
    assert!(require(
        &db,
        &root,
        &ResourceRef::Workspace(ws),
        Permission::ManageKeys
    )
    .await
    .is_ok());
}

#[tokio::test]
async fn readable_scope_reflects_membership_and_grants() {
    let db = db().await;
    let owner = make_user(&db, "owner", false).await;
    let guest = make_user(&db, "guest", false).await;
    let (ws, _p, file) = seed_ws_project_file(&db, &owner).await;

    // Owner: member of the workspace → the file is readable.
    let owner_scope = readable_scope(&db, &owner).await.unwrap();
    let f = FileRepo::new(&db).find_by_id(&file).await.unwrap();
    assert!(owner_scope.can_view_file(&f));

    // Guest: no membership → not readable until a direct grant lands.
    let guest_scope = readable_scope(&db, &guest).await.unwrap();
    assert!(!guest_scope.can_view_file(&f));
    assert!(!guest_scope.workspaces.contains(&ws));

    AclRepo::new(&db)
        .grant(&NewAclGrant {
            resource_kind: resource_kind::FILE.into(),
            resource_id: file.clone(),
            subject_kind: subject_kind::USER.into(),
            subject_id: guest.clone(),
            role: "viewer".into(),
            created_by: owner,
        })
        .await
        .unwrap();
    let guest_scope = readable_scope(&db, &guest).await.unwrap();
    assert!(
        guest_scope.can_view_file(&f),
        "direct file grant is readable"
    );
}
