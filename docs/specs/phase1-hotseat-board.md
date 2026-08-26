# Spec: Phase 1 hotseat board (KET-12 + KET-13)

Playable local **hotseat** Dots and Boxes in the browser: PixiJS board + WASM rules.

## Goals

- Two humans, one screen.
- Rules/scoring/extra-turn from `@dab/dab-wasm` (same engine as CLI).
- Click/tap undrawn edges to play; HUD shows turn, scores, winner.

## Non-goals

- AI opponents, multiplayer, theory overlay, auth.
- Motion polish details live in [`phase1-board-polish.md`](./phase1-board-polish.md) (KET-14).

## Mode & board size

| Setting | Value |
|---------|--------|
| Mode | Local hotseat only |
| Default size | **3×3** boxes |
| Allowed sizes | 2–5 boxes per side (square boards) |
| Size change | “New game” with selected size; resets state |

## Coordinate contract

Identical to `core` / CLI / WASM:

- Horizontal edges first (row-major), then vertical.
- `edgeCoord(id) → [orientation, row, col]` with `0 = H`, `1 = V`.
- `edgeId(orient, row, col)`, `edgeIsDrawn`, `boxOwner` (`-1` / `0` / `1`).
- `play(edge)` → `[extraTurn, completedCount, ...boxIds]`.

The board **never** invents rules; it only renders snapshots and emits `edgeId` clicks.

## Visual model

Colors align with the existing cream / terracotta UI (`--ink`, `--accent`, `--muted`, `--ok`):

| Element | Look |
|---------|------|
| Background | Transparent over page gradient |
| Dots | Filled circles, `--ink` |
| Undrawn edge | Faint line / wide invisible hit pad; hover brightens (desktop) |
| Drawn by P1 | Stroke `--ok` (green) |
| Drawn by P2 | Stroke `--accent` (terracotta) |
| Claimed box | Soft fill + label `1` or `2` |
| Hover (legal) | Current player’s stroke color (P1 `--ok`, P2 `--accent`) |

**Drawn-by:** WASM does not store who drew an edge. The client store records the current player when `play` succeeds.

## Interaction

1. Pointer down/up (click or tap) on a **legal undrawn** edge → `store.play(edgeId)`.
2. Drawn, illegal, or out-of-range edges: no-op.
3. While terminal: board ignores edge clicks; HUD shows win/draw.
4. Hit pads ≥ ~24px for touch.

## HUD

- **Mute** toggle first in tab order (sticky via `localStorage`).
- Brand / title.
- Scores: P1 / P2.
- Current turn (or “Game over”).
- Board size control (2–5) + **New game**.
- Status line (last move / error / extra turn).
- Win/draw banner when `winner !== -1`.

## Sound (soft UI pops)

Uses [`use-sound`](https://github.com/joshwcomeau/use-sound) + Freesound CC0 samples
(see [`web/public/sounds/ATTRIBUTION.md`](../../web/public/sounds/ATTRIBUTION.md)).

| Event | Behavior |
|-------|----------|
| Edge hover | Soft pop; **leading-edge throttle** (~140ms) so scrubbing doesn’t spam |
| Edge draw | Soft pop on successful `play` |
| Box claim | Slightly brighter pop (higher playback rate) |
| Win / tie | Success cue after a short delay so claim/draw can be heard |
| New game | Soft click |

Mute silences all cues; game remains fully playable without audio.
Architecture: store returns `PlayOutcome`; `useGameSounds` maps outcomes → SFX;
Pixi only emits `onEdgeHover` / `onEdgeClick`.

## Layout (Pixi)

- Responsive: board fits container width (max ~480px), square cell grid.
- Dot grid: `(rows+1) × (cols+1)`.
- Cell size derived from available width/height with padding.
- Rebuild graphics when snapshot size or edge/box state changes; resize observer rebuilds layout.

## Architecture

```
App
├── useGameStore → WasmGame (rules)
├── useGameSounds → use-sound (SFX)
└── PixiBoard (render + hit-test only)
```

`play()` returns a `PlayOutcome`; App maps that to SFX. Pixi has no rules or audio.

Audio lives in two files: `sounds.ts` (catalog + pure helpers) and `useGameSounds.ts` (hook + mute).

## Acceptance

- [ ] `pnpm build:wasm && pnpm install && pnpm --filter @dab/web build` succeeds.
- [ ] `pnpm --filter @dab/web lint` clean.
- [ ] `pnpm dev`: loading → playable 3×3 board (not edge-id button grid).
- [ ] Clicking edges updates board, scores, and turn; capture grants extra turn.
- [ ] Game reaches terminal; winner/draw shown; New game resets.
- [ ] Changing size (2–5) starts a fresh board of that size.
- [ ] Mute toggle works and persists; hover SFX is throttled; win/claim/new-game cues fire.

## Files

| Path | Role |
|------|------|
| `docs/specs/phase1-hotseat-board.md` | This spec |
| `web/src/game/store.ts` | Zustand + WASM |
| `web/src/board/layout.ts` | Pixel geometry |
| `web/src/board/PixiBoard.tsx` | Pixi scene |
| `web/src/board/motion.ts` | rAF tweens (KET-14) |
| `web/src/audio/sounds.ts` | SFX catalog + pure helpers |
| `web/src/audio/useGameSounds.ts` | `use-sound` hook + mute state |
| `web/public/sounds/` | CC0 MP3s + attribution |
| `web/src/App.tsx` | HUD shell |
| `web/src/lib/wasmGame.ts` | WASM init helpers |
