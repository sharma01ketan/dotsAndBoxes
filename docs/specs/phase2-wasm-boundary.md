# Spec: Phase 2 WASM boundary (KET-59)

The JS/WASM fence must not panic, must not cache a failed init forever, and
must not copy Perfect / policy numbers in TypeScript.

Linear: [KET-59](https://linear.app/sharma01ketan/issue/KET-59/dotsandboxes-wasm-range-check-boxowner-panic-hook-retry-init).

Depends on: [`wasm/src/lib.rs`](../../wasm/src/lib.rs),
[`phase2-exact-solver.md`](./phase2-exact-solver.md) (KET-18),
[`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) (KET-20 Worker).

## Why

[`Game::box_owner`](../../core/src/game.rs) calls [`Bitboard::get`](../../core/src/bitboard.rs)
with only a `debug_assert`. Box capacity is 128 bits. An id ≥ 128 indexes off
`words` and **panics in release WASM**, poisoning the instance.

WASM `boxOwner` maps `None` → −1 but does not check `box_id < box_count()`
first, so a large id never reaches that arm. Pixi only iterates
`0..boxCount()`, so this is a future-caller / `readBoxOwners` drift bug, not
a click bug.

`init_panic_hook` is an empty `#[wasm_bindgen(start)]`. `initWasm` (and the
Worker `ensureInit`) cache a **rejected** `init()` forever, so retry needs a
reload.

JS copies `POLICY_*` and `isPerfectHudSize`. A drift can call Perfect on 4×4,
throw, and hit the CPU-fail path (KET-57 recovers; the HUD should not call).

## Goals

- `boxOwner` range-checks at the WASM boundary; out of range → −1.
- Document the out-of-range contract for every WASM index API.
- `console_error_panic_hook` on start so a future panic is visible in the
  console.
- `initWasm` / Worker `ensureInit`: on rejection, clear the cached promise so
  the next call retries.
- Export `isPerfectHudSize` and `POLICY_*` from `@dab/dab-wasm`. JS imports
  them (no second copy).

## Non-goals

- Changing `Bitboard::get` to be release-safe. Core stays trusting; the
  binding is the fence.
- Silently falling back to CGT when Perfect is called on a bad size.
- 4×4 Perfect / per-move time budget.
- CI (KET-47). Recreating `server/` / `gpu/` / `ai/` (KET-61, later phases).

## WASM index contract

**Queries (soft — never throw):**

- `edgeIsDrawn(edge)` → `false` if `edge >= edgeCount`.
- `boxOwner(id)` → −1 if `id >= boxCount` **or** the box is unclaimed.
  Check `id >= box_count()` **before** `Game::box_owner`.
- `isLegal(edge)` → `false` when out of range (`edge < edge_count && !drawn`
  in core).

**Commands (hard — `Err` / throw):**

- `new(rows, cols)` → invalid-size error (existing).
- `play(edge)` → `MoveError` (`OutOfRange` / `AlreadyDrawn`).
- `edgeCoord(edge)` / `edgeId(...)` → existing out-of-range string.

**Search (hard, honest):**

- `chooseMove` / `perfectValue` → `Err` if terminal, unknown policy, or
  Perfect on a board that fails `is_perfect_hud_size(rows, cols)`.
- Do **not** play CGT and still say Perfect. HUD uses the exported size gate
  so it never calls. KET-57 remains the safety net if something still throws.

In-range unclaimed `boxOwner` stays −1; claimed is `0` (P1) or `1` (P2).

## Panic hook

Add `console_error_panic_hook` in [`wasm/Cargo.toml`](../../wasm/Cargo.toml).
`init_panic_hook` calls `console_error_panic_hook::set_once()`. Keep
`#[wasm_bindgen(start)]` so main-thread and Worker inits both install it.

## Init retry

Cache `init()` once. On rejection, clear the slot so the next call runs
`init()` again.

```ts
ready = load().then(
  () => undefined,
  (err) => {
    ready = null;
    throw err;
  },
);
```

Same helper for [`web/src/lib/wasmGame.ts`](../../web/src/lib/wasmGame.ts) and
[`web/src/game/aiWorker.ts`](../../web/src/game/aiWorker.ts).

## Constants

Export from WASM:

- `POLICY_RANDOM` = 0, `POLICY_GREEDY` = 1, `POLICY_CGT` = 2,
  `POLICY_PERFECT` = 3, `POLICY_MCTS` = 4
- `isPerfectHudSize(rows, cols)` → core `is_perfect_hud_size`

Re-export from `wasmGame.ts`. Store `isPerfectHudSize(size)` is
`isPerfectHudSize(size, size)` over the export. `policyForMode` uses the
imported `POLICY_*` values.

## Acceptance

- [x] `boxOwner(boxCount)` and `boxOwner(200)` return −1 and do not panic.
- [x] In-range unclaimed `boxOwner` is −1; after a claim, `0` or `1`.
- [x] `edgeIsDrawn(edgeCount)` is false.
- [x] `isPerfectHudSize(4, 4)` is false; `chooseMove(3)` / `perfectValue` on
  that board stay `Err` (honest — not a CGT fallback). Native tests cover the
  gate; `JsValue` Err is browser-only.
- [x] First `init` reject clears the cache; second call invokes `init` again.
- [x] Store / HUD do not define their own policy numbers or size-gate math.
- [x] `cargo test -p dab-wasm`; `pnpm --filter @dab/web lint`, `tsc`, `test`.
- [x] `pnpm build:wasm` so `wasm/pkg` matches source.

## Handoff

| Next | Uses this |
|------|-----------|
| KET-60 | Pixi can keep iterating `0..boxCount()`; a drifted id will not kill the tab |
| KET-47 | wasm-pkg diff still required after this crate change |
| KET-21 | Overlay reads the same range-checked owners |

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-wasm-boundary.md` | This spec |
| `wasm/src/lib.rs` | Range-check, exports, panic hook |
| `wasm/Cargo.toml` | `console_error_panic_hook` |
| `web/src/lib/wasmInit.ts` | Retryable `init()` cache (tested) |
| `web/src/lib/wasmGame.ts` | `initWasm` + policy / size re-exports |
| `web/src/game/aiWorker.ts` | Same init cache |
| `web/src/game/store.ts` | Import policy + size gate |
