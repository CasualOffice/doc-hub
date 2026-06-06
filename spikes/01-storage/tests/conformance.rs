//! Conformance suite. Same set of cases runs against fs + memory.
//! MinIO via testcontainers is deferred until Phase 1 (Docker on CI is its own
//! decision; the spike's job is to confirm the API surface).

use std::time::Duration;

use bytes::Bytes;
use futures::TryStreamExt;
use spike_01_storage::{SignedUrl, Storage, StorageError};
use tempfile::TempDir;

fn key() -> [u8; 32] {
    let mut k = [0u8; 32];
    for (i, b) in k.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(7);
    }
    k
}

enum Backend {
    Fs(TempDir),
    Memory,
}

fn make(backend: &Backend) -> Storage {
    match backend {
        Backend::Fs(td) => Storage::fs(td.path().to_string_lossy().into_owned(), key()).unwrap(),
        Backend::Memory => Storage::memory(key()).unwrap(),
    }
}

async fn put_get_roundtrip(b: Backend) {
    let s = make(&b);
    let body = Bytes::from_static(b"hello world");
    let meta = s.put("dir/file.txt", body.clone(), None).await.unwrap();
    assert_eq!(meta.size, body.len() as u64);
    assert_eq!(meta.key, "dir/file.txt");

    let (got_meta, stream) = s.get("dir/file.txt", None).await.unwrap();
    assert_eq!(got_meta.size, body.len() as u64);
    let chunks: Vec<Bytes> = stream.try_collect().await.unwrap();
    let total: Vec<u8> = chunks.into_iter().flatten().collect();
    assert_eq!(total, body.as_ref());
}

async fn stat_then_delete(b: Backend) {
    let s = make(&b);
    s.put("stat-me", Bytes::from_static(b"x"), None).await.unwrap();
    let m = s.stat("stat-me").await.unwrap();
    assert_eq!(m.size, 1);
    s.delete("stat-me").await.unwrap();
    let err = s.stat("stat-me").await.unwrap_err();
    assert!(matches!(err, StorageError::NotFound(_)), "got {err:?}");
}

async fn list_returns_entries(b: Backend) {
    let s = make(&b);
    for i in 0..3 {
        s.put(&format!("prefix/{i}.txt"), Bytes::from(vec![i as u8; 4]), None)
            .await
            .unwrap();
    }
    let page = s.list("prefix/", None).await.unwrap();
    let keys: Vec<&str> = page.entries.iter().map(|e| e.key.as_str()).collect();
    for i in 0..3 {
        assert!(keys.iter().any(|k| k.ends_with(&format!("{i}.txt"))),
                "missing {i}, got {keys:?}");
    }
}

async fn copy_then_rename(b: Backend) {
    let s = make(&b);
    s.put("src.txt", Bytes::from_static(b"hi"), None).await.unwrap();
    s.copy("src.txt", "copy.txt").await.unwrap();
    assert_eq!(s.stat("src.txt").await.unwrap().size, 2);
    assert_eq!(s.stat("copy.txt").await.unwrap().size, 2);
    s.rename("copy.txt", "renamed.txt").await.unwrap();
    assert!(matches!(
        s.stat("copy.txt").await.unwrap_err(),
        StorageError::NotFound(_)
    ));
    assert_eq!(s.stat("renamed.txt").await.unwrap().size, 2);
}

async fn signed_get_round_trip(b: Backend) {
    let s = make(&b);
    s.put("signed.txt", Bytes::from_static(b"sig"), None).await.unwrap();
    let url = s.signed_get("signed.txt", Duration::from_secs(60)).await.unwrap();
    match url {
        SignedUrl::Native { .. } => panic!("fs/memory should NOT have native presign"),
        SignedUrl::Token { token, expires_at } => {
            assert!(expires_at > time::OffsetDateTime::now_utc());
            let (key, method) = s.verify_token(&token).unwrap();
            assert_eq!(key, "signed.txt");
            assert_eq!(method, "GET");
        }
    }
}

async fn signed_token_rejects_tamper(b: Backend) {
    let s = make(&b);
    s.put("t.txt", Bytes::from_static(b"x"), None).await.unwrap();
    let SignedUrl::Token { token, .. } =
        s.signed_get("t.txt", Duration::from_secs(60)).await.unwrap()
    else { panic!("expected Token variant") };
    // Tamper the last char.
    let mut bad = token.clone();
    let last_idx = bad.len() - 1;
    let ch = bad.chars().last().unwrap();
    let new_ch = if ch == 'A' { 'B' } else { 'A' };
    bad.replace_range(last_idx..last_idx + 1, &new_ch.to_string());
    let err = s.verify_token(&bad).unwrap_err();
    assert!(matches!(err, StorageError::InvalidToken), "got {err:?}");
}

async fn invalid_keys_rejected(b: Backend) {
    let s = make(&b);
    for bad in ["", "../escape", "/abs", "back\\slash", "null\0byte"] {
        let err = s.put(bad, Bytes::from_static(b"x"), None).await.unwrap_err();
        assert!(matches!(err, StorageError::InvalidKey(_)),
                "expected InvalidKey for {bad:?}, got {err:?}");
    }
}

// --- entry points ---

#[tokio::test]
async fn fs_put_get()                     { put_get_roundtrip(fs_backend()).await; }
#[tokio::test]
async fn mem_put_get()                    { put_get_roundtrip(Backend::Memory).await; }
#[tokio::test]
async fn fs_stat_delete()                 { stat_then_delete(fs_backend()).await; }
#[tokio::test]
async fn mem_stat_delete()                { stat_then_delete(Backend::Memory).await; }
#[tokio::test]
async fn fs_list()                        { list_returns_entries(fs_backend()).await; }
#[tokio::test]
async fn mem_list()                       { list_returns_entries(Backend::Memory).await; }
#[tokio::test]
async fn fs_copy_rename()                 { copy_then_rename(fs_backend()).await; }
#[tokio::test]
async fn mem_copy_rename()                { copy_then_rename(Backend::Memory).await; }
#[tokio::test]
async fn fs_signed_get_token()            { signed_get_round_trip(fs_backend()).await; }
#[tokio::test]
async fn mem_signed_get_token()           { signed_get_round_trip(Backend::Memory).await; }
#[tokio::test]
async fn fs_signed_tamper_rejected()      { signed_token_rejects_tamper(fs_backend()).await; }
#[tokio::test]
async fn mem_signed_tamper_rejected()     { signed_token_rejects_tamper(Backend::Memory).await; }
#[tokio::test]
async fn fs_invalid_keys()                { invalid_keys_rejected(fs_backend()).await; }
#[tokio::test]
async fn mem_invalid_keys()               { invalid_keys_rejected(Backend::Memory).await; }

fn fs_backend() -> Backend {
    Backend::Fs(TempDir::new().expect("tempdir"))
}
