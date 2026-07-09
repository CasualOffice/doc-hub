//! Integration tests for `GET /api/files/{id}/summary` — Phase 3 P3.5 on-demand
//! document summary. Real sqlite + in-memory storage + the deterministic
//! **mock** AI provider (no network in CI).
//!
//! Covers: a summary returns for a text doc; an `ai.summary` audit row is
//! written with the model; a second call is a cache hit (no second audit /
//! provider call); AI-off ⇒ disabled (409); unauth ⇒ 401; other-user ⇒ 403; and
//! the read-only invariant (the hash chain length is unchanged after summarize).

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dochub_auth::{hash_password, AuthState};
use dochub_core::{AiConfig, Backend, Config};
use dochub_db::{
    AuditRepo, Db, FileRepo, FileVersionsRepo, NewFile, NewUser, Registry, UserRepo, WorkspaceDeks,
    WorkspaceKind, WorkspaceRepo,
};
use dochub_http::{router, HttpState};
use dochub_storage::Storage;
use dochub_wopi::WopiState;
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt;
use url::Url;

const APP: &str = "drive.test";
const UCN: &str = "usercontent-drive.test";

async fn fixture(ai: AiConfig) -> HttpState {
    let storage = Storage::memory([1u8; 32]).unwrap();
    let db = Db::connect("sqlite::memory:").await.unwrap();
    UserRepo::new(&db)
        .insert(&NewUser {
            username: "admin".into(),
            password_hash: hash_password("hunter2hunter2").unwrap(),
            is_admin: true,
        })
        .await
        .unwrap();
    let cfg = Config {
        app_origin: Url::parse(&format!("http://{APP}")).unwrap(),
        usercontent_origin: Url::parse(&format!("http://{UCN}")).unwrap(),
        bind: "127.0.0.1:0".parse::<SocketAddr>().unwrap(),
        backend: Backend::Memory,
        fs_root: None,
        s3_bucket: None,
        s3_region: None,
        s3_endpoint: None,
        aws_access_key_id: None,
        aws_secret_access_key: None,
        db_url: "sqlite::memory:".into(),
        body_limit_mb: 100,
        signed_url_ttl_secs: 300,
        oidc: None,
        allow_password_auth: true,
        session_secret: vec![0u8; 32],
        wopi_hmac_secret: [2u8; 32],
        signed_url_hmac_secret: [1u8; 32],
        admin_user: "admin".into(),
        admin_password_hash: "$argon2id$test".into(),
        recipient_footer: true,
        is_prod: false,
        sheet_origin: None,
        document_origin: None,
        collab_url: None,
        master_kek: dochub_core::dev_master_kek(),
        ai,
        master_kek_next: None,
    };
    let auth = AuthState::new(db.clone(), false, time::Duration::hours(1));
    let registry = HttpState::default_registry(storage.clone(), [0u8; 32]);
    HttpState {
        storage,
        wopi: WopiState::new(),
        db,
        auth,
        jwt_secret: Arc::new([2u8; 32]),
        config: Arc::new(cfg),
        upload_limiter: HttpState::default_upload_limiter(),
        registry,
        storage_secret_key: None,
        presence: dochub_http::presence::PresenceHub::new(),
    }
}

async fn sign_in_as(app: &axum::Router, user: &str) -> String {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("host", APP)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{user}","password":"hunter2hunter2"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    r.headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn user_id(state: &HttpState, username: &str) -> String {
    UserRepo::new(&state.db)
        .find_by_username(username)
        .await
        .unwrap()
        .id
}

async fn personal_ws(state: &HttpState, user_id: &str) -> String {
    WorkspaceRepo::new(&state.db)
        .list_for_user(user_id)
        .await
        .unwrap()
        .into_iter()
        .find(|w| matches!(w.kind, WorkspaceKind::Personal))
        .expect("seeded user must have a Personal workspace")
        .id
}

async fn make_file(state: &HttpState, ws: &str, owner: &str, name: &str, content: &[u8]) -> String {
    let id = ulid::Ulid::new().to_string();
    FileRepo::new(&state.db)
        .insert(&NewFile {
            id: id.clone(),
            parent_id: None,
            name: name.into(),
            size: content.len() as u64,
            content_type: None,
            etag: None,
            owner_id: owner.into(),
            workspace_id: ws.into(),
            storage_id: None,
            status: dochub_db::FileStatus::Ready,
            expected_size: None,
        })
        .await
        .unwrap();
    let deks = WorkspaceDeks::new(state.db.clone(), state.config.master_kek.clone());
    let registry = Registry::new(state.db.clone(), state.storage.clone(), deks);
    registry
        .commit_version(ws, &id, content, owner, "test upload")
        .await
        .unwrap();
    id
}

