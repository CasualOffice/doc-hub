//! Integration tests for the version-diff API (build spec §2 — P1.5):
//! `GET /api/files/{id}/diff?from=&to=`. Real in-memory SQLite + memory storage,
//! exercised end to end through the assembled router with a signed-in session.

use std::{net::SocketAddr, sync::Arc};

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use bytes::Bytes;
use dochub_auth::{hash_password, AuthState};
use dochub_core::{Backend, Config};
use dochub_db::{Db, NewUser, UserRepo};
use dochub_http::{router, HttpState};
use dochub_storage::Storage;
use dochub_wopi::WopiState;
use http_body_util::BodyExt;
use serde_json::Value;
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
            password_hash: hash_password("hunter2").unwrap(),
            is_admin: true,
        })
        .await
        .unwrap();
    UserRepo::new(&db)
        .insert(&NewUser {
            username: "bob".into(),
            password_hash: hash_password("bobpass").unwrap(),
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

/// Sign in, returning the `name=value` cookie pair.
async fn sign_in(app: &axum::Router, user: &str, pass: &str) -> String {
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/sign-in")
                .header("host", APP)
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"username":"{user}","password":"{pass}"}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let set_cookie = r
        .headers()
        .get("set-cookie")
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    set_cookie.split(';').next().unwrap().to_string()
}

fn auth_req(
    method: &str,
    path: &str,
    cookie: &str,
    content_type: Option<&str>,
    body: Body,
) -> Request<Body> {
    let mut b = Request::builder()
        .method(method)
        .uri(path)
        .header("host", APP)
        .header("cookie", cookie);
    if let Some(ct) = content_type {
        b = b.header("content-type", ct);
    }
    b.body(body).unwrap()
}

async fn json_body(r: axum::http::Response<Body>) -> Value {
    let bytes = r.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}

fn build_multipart(boundary: &str, filename: &str, content_type: &str, bytes: &[u8]) -> Bytes {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\
             Content-Type: {content_type}\r\n\r\n"
        )
        .as_bytes(),
    );
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n--");
    out.extend_from_slice(boundary.as_bytes());
    out.extend_from_slice(b"--\r\n");
    Bytes::from(out)
}

