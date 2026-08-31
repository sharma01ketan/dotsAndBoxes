# Spec: Phase 2 hint button (KET-22)

Optional assistance: suggest a legal edge from the same engine the HUD already
uses. Search stays in the Worker.

Linear: [KET-22](https://linear.app/sharma01ketan/issue/KET-22/dotsandboxes-web-hint-system-suggest-best-move).

Depends on: [`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) (KET-20 Worker).

## Goals

- **Hint** queries Worker `chooseMove` and does **not** `play`.
- Policy = `policyForMode(mode)`. Opponent (`hotseat`): CGT. vs Perfect: Perfect
  (disabled off 2×2/3×3).
- Highlight the suggested undrawn edge (current-player color).
- Bound to `gameGeneration` like `runAiTurn`.

## Non-goals

- Applying the move.
- A second search RPC or main-thread `chooseMove`.
- Overlay rationale beyond “this is what this engine would play.”
- AI as P1.

## UI

- Hint button in the HUD; disabled while terminal, `aiBusy`, or Perfect on a
  gated size.
- Pixi: `hintEdgeId` prop. Clear on play, new game, mode/size change, or a
  second hint that returns a different edge (replace). A second click of Hint
  on the same position may replace the highlight with a new choose (same seed
  ⇒ same edge).
- Clear `hintEdgeId` in App when `gameGeneration` or `lastMove` changes.

## Acceptance

- [ ] Hint edge is legal and does not apply.
- [ ] vs Greedy hint matches `POLICY_GREEDY` choose (same seed path as CPU).
- [ ] Worker-only search; abort/stale generation does not apply a highlight
      after New game.
- [ ] Web lint / tsc / test.

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-hints.md` | This spec |
| `web/src/game/store.ts` | `hintPolicyForMode` (hotseat → CGT) |
| `web/src/App.tsx` | Hint button + clear |
| `web/src/board/PixiBoard.tsx` | highlight |
