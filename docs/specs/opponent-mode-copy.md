# Spec: Mode copy — Hotseat → Opponent

User-facing rename of the local two-human mode. Behavior unchanged.

## Goals

- Everywhere the **web mode picker / titles** said **Hotseat**, say **Opponent**.
- Keep the store/WASM id `PlayMode = 'hotseat'` (stable API; rename is copy-only).
- Default mode is **vs Greedy** (`vs-greedy`).

## Non-goals

- First-load opponent modal (separate follow-up).
- Renaming CLI playground, Phase 1 file names, or Linear ticket titles.
- Changing who sits where (still two humans on one screen for this mode).

## Copy map

| Surface | Before | After |
|---------|--------|--------|
| Mode `<select>` option | Hotseat | Opponent |
| Page title (`modeTitle`) | Hotseat | Opponent |
| Mode table / vs-AI UI spec | Hotseat | Opponent |
| Confirm modal | uses `PLAY_MODES[].label` | Opponent (automatic) |
| Score / turn | P1 / P2 | unchanged |
| Lede | Two players, one screen… | unchanged |

## Acceptance

- [x] Mode dropdown shows **Opponent**, not Hotseat.
- [x] Selecting Opponent still uses `mode === 'hotseat'` and local P1/P2 play.
- [x] Title when in that mode is **Opponent**.
- [x] [`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) documents Opponent as the label.
- [x] Lint/typecheck clean for web.
