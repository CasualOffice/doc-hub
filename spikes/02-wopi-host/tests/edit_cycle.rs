//! End-to-end edit cycle: CheckFileInfo → GetFile → Lock → PutFile →
//! RefreshLock → Unlock. Plus the asymmetric 409 + X-WOPI-Lock contract,
//! the createnew 0-byte path, UnlockAndRelock, and token scoping.

use std::{collections::HashMap, sync::Arc};

use axum::{
    body::{Body, Bytes},
    http::{Method, Request, StatusCode},
};
use bytes::Bytes as RawBytes;
use http_body_util::BodyExt;
use spike_02_wopi_host::{
    AppState, CheckFileInfo, FileEntry, WopiClaims, WopiPerms, mint_token, router,
};
use tokio::sync::Mutex;
use tower::ServiceExt;

fn jwt_secret() -> [u8; 32] {
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(13);
    }
    k
}

async fn state_with(file_id: &str, name: &str, body: RawBytes) -> AppState {
    let mut files = HashMap::new();
    files.insert(
        file_id.into(),
        FileEntry {
            name: name.into(),
            bytes: Bytes::copy_from_slice(&body),
            version: 1,
            lock: None,
        },
    );
    AppState {
        files: Arc::new(Mutex::new(files)),
        jwt_secret: Arc::new(jwt_secret()),
    }
}

fn token_for(state: &AppState, file_id: &str, perms: WopiPerms) -> String {
    let exp = (time::OffsetDateTime::now_utc() + time::Duration::minutes(10)).unix_timestamp();
    mint_token(
        state,
        &WopiClaims {
            user_id: "user-1".into(),
            file_id: file_id.into(),
            perms,
            exp,
            jti: "spike".into(),
        },
    )
}

#[tokio::test]
async fn happy_path_full_edit_cycle() {
    let state = state_with("abc", "Budget.xlsx", RawBytes::from_static(b"v1")).await;
    let token = token_for(&state, "abc", WopiPerms::Write);
    let app = router(state.clone());

    // 1. CheckFileInfo
    let r = app.clone().oneshot(
        Request::builder()
            .uri(format!("/wopi/files/abc?access_token={token}"))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let body = r.into_body().collect().await.unwrap().to_bytes();
    let info: CheckFileInfo = serde_json::from_slice(&body).unwrap();
    assert_eq!(info.base_file_name, "Budget.xlsx");
    assert!(info.user_can_write);
    assert_eq!(info.size, 2);

    // 2. GetFile
    let r = app.clone().oneshot(
        Request::builder()
            .uri(format!("/wopi/files/abc/contents?access_token={token}"))
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers().get("x-wopi-itemversion").unwrap(), "1");
    let body = r.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(body.as_ref(), b"v1");

    // 3. Lock with a fresh ID
    let r = app.clone().oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(format!("/wopi/files/abc?access_token={token}"))
            .header("x-wopi-override", "LOCK")
            .header("x-wopi-lock", "lock-1")
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // 4. PutFile (with the lock)
    let r = app.clone().oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(format!("/wopi/files/abc/contents?access_token={token}"))
            .header("x-wopi-override", "PUT")
            .header("x-wopi-lock", "lock-1")
            .body(Body::from("v2!"))
            .unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers().get("x-wopi-itemversion").unwrap(), "2");

    // 5. RefreshLock
    let r = app.clone().oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(format!("/wopi/files/abc?access_token={token}"))
            .header("x-wopi-override", "REFRESH_LOCK")
            .header("x-wopi-lock", "lock-1")
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // 6. Unlock
    let r = app.clone().oneshot(
        Request::builder()
            .method(Method::POST)
            .uri(format!("/wopi/files/abc?access_token={token}"))
            .header("x-wopi-override", "UNLOCK")
            .header("x-wopi-lock", "lock-1")
            .body(Body::empty()).unwrap(),
    ).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn putfile_without_lock_returns_409_with_lock_header() {
    let state = state_with("z", "x.bin", RawBytes::from_static(b"hi")).await;
    let token = token_for(&state, "z", WopiPerms::Write);
    let app = router(state);

    // Lock it under "other"
    let _ = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/z?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "other")
        .body(Body::empty()).unwrap()).await.unwrap();

    // PutFile with mismatched lock
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/z/contents?access_token={token}"))
        .header("x-wopi-override", "PUT")
        .header("x-wopi-lock", "mine")
        .body(Body::from("nope"))
        .unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::CONFLICT);
    // Spec §4 — asymmetric 409 + X-WOPI-Lock header is mandatory.
    assert_eq!(r.headers().get("x-wopi-lock").unwrap(), "other");
}

