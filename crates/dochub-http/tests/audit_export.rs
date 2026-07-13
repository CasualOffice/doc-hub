//! Integration tests for `GET /api/admin/audit/export` — the admin-only,
//! hash-verifiable audit log export (Phase 4 compliance: "an exported audit
//! report is complete and hash-verifiable").

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dochub_auth::{hash_password, AuthState};
use dochub_core::{Backend, Config};
use dochub_db::{
    action, verify_exported_chain, AuditChainStatus, AuditExport, AuditRepo, Db, NewAuditEvent,
    NewUser, UserRepo,
};
use dochub_http::{router, HttpState};
use dochub_storage::Storage;
use dochub_wopi::WopiState;
use http_body_util::BodyExt;
use tower::ServiceExt;
use url::Url;

const APP: &str = "drive.test";
const UCN: &str = "usercontent-drive.test";

async fn fixture() -> HttpState {
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
    UserRepo::new(&db)
        .insert(&NewUser {
            username: "bob".into(),
            password_hash: hash_password("hunter2hunter2").unwrap(),
            is_admin: false,
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

async fn seed_audit(state: &HttpState) {
    let repo = AuditRepo::new(&state.db);
    for a in [
        action::VERSION_COMMIT,
        action::FILE_TOMBSTONE,
        action::PII_SCAN,
    ] {
        repo.insert(NewAuditEvent {
            actor_id: Some("u_actor".into()),
            actor_username: Some("actor".into()),
            action: a.into(),
            target_kind: Some("file".into()),
            target_id: Some("F_1".into()),
            target_name: Some("Q3.xlsx".into()),
            ip_address: Some("10.0.0.1".into()),
            metadata: None,
        })
        .await
        .unwrap();
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

async fn export(app: &axum::Router, cookie: &str) -> (StatusCode, Option<String>, Vec<u8>) {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/admin/audit/export")
                .header("host", APP)
                .header("cookie", cookie)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = r.status();
    let disposition = r
        .headers()
        .get(axum::http::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);
    let bytes = r.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, disposition, bytes)
}

#[tokio::test]
async fn admin_exports_a_hash_verifiable_report() {
    let state = fixture().await;
    seed_audit(&state).await;

    let app = router(state);
    let cookie = sign_in_as(&app, "admin").await;
    let (status, disposition, bytes) = export(&app, &cookie).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        disposition.as_deref(),
        Some("attachment; filename=\"audit-export.json\"")
    );

    let report: AuditExport = serde_json::from_slice(&bytes).unwrap();
    // Complete: at least the three seeded events (sign-in may add more).
    assert!(report.count >= 3);
    assert_eq!(report.count, report.events.len());
    assert_eq!(report.chain_status, "intact");
    // Hash-verifiable: re-walk the chain offline, exactly as `verify-audit` does.
    assert_eq!(
        verify_exported_chain(&report.events),
        AuditChainStatus::Intact
    );

    // Tampering any exported field is caught by the offline re-verification.
    let mut tampered = report.events.clone();
    tampered[0].action = "version.forged".into();
    assert!(matches!(
        verify_exported_chain(&tampered),
        AuditChainStatus::Broken { .. }
    ));
}

#[tokio::test]
async fn non_admin_is_forbidden() {
    let state = fixture().await;
    let app = router(state);
    let cookie = sign_in_as(&app, "bob").await;
    let (status, _, _) = export(&app, &cookie).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unauthenticated_is_rejected() {
    let state = fixture().await;
    let app = router(state);
    let (status, _, _) = export(&app, "").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
