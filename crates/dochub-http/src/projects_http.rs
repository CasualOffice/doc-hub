//! Projects — list + create access containers inside a workspace.
//! Spec: docs/design/foundation-access-rag-mcp.md §3 (F2a). Minimal surface:
//!
//!   - `GET  /api/workspaces/{id}/projects`  — list (readable-scoped: `view`)
//!   - `POST /api/workspaces/{id}/projects`  — create (`manage_settings`)
//!
//! Listing is gated by `view` on the workspace, so a member (who inherits the
//! workspace role on every project) sees the workspace's projects and a
//! non-member is denied. Create is an admin/settings op.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use dochub_auth::AuthSession;
use dochub_authz::{Permission, ResourceRef};
use dochub_db::{AuditRepo, NewAuditEvent, NewProject, ProjectRepo, WorkspaceRepo};
use serde::{Deserialize, Serialize};

use crate::authz::gate;
use crate::HttpState;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ProjectError {
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("validation: {0}")]
    Validation(String),
    #[error("internal: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrBody<'a> {
    error: &'a str,
}

impl From<dochub_authz::AuthzError> for ProjectError {
    fn from(e: dochub_authz::AuthzError) -> Self {
        match e {
            dochub_authz::AuthzError::Forbidden => Self::Forbidden,
            dochub_authz::AuthzError::Db(err) => Self::Internal(err.to_string()),
        }
    }
}

impl IntoResponse for ProjectError {
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
            Self::Internal(m) => {
                tracing::error!(error = %m, "projects handler error");
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

#[derive(Serialize)]
struct ProjectDto {
    id: String,
    workspace_id: String,
    name: String,
    kind: String,
    created_at: String,
}

#[derive(Serialize)]
struct ListResp {
    projects: Vec<ProjectDto>,
}

async fn list_projects(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(workspace_id): Path<String>,
) -> Result<Json<ListResp>, ProjectError> {
    gate(
        &s,
        &session,
        ResourceRef::Workspace(workspace_id.clone()),
        Permission::View,
    )
    .await?;

    let projects = ProjectRepo::new(&s.db)
        .list_for_workspace(&workspace_id)
        .await
        .map_err(|e| ProjectError::Internal(e.to_string()))?
        .into_iter()
        .map(|p| ProjectDto {
            id: p.id,
            workspace_id: p.workspace_id,
            name: p.name,
            kind: p.kind,
            created_at: rfc3339(p.created_at),
        })
        .collect();
    Ok(Json(ListResp { projects }))
}

#[derive(Deserialize)]
struct CreateBody {
    name: String,
}

async fn create_project(
    State(s): State<HttpState>,
    session: AuthSession,
    Path(workspace_id): Path<String>,
    Json(body): Json<CreateBody>,
) -> Result<(StatusCode, Json<ProjectDto>), ProjectError> {
    gate(
        &s,
        &session,
        ResourceRef::Workspace(workspace_id.clone()),
        Permission::ManageSettings,
    )
    .await?;

    let name = sanitise_name(&body.name)?;
    let ws = WorkspaceRepo::new(&s.db)
        .find_by_id(&workspace_id)
        .await
        .map_err(|_| ProjectError::NotFound)?;
    // Mirror the workspace kind onto the project (team | personal).
    let kind = match ws.kind {
        dochub_db::WorkspaceKind::Personal => "personal",
        dochub_db::WorkspaceKind::Team => "team",
    };

    let project = ProjectRepo::new(&s.db)
        .insert(&NewProject {
            workspace_id: workspace_id.clone(),
            name: name.clone(),
            kind: kind.into(),
        })
        .await
        .map_err(|e| ProjectError::Internal(e.to_string()))?;

    AuditRepo::emit(
        &s.db,
        NewAuditEvent {
            actor_id: Some(session.user_id.clone()),
            actor_username: Some(session.username.clone()),
            action: "project.create".into(),
            target_kind: Some("project".into()),
            target_id: Some(project.id.clone()),
            target_name: Some(project.name.clone()),
            ip_address: None,
            metadata: Some(format!(r#"{{"workspace_id":"{workspace_id}"}}"#)),
        },
    );

    Ok((
        StatusCode::CREATED,
        Json(ProjectDto {
            id: project.id,
            workspace_id: project.workspace_id,
            name: project.name,
            kind: project.kind,
            created_at: rfc3339(project.created_at),
        }),
    ))
}

fn sanitise_name(s: &str) -> Result<String, ProjectError> {
    let t = s.trim();
    if t.chars().count() < 2 || t.chars().count() > 60 {
        return Err(ProjectError::Validation(
            "project name must be 2–60 characters".into(),
        ));
    }
    Ok(t.to_string())
}

fn rfc3339(t: time::OffsetDateTime) -> String {
    t.format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
}

pub(crate) fn router(state: HttpState) -> Router {
    Router::new()
        .route(
            "/api/workspaces/{workspace_id}/projects",
            get(list_projects).post(create_project),
        )
        .with_state(state)
}