#[tokio::test]
async fn happy_putfile_does_not_send_lock_header_back() {
    let state = state_with("z", "x.bin", RawBytes::from_static(b"hi")).await;
    let token = token_for(&state, "z", WopiPerms::Write);
    let app = router(state);

    let _ = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/z?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "mine")
        .body(Body::empty()).unwrap()).await.unwrap();

    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/z/contents?access_token={token}"))
        .header("x-wopi-override", "PUT")
        .header("x-wopi-lock", "mine")
        .body(Body::from("ok"))
        .unwrap()).await.unwrap();
    // The 200 path forbids the X-WOPI-Lock response header (spec §4).
    assert_eq!(r.status(), StatusCode::OK);
    assert!(r.headers().get("x-wopi-lock").is_none(),
            "X-WOPI-Lock must not appear on 200");
}

#[tokio::test]
async fn createnew_zero_byte_put_without_lock_allowed() {
    let state = state_with("e", "blank.txt", RawBytes::new()).await;
    let token = token_for(&state, "e", WopiPerms::Write);
    let app = router(state);
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/e/contents?access_token={token}"))
        .header("x-wopi-override", "PUT")
        .body(Body::empty())
        .unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn token_for_other_file_rejected() {
    let state = state_with("a", "n", RawBytes::from_static(b"x")).await;
    let _ = state.files.lock().await.insert("b".into(), FileEntry {
        name: "n2".into(),
        bytes: Bytes::from_static(b"x"),
        version: 1,
        lock: None,
    });
    let bad = token_for(&state, "a", WopiPerms::Write); // claims file_id=a
    let app = router(state);
    let r = app.clone().oneshot(Request::builder()
        .uri(format!("/wopi/files/b?access_token={bad}")) // URL claims b
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn read_token_cannot_putfile() {
    let state = state_with("r", "n", RawBytes::from_static(b"x")).await;
    let token = token_for(&state, "r", WopiPerms::Read);
    let app = router(state);
    // Acquiring lock with a read token is allowed in this spike (we don't
    // gate lock by perms — many editors grab the lock then probe perms via
    // CheckFileInfo).
    let _ = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/r?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "L")
        .body(Body::empty()).unwrap()).await.unwrap();
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/r/contents?access_token={token}"))
        .header("x-wopi-override", "PUT")
        .header("x-wopi-lock", "L")
        .body(Body::from("nope"))
        .unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn unlock_and_relock_atomic_swap() {
    let state = state_with("u", "n", RawBytes::from_static(b"x")).await;
    let token = token_for(&state, "u", WopiPerms::Write);
    let app = router(state);
    let _ = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/u?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "old-id")
        .body(Body::empty()).unwrap()).await.unwrap();

    // LOCK + X-WOPI-OldLock present → dispatched to UnlockAndRelock.
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/u?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-oldlock", "old-id")
        .header("x-wopi-lock", "new-id")
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);

    // PutFile with the new lock now works
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/u/contents?access_token={token}"))
        .header("x-wopi-override", "PUT")
        .header("x-wopi-lock", "new-id")
        .body(Body::from("ok"))
        .unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}

#[tokio::test]
async fn lock_with_same_id_acts_as_refresh() {
    let state = state_with("k", "n", RawBytes::from_static(b"x")).await;
    let token = token_for(&state, "k", WopiPerms::Write);
    let app = router(state);
    let _ = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/k?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "L")
        .body(Body::empty()).unwrap()).await.unwrap();
    // Calling Lock again with the SAME id must succeed (spec: treat as refresh).
    let r = app.clone().oneshot(Request::builder()
        .method(Method::POST)
        .uri(format!("/wopi/files/k?access_token={token}"))
        .header("x-wopi-override", "LOCK")
        .header("x-wopi-lock", "L")
        .body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(r.status(), StatusCode::OK);
}
