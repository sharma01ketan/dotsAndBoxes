# Spec: Phase 2 CGT theory overlay (KET-21)

Teaching HUD that visualizes chains, loops, takeables, and long-chain parity
from core analysis. The board does not reimplement CGT.

Linear: [KET-21](https://linear.app/sharma01ketan/issue/KET-21/dotsandboxes-web-cgt-theory-overlay-chainsloopsparitycontrol).

Depends on: [`phase2-cgt-endgame-analysis.md`](./phase2-cgt-endgame-analysis.md) (KET-16).

## Goals

- WASM-export `analyze_endgame()` as a compact `u16` list.
- Toggle overlay: tint unclaimed corridor boxes by region kind.
- HUD line: long-chain count, target parity, whether the side to move has it.
- Updates live after `play` / `newGame`. Matches core analysis.

## Non-goals

- Steering `CgtEngine` by parity (KET-17 / KET-58).
- Worker RPC for analysis (≤25 boxes; not search).
- Loony nimber / control-value tables.
- Hint button → [`phase2-hints.md`](./phase2-hints.md) (KET-22).

## WASM dump

`WasmGame.analyze()` → `Vec<u16>`:

```
header: decomposed, L, short_count, loop_count, takeable_count,
        long_parity, target_parity, region_count
each region: kind (0 short, 1 long, 2 loop), length, n, ...boxIds
then: takeable box ids (count = takeable_count)
```

Range-safe; does not mutate the game.

## UI

- Toggle in App (default off). Not a second Zustand store.
- Store holds `analysis` only when overlay is on; refresh with the snapshot.
- Pixi: unclaimed boxes get a light region tint; claimed boxes stay owner fill.
- Copy: `L=n · target even/odd · you have / lack this parity`.

## Acceptance

- [ ] Dump matches `analyze_endgame` membership + counts on a 1×3 chain and a 2×2 loop.
- [ ] Overlay off: board unchanged.
- [ ] Overlay on: live update after a move; Pixi does not invent legality.
- [ ] `pnpm build:wasm`; web lint / tsc / test.

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-theory-overlay.md` | This spec |
| `wasm/src/lib.rs` | `analyze()` |
| `web/src/game/store.ts` | `analysis` snapshot |
| `web/src/App.tsx` | toggle + HUD line |
| `web/src/board/PixiBoard.tsx` | region tints |
