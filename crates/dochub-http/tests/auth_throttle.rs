//! Integration test for password sign-in brute-force throttling. Its own test
//! binary, so the process-global login throttle is isolated from other suites.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use dochub_auth::{hash_password, AuthState};
use dochub_core::{Backend, Config};
use dochub_db::{Db, NewUser, UserRepo};
use dochub_http::{router, HttpState};
use dochub_storage::Storage;
use dochub_wopi::WopiState;
use tower::ServiceExt;
use url::Url;

const APP: &str = "drive.test";
const UCN: &str = "usercontent-drive.test";

async fn fixture() -> HttpState {
    let storage = Storage::memory([1u8; 32]).unwrap();
    let db = Db::connect("sqlite::memory:").await.unwrap();
    // Two users with distinct usernames so the two tests (which share this
    // binary's process-global throttle, keyed by username) can't collide
    // regardless of run order.
    for u in ["target", "target2"] {
        UserRepo::new(&db)
            .insert(&NewUser {
                username: u.into(),
                password_hash: hash_password("correct-horse-battery").unwrap(),
                is_admin: false,
            })
            .await
            .unwrap();
    }
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

async fn sign_in(app: &axum::Router, username: &str, password: &str) -> StatusCode {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("host", APP)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{username}","password":"{password}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap()
        .status()
}

#[tokio::test]
async fn repeated_bad_passwords_are_throttled_with_429() {
    let app = router(fixture().await);

    // The first 5 wrong-password attempts are refused as invalid (401).
    for _ in 0..5 {
        assert_eq!(
            sign_in(&app, "target", "wrong").await,
            StatusCode::UNAUTHORIZED
        );
    }
    // The 6th is throttled (429) — refused before the hash verify even runs.
    assert_eq!(
        sign_in(&app, "target", "wrong").await,
        StatusCode::TOO_MANY_REQUESTS
    );
    // Even the CORRECT password is refused while throttled: a pre-verify gate
    // stops an attacker's lucky guess landing on the attempt that trips the cap.
    assert_eq!(
        sign_in(&app, "target", "correct-horse-battery").await,
        StatusCode::TOO_MANY_REQUESTS
    );
}

#[tokio::test]
async fn a_few_failures_then_the_right_password_still_signs_in() {
    let app = router(fixture().await);

    // Distinct user from the throttling test (own bucket). Two typos (well
    // under the cap of 5) then the correct password — still under the cap, so
    // the correct password signs in (200) and clears the bucket.
    assert_eq!(
        sign_in(&app, "target2", "nope").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sign_in(&app, "target2", "nope").await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        sign_in(&app, "target2", "correct-horse-battery").await,
        StatusCode::OK
    );
}
