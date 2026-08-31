# Spec: Phase 1 board polish (KET-14)

Soft motion for the hotseat Pixi board. Rules stay in WASM; SFX/HUD already ship in
[`phase1-hotseat-board.md`](./phase1-hotseat-board.md).

## Goals

- Edge-draw and box-claim motion that feels physical and calm (cream / terracotta palette).
- Multi-box captures read as a short chain (staggered).
- ~60fps on a mid-range laptop up to 5×5.
- Honor `prefers-reduced-motion: reduce` (instant final state).

## Non-goals

- Arcade confetti, heavy glow, particle systems as the main effect.
- AI, multiplayer, theory overlay, new audio assets, HUD redesign.

## Already done (do not re-scope)

- Local hotseat + Pixi board (KET-12 / KET-13).
- Soft UI SFX + sticky mute (`use-sound`).
- New game, board size, scores, win/draw banner.

## Motion model

Driven by `PlayOutcome` from `store.play` (includes `edgeId`, `mover`, `boxIds`).

| Trigger | Motion |
|---------|--------|
| Edge hover | Full-length player-colored stroke at reduced opacity (~0.4) |
| Edge draw | Full stroke fades opacity up (~180ms); no grow-along-segment; color = mover |
| Box claim | Soft fill fades + scales 0.92→1.0 ~250ms; label fades in |
| Multi-box claim | Stagger claims ~70ms apart |
| Extra turn | No extra VFX (claim motion + SFX suffice) |
| Win | Soft pulse (~200ms) on winner’s claimed boxes; HUD banner unchanged |
| Tie | No board fireworks |
| New game / resize | Cancel tweens; rebuild board instantly |

**Reduced motion:** skip tweens; show final stroke/fill immediately.

## Architecture

```
App --PlayOutcome--> PixiBoard --rAF--> edge/box display objects
         ^
   useGameStore / WasmGame
```

- Store stays free of Pixi; enrich `PlayOutcome` only.
- Prefer **incremental** edge/box updates so tweens are not destroyed every move.
  Claimed boxes come from `snap.boxOwner`; `lastMove` is motion-only
  ([`phase2-pixi-incremental.md`](./phase2-pixi-incremental.md)).
- Full rebuild on new game / board size change / container resize.
- One small helper: `web/src/board/motion.ts` (`easeOutCubic`, `animate`).

## Acceptance

- [ ] Draw tween visible on each successful edge.
- [ ] Single- and multi-box claims animate with stagger.
- [ ] Input remains snappy; hover/click SFX behavior unchanged.
- [ ] `prefers-reduced-motion: reduce` snaps to final art.
- [ ] 5×5 play stays smooth (~60fps feel).
- [ ] `pnpm --filter @dab/web lint` + `build` green.

## Files

| Path | Role |
|------|------|
| `docs/specs/phase1-board-polish.md` | This spec |
| `web/src/board/motion.ts` | rAF tween helper |
| `web/src/board/PixiBoard.tsx` | Incremental gfx + motion |
| `web/src/game/store.ts` | `PlayOutcome.edgeId` / `boxIds` / `mover` |
| `web/src/App.tsx` | Pass `lastMove` into board |
