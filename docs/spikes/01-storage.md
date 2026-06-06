# Spike #1 — Storage facade

Location: [`../../spikes/01-storage/`](../../spikes/01-storage/). Standalone Cargo project, not (yet) wired into the workspace.

## Goal

Prove the `Storage` facade shape from [`../ARCHITECTURE.md`](../ARCHITECTURE.md) compiles, that OpenDAL covers our needs cleanly, and that the conformance suite is a meaningful gate before Phase 1 hardens it into `crates/drive-storage`.

## Outcome

Green. 14/14 conformance tests pass against **fs** and **memory** backends. ~35 s clean build.

```
test result: ok. 14 passed; 0 failed
```

| Backend | put/get | stat/delete | list | copy/rename | signed_get | tamper-reject | invalid-key |
|---|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| fs | ✓ | ✓ | ✓ | ✓ (native) | ✓ (Token) | ✓ | ✓ |
| memory | ✓ | ✓ | ✓ | ✓ (synthesized) | ✓ (Token) | ✓ | ✓ |

S3 / MinIO via testcontainers: deferred to Phase 1 (the API surface is exercised by fs + memory; Docker on CI is its own decision).

## What worked

- **OpenDAL 0.54 is exactly what the brief said it was.** `Operator::new(services::Fs::default().root(...))?.finish()` is one line; the same shape works for `Memory`, `S3`. ~35 s from clean to a working `Operator` with all four services compiled in.
- **The `Capability` gate is the right primitive.** Branching on `op.info().full_capability().presign_read` makes the facade's `SignedUrl::Native` vs `SignedUrl::Token` split trivial — three lines of conditional, no `match` on backend type. Confirmed for `presign_read`, `copy`, `rename`. Same pattern scales to future gaps.
- **HMAC token path is small and fast.** ~30 LoC of `mint_token` + `verify_token`, including constant-time MAC compare (`subtle::ConstantTimeEq`) and `time::OffsetDateTime` expiry handling. No surprises.
- **Streaming reads via `into_bytes_stream` map cleanly into `BoxStream<Item = Result<Bytes, StorageError>>`.** Drops straight into Axum response bodies in Phase 1.

## What surprised

1. **OpenDAL's memory service doesn't support native `copy` or `rename`.** Calling `op.copy("a", "b")` returns `Unsupported (permanent)`. The first test run failed `mem_copy_rename` here.
   - **Fix landed in the facade:** `Storage::copy` / `Storage::rename` consult `Capability::copy` / `Capability::rename` and fall back to read-then-write (and read-then-write-then-delete for rename). Synthesized paths re-test clean.
   - **Implication for Phase 1:** the same pattern applies whenever a backend's capability is `false`. Don't surface "this op isn't supported" to callers — synthesize or fail with a typed error.

2. **`opendal::Buffer`'s `to_bytes()` is what you want for the fall-back read** — not `to_vec()` or `into_bytes()`. Spec read once and remembered now.

3. **`presign_read` on filesystem is exposed as `false`** as expected. The facade's `Token` branch covers it; no per-backend special-case in callers.

## What we didn't do (and why)

- **MinIO via testcontainers** — adds Docker as a hard dependency for the spike's CI. The fs + memory pair already exercises the whole `Storage` API and the capability-gate split. Phase 1's `crates/drive-storage/tests/` is where MinIO becomes mandatory.
- **S3 native presign verification** — same reason. The facade routes correctly to `SignedUrl::Native` when `presign_read` is true; the actual signature-format check is an OpenDAL test, not ours.
- **Multipart upload** — `put` takes `Bytes` not a stream. Phase 1 hardens this to `put_stream(key, BoxStream<Bytes>)` per the architecture doc; sketched but not in this spike since fs/memory don't expose multipart semantics anyway.
- **Pagination on `list`** — eager-load the whole listing. Phase 1 wires `OpenDAL`'s `lister_with(...).limit(N)` and threads `next_token`.

## Recommended revisions to ARCHITECTURE.md before Phase 1

1. **Document the copy/rename synthesis explicitly.** The `Storage::copy` / `Storage::rename` methods in the arch doc currently imply backend-native; the truth is "native when capable, synthesised otherwise". Worth a one-line note so callers don't assume atomicity guarantees they don't have on memory.
2. **The `put` signature should be `put_stream`** in Phase 1 (already stated in arch doc as the goal; spike kept it simple as `put(key, Bytes)`).
3. **Add `Capability` to the spike-cited API:** the facade exposes `fn capabilities(&self) -> opendal::Capability`. Worth promoting to a named method on the public surface so the WOPI handlers can branch on `Capability::write_can_append` when implementing PutFile.

## Files

- [`spikes/01-storage/Cargo.toml`](../../spikes/01-storage/Cargo.toml) — `opendal = "0.54"` + minimum deps
- [`spikes/01-storage/src/lib.rs`](../../spikes/01-storage/src/lib.rs) — `Storage`, `ObjectMeta`, `SignedUrl`, `StorageError`, `validate_key`, HMAC token mint/verify
- [`spikes/01-storage/tests/conformance.rs`](../../spikes/01-storage/tests/conformance.rs) — 14 tests across 2 backends

## Decision

**Greenlit.** The `Storage` facade shape from ARCHITECTURE.md survives contact with OpenDAL 0.54 with one (small, expected) addition: capability-gated synthesis for `copy`/`rename`. Move to Spike #2 (WOPI host).
