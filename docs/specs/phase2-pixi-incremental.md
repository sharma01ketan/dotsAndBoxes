# Spec: Phase 2 Pixi incremental sync (KET-60)

Incremental `syncBoard` must paint claimed boxes from `snap.boxOwner`, and
must not rewire hover hits (or fire a second hover SFX) when the live hit
set is unchanged.

Linear: [KET-60](https://linear.app/sharma01ketan/issue/KET-60/dotsandboxes-web-pixi-incremental-sync-for-claimed-boxes-and-hover).

Depends on: [`phase1-board-polish.md`](./phase1-board-polish.md) (KET-14 motion),
[`web/src/board/PixiBoard.tsx`](../../web/src/board/PixiBoard.tsx),
[`web/src/board/edgeHoverSfx.ts`](../../web/src/board/edgeHoverSfx.ts).

## Why

Incremental sync rewires edges and animates `lastMove.boxIds`, but never
creates box gfx from `snap.boxOwner`. Boxes appear only on `fullRebuild` or
`animateBoxClaim`. A store update with claims and no matching `lastMove`
(stale AI loop, snap before `setLastMove`) shows correct scores and **blank
boxes** until the next `gameGeneration`.

`animateBoxClaim` no-ops if the gfx already exists, so filling from snap
*before* the lastMove tween would skip motion. Order: tween lastMove boxes
that are still missing, **then** instant-fill every other claimed box still
missing.

Incremental `wireEdgeHit` calls `removeAllListeners` on every paint and
never `hoverSfx.rebuild()`. After an `inputEnabled` flip with the pointer
still on an edge, Pixi can fire out/over and a second hover SFX plays.
`fullRebuild` already calls `hoverSfx.rebuild()`.

## Goals

- Incremental sync creates box gfx from `snap.boxOwner`. Never leave a
  claimed box blank.
- Claim tween only for boxes that are **missing** in `rt.boxes` **and**
  listed in this paint’s **new** `lastMove.boxIds`.
- All other claimed-and-missing boxes get instant `createBoxGfx` (final
  alpha/scale).
- Rewire hit listeners only when the live hit set would change (or the
  hover preview player changed). If any rewire happens, `hoverSfx.rebuild()`
  once.
- Reducer test: `rebuild` while `armed` → idle.
- Layout tests for H/V endpoints; motion tests for reduced-motion.

## Non-goals

- StrictMode `Application.destroy` mid-init (already encoded).
- Clearing claimed boxes on incremental (newGame still uses `gameGeneration`
  full rebuild).
- Overlay / MCTS / new SFX assets.
- A second Pixi store.

## Box paint

`snap.boxOwner` is the source of truth. `lastMove` is motion-only.

On the incremental branch, after edge strokes:

- `isNewMove` = `lastMove` is non-null and its `moveKey` is not
  `rt.lastMoveKey`.
- `animating` = `isNewMove ? lastMove.boxIds : []`.
- For each `id` with `snap.boxOwner[id] >= 0` and `!rt.boxes.has(id)`:
  - if `animating` contains `id` → existing `animateBoxClaim` (stagger
    unchanged)
  - else → `createBoxGfx` + add to layer (no tween)

If `lastMove` is null or already applied, every missing claimed box still
fills. Timings stay in [`phase1-board-polish.md`](./phase1-board-polish.md).

## Hover rewire

- `live = inputEnabled && !edge.drawn && !snap.isTerminal`.
- Skip `removeAllListeners` / re-bind when `eventMode`, `cursor`, and hover
  preview player would be unchanged.
- If any edge was rewired this paint, `hoverSfx.rebuild()` once.
- Reducer: enter → tick (`armed`) → rebuild → idle; a following enter can
  pending/play.

## Layout / motion tests

- `horizontalEdgeEnds` / `verticalEdgeEnds` on a known layout (cell 40,
  origin 10) match `dotPosition`.
- Mock `matchMedia('(prefers-reduced-motion: reduce)')` so `animate(180, …)`
  calls `onUpdate(1)` and `onDone` synchronously (no rAF).

## Acceptance

- [x] Incremental paint with claimed `snap.boxOwner` and `lastMove === null`
  still shows those boxes.
- [x] New `lastMove.boxIds` still tween (stagger unchanged) when gfx was
  missing.
- [x] Hover SFX does not double-play on `inputEnabled` flip with the pointer
  still on an edge (skip rewire or rebuild).
- [x] `rebuild` while `armed` → idle; next enter can play.
- [x] Layout H/V endpoint tests; reduced-motion `animate` test.
- [x] `pnpm --filter @dab/web lint`, `tsc`, `test`.

## Handoff

| Next | Uses this |
|------|-----------|
| KET-21 | Overlay draws on the same incremental box layer |
| KET-61 | Stub crates deleted; do not recreate until Phase 3–4 |

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-pixi-incremental.md` | This spec |
| `web/src/board/PixiBoard.tsx` | Incremental boxes + hit rewire |
| `web/src/board/edgeHoverSfx.ts` | `rebuild` while armed (already idle) |
| `web/src/board/edgeHoverSfx.test.ts` | Armed + rebuild case |
| `web/src/board/layout.ts` | H/V endpoints (tested) |
| `web/src/board/motion.ts` | Reduced-motion `animate` (tested) |