async fn get_summary(app: &axum::Router, cookie: Option<&str>, id: &str) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .uri(format!("/api/files/{id}/summary"))
        .header("host", APP);
    if let Some(c) = cookie {
        b = b.header("cookie", c);
    }
    let r = app
        .clone()
        .oneshot(b.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = r.status();
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

async fn ai_summary_audit_count(state: &HttpState) -> usize {
    AuditRepo::new(&state.db)
        .list_filtered(&["ai.summary"], 100)
        .await
        .unwrap()
        .len()
}

#[tokio::test]
async fn summary_returns_for_text_doc_and_audits() {
    let state = fixture(AiConfig::mock()).await;
    let owner = user_id(&state, "admin").await;
    let ws = personal_ws(&state, &owner).await;
    let id = make_file(
        &state,
        &ws,
        &owner,
        "notes.md",
        b"First sentence about the roadmap. Second sentence with detail. Third sentence closes it. A fourth is dropped.",
    )
    .await;

    let app = router(state.clone());
    let cookie = sign_in_as(&app, "admin").await;

    let (status, body) = get_summary(&app, Some(&cookie), &id).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["cached"], Value::Bool(false));
    assert_eq!(body["model"].as_str().unwrap(), "mock-summarizer-v1");
    // Deterministic mock: first three sentences.
    assert_eq!(
        body["summary"].as_str().unwrap(),
        "First sentence about the roadmap. Second sentence with detail. Third sentence closes it."
    );
    assert!(body["input_tokens"].as_u64().unwrap() > 0);
    assert!(body["output_tokens"].as_u64().unwrap() > 0);

    // An ai.summary audit row was written, carrying the model.
    let rows = AuditRepo::new(&state.db)
        .list_filtered(&["ai.summary"], 100)
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].target_id.as_deref(), Some(id.as_str()));
    assert!(rows[0]
        .metadata
        .as_deref()
        .unwrap()
        .contains("mock-summarizer-v1"));
}

#[tokio::test]
async fn second_call_is_a_cache_hit_no_new_audit() {
    let state = fixture(AiConfig::mock()).await;
    let owner = user_id(&state, "admin").await;
    let ws = personal_ws(&state, &owner).await;
    let id = make_file(&state, &ws, &owner, "notes.md", b"Only sentence here.").await;

    let app = router(state.clone());
    let cookie = sign_in_as(&app, "admin").await;

    let (s1, b1) = get_summary(&app, Some(&cookie), &id).await;
    assert_eq!(s1, StatusCode::OK);
    assert_eq!(b1["cached"], Value::Bool(false));
    assert_eq!(ai_summary_audit_count(&state).await, 1);

    // Second call for the same head hash: cache hit — no provider call, no new
    // audit row, identical summary.
    let (s2, b2) = get_summary(&app, Some(&cookie), &id).await;
    assert_eq!(s2, StatusCode::OK);
    assert_eq!(b2["cached"], Value::Bool(true));
    assert_eq!(b2["summary"], b1["summary"]);
    assert_eq!(b2["model"], b1["model"]);
    assert_eq!(
        ai_summary_audit_count(&state).await,
        1,
        "cache hit must not write a second audit row"
    );
}

#[tokio::test]
async fn ai_off_is_disabled() {
    let state = fixture(AiConfig::disabled()).await;
    let owner = user_id(&state, "admin").await;
    let ws = personal_ws(&state, &owner).await;
    let id = make_file(&state, &ws, &owner, "notes.md", b"anything at all here.").await;

    let app = router(state.clone());
    let cookie = sign_in_as(&app, "admin").await;

    let (status, body) = get_summary(&app, Some(&cookie), &id).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["error"].as_str().unwrap(), "ai disabled");
    // Nothing was audited.
    assert_eq!(ai_summary_audit_count(&state).await, 0);
}

#[tokio::test]
async fn unauthenticated_is_401() {
    let state = fixture(AiConfig::mock()).await;
    let owner = user_id(&state, "admin").await;
    let ws = personal_ws(&state, &owner).await;
    let id = make_file(&state, &ws, &owner, "notes.md", b"private content here.").await;

    let app = router(state);
    let (status, _) = get_summary(&app, None, &id).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn other_user_is_forbidden() {
    let state = fixture(AiConfig::mock()).await;
    UserRepo::new(&state.db)
        .insert(&NewUser {
            username: "other".into(),
            password_hash: hash_password("hunter2hunter2").unwrap(),
            is_admin: false,
        })
        .await
        .unwrap();

    let admin = user_id(&state, "admin").await;
    let admin_ws = personal_ws(&state, &admin).await;
    let id = make_file(
        &state,
        &admin_ws,
        &admin,
        "notes.md",
        b"admin only content.",
    )
    .await;

    let app = router(state);
    let other_cookie = sign_in_as(&app, "other").await;
    let (status, _) = get_summary(&app, Some(&other_cookie), &id).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn summarize_is_read_only_chain_unchanged() {
    let state = fixture(AiConfig::mock()).await;
    let owner = user_id(&state, "admin").await;
    let ws = personal_ws(&state, &owner).await;
    let id = make_file(&state, &ws, &owner, "notes.md", b"Body one. Body two.").await;

    let before = FileVersionsRepo::new(&state.db)
        .list_chain(&id)
        .await
        .unwrap()
        .len();

    let app = router(state.clone());
    let cookie = sign_in_as(&app, "admin").await;
    let (status, _) = get_summary(&app, Some(&cookie), &id).await;
    assert_eq!(status, StatusCode::OK);

    let after = FileVersionsRepo::new(&state.db)
        .list_chain(&id)
        .await
        .unwrap()
        .len();
    assert_eq!(
        before, after,
        "summarize must not append or mutate versions"
    );
}