/// Upload a fresh file (seq=1) with the given name + content-type, returning id.
async fn upload(app: &axum::Router, cookie: &str, name: &str, ct: &str, bytes: &[u8]) -> String {
    let boundary = "----dbound";
    let body = build_multipart(boundary, name, ct, bytes);
    let r = app
        .clone()
        .oneshot(auth_req(
            "POST",
            "/api/files",
            cookie,
            Some(&format!("multipart/form-data; boundary={boundary}")),
            Body::from(body),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    json_body(r).await["id"].as_str().unwrap().to_string()
}

/// Commit a new head version via `PUT /content`.
async fn put_content(app: &axum::Router, cookie: &str, id: &str, bytes: &[u8]) {
    let r = app
        .clone()
        .oneshot(auth_req(
            "PUT",
            &format!("/api/files/{id}/content"),
            cookie,
            Some("application/octet-stream"),
            Body::from(bytes.to_vec()),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

async fn get_diff(
    app: &axum::Router,
    cookie: &str,
    id: &str,
    from: i64,
    to: i64,
) -> axum::http::Response<Body> {
    app.clone()
        .oneshot(auth_req(
            "GET",
            &format!("/api/files/{id}/diff?from={from}&to={to}"),
            cookie,
            None,
            Body::empty(),
        ))
        .await
        .unwrap()
}

/// A text file with a changed + added line diffs as `kind:"text"` with the
/// removed line in a `delete` hunk and the new lines in `insert` hunks.
#[tokio::test]
async fn text_diff_reports_insert_and_delete() {
    let app = router(fixture().await);
    let cookie = sign_in(&app, "admin", "hunter2").await;
    let id = upload(
        &app,
        &cookie,
        "doc.txt",
        "text/plain",
        b"alpha\nbeta\ngamma\n",
    )
    .await;
    put_content(&app, &cookie, &id, b"alpha\nBETA\ngamma\ndelta\n").await;

    let r = get_diff(&app, &cookie, &id, 1, 2).await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = json_body(r).await;
    assert_eq!(body["kind"], "text");
    assert_eq!(body["truncated"], false);

    let hunks = body["hunks"].as_array().unwrap();
    // The removed "beta" shows up in a delete hunk...
    assert!(
        hunks
            .iter()
            .any(|h| h["tag"] == "delete" && h["content"].as_str().unwrap().contains("beta")),
        "expected a delete hunk containing the removed line, got {hunks:?}"
    );
    // ...and the added "BETA"/"delta" lines in insert hunks.
    assert!(
        hunks
            .iter()
            .any(|h| h["tag"] == "insert" && h["content"].as_str().unwrap().contains("BETA")),
        "expected an insert hunk containing the changed line"
    );
    assert!(
        hunks
            .iter()
            .any(|h| h["tag"] == "insert" && h["content"].as_str().unwrap().contains("delta")),
        "expected an insert hunk containing the appended line"
    );
}

/// Identical bytes across two versions diff to all-equal — no insert/delete.
#[tokio::test]
async fn identical_text_has_no_changes() {
    let app = router(fixture().await);
    let cookie = sign_in(&app, "admin", "hunter2").await;
    let id = upload(
        &app,
        &cookie,
        "notes.md",
        "text/markdown",
        b"one\ntwo\nthree\n",
    )
    .await;
    put_content(&app, &cookie, &id, b"one\ntwo\nthree\n").await;

    let r = get_diff(&app, &cookie, &id, 1, 2).await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = json_body(r).await;
    assert_eq!(body["kind"], "text");
    let hunks = body["hunks"].as_array().unwrap();
    assert!(
        hunks.iter().all(|h| h["tag"] == "equal"),
        "identical content should produce only equal hunks, got {hunks:?}"
    );
}

/// A `.docx` (opaque) diffs as `kind:"binary"` with correct sizes and an
/// `identical` flag reflecting whether the decrypted bytes match.
#[tokio::test]
async fn binary_diff_reports_sizes_and_identical() {
    let app = router(fixture().await);
    let cookie = sign_in(&app, "admin", "hunter2").await;
    // Minimal ZIP magic so the upload sniff accepts it as an office document.
    let v1: &[u8] = b"PK\x03\x04docx-body-one";
    let v2: &[u8] = b"PK\x03\x04docx-body-two-longer";
    let id = upload(
        &app,
        &cookie,
        "sheet.docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        v1,
    )
    .await;
    put_content(&app, &cookie, &id, v2).await;

    // Differing versions → not identical, sizes reported.
    let r = get_diff(&app, &cookie, &id, 1, 2).await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = json_body(r).await;
    assert_eq!(body["kind"], "binary");
    assert_eq!(body["from_size"], v1.len());
    assert_eq!(body["to_size"], v2.len());
    assert_eq!(body["identical"], false);
    assert!(body.get("hunks").is_none(), "binary diff carries no hunks");

    // A version compared with itself is identical (nonce-randomized ciphertext
    // notwithstanding — equality is over decrypted plaintext).
    let r = get_diff(&app, &cookie, &id, 1, 1).await;
    let body = json_body(r).await;
    assert_eq!(body["kind"], "binary");
    assert_eq!(body["identical"], true);
    assert_eq!(body["from_size"], v1.len());
}

/// A `.txt` holding non-UTF-8 bytes falls through to the binary shape.
#[tokio::test]
async fn non_utf8_text_extension_is_binary() {
    let app = router(fixture().await);
    let cookie = sign_in(&app, "admin", "hunter2").await;
    let id = upload(&app, &cookie, "weird.txt", "text/plain", b"hello\n").await;
    // Invalid UTF-8 payload for the second version.
    put_content(&app, &cookie, &id, &[0xff, 0xfe, 0x00, 0x9c]).await;

    let r = get_diff(&app, &cookie, &id, 1, 2).await;
    assert_eq!(r.status(), StatusCode::OK);
    let body = json_body(r).await;
    assert_eq!(body["kind"], "binary");
    assert_eq!(body["from_size"], 6);
    assert_eq!(body["to_size"], 4);
    assert_eq!(body["identical"], false);
}

/// An unknown `seq` on either side is a 404.
#[tokio::test]
async fn unknown_seq_is_404() {
    let app = router(fixture().await);
    let cookie = sign_in(&app, "admin", "hunter2").await;
    let id = upload(
        &app,
        &cookie,
        "doc.txt",
        "text/plain",
        b"only one version\n",
    )
    .await;

    for (from, to) in [(1, 99), (99, 1)] {
        let r = get_diff(&app, &cookie, &id, from, to).await;
        assert_eq!(
            r.status(),
            StatusCode::NOT_FOUND,
            "from={from} to={to} → 404"
        );
    }
}

/// The diff surface is authenticated and owner-gated like the sibling version
/// endpoints: no session → 401, a different user → 403.
#[tokio::test]
async fn diff_enforces_auth_and_ownership() {
    let app = router(fixture().await);
    let owner = sign_in(&app, "admin", "hunter2").await;
    let id = upload(&app, &owner, "doc.txt", "text/plain", b"v1\n").await;
    put_content(&app, &owner, &id, b"v2\n").await;

    // Unauthenticated → 401.
    let r = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/files/{id}/diff?from=1&to=2"))
                .header("host", APP)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);

    // A different user → 403.
    let bob = sign_in(&app, "bob", "bobpass").await;
    let r = get_diff(&app, &bob, &id, 1, 2).await;
    assert_eq!(r.status(), StatusCode::FORBIDDEN);
}
