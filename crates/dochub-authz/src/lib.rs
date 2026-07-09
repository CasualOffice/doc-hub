//! Doc-Hub access control: RBAC roles + permission matrix (`perms`) and
//! per-resource ACL resolution + enforcement (`resolve`).
//!
//! Spec: docs/design/foundation-access-rag-mcp.md §1–§3. Deny-by-default,
//! least privilege, ACL inheritance down the tree, superadmin bypass. Handlers
//! call [`require`] instead of ad-hoc owner/role checks, and filter lists with
//! [`readable_scope`].

#![forbid(unsafe_code)]

mod perms;
mod resolve;

pub use perms::{role_permissions, PermSet, Permission, Role};
pub use resolve::{
    can, effective_perms, readable_scope, require, AuthzError, ReadableScope, ResourceRef,
};
