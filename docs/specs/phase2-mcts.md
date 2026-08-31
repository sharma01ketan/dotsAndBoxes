# Spec: Phase 2 local WASM MCTS (KET-19)

UCT Monte Carlo Tree Search in `dab-core`, played as P2 through the existing
Worker `chooseMove` path. Medium rung of the HUD ladder; Hard stays CGT.

Linear: [KET-19](https://linear.app/sharma01ketan/issue/KET-19/dotsandboxes-ai-local-mcts-engine-wasm).

Depends on: [`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) (KET-20 Worker),
[`phase2-random-greedy-engines.md`](./phase2-random-greedy-engines.md) (rollouts).

## Goals

- `MctsEngine` implements `Engine` (same contract as Random / Greedy / CGT).
- UCT; extra-turn stays the same player to move in the tree.
- Iteration budget (default 256). Deterministic given seed + budget.
- WASM `policy = 4`; HUD **Medium (MCTS)** (`vs-mcts`). Search stays in the Worker.
- Arena: beats Random on 2×2; beats Greedy on 2×2 (≥ 60% decisive); not crushed by CGT.

## Non-goals

- Wall-clock time budget in WASM (`Instant` is not reliable there).
- Neural policy/value (Phase 4).
- Perfect / CGT algorithm changes.
- Theory overlay (KET-21).
- A second Worker RPC — reuse `chooseMove(policy, seed)`.

## Search

Callers must not invoke `choose` on a terminal game. `Game` is `Copy`; the tree
stores copies. Ties: `XorShift64`.

1. If one legal move, return it.
2. If any legal move completes a box, take the max-completed (ties RNG).
   Greedy rollouts cannot represent double-cross, so a free box at the root
   is taken. Extra-turn still appears in the tree when a deeper expand captures.
3. Repeat `iterations` times:
   - **Select** — while a node is fully expanded and non-terminal, pick the
     child with max UCT. Value is always the **root player** score margin
     (`score_root − score_other`). If the node to move is not the root player,
     negate the mean (they minimize that margin). Extra-turn nodes keep the
     same to-move, so they do not negate.
   - **Expand** — play one untried legal edge (uniform among untried).
   - **Rollout** — `GreedyEngine` to terminal (fresh seed from the MCTS RNG).
   - **Backup** — add the terminal root-margin along the path.
4. Return the root child with the most visits (RNG on ties).

UCT: \(Q + \sqrt{2}\,\sqrt{\ln N / n}\).

## API

[`core/src/mcts.rs`](../../core/src/mcts.rs):

```rust
pub struct MctsEngine { /* rng + iterations */ }
impl MctsEngine {
    pub fn new(seed: u64) -> Self;           // 256 iterations
    pub fn with_iterations(self, n: u32) -> Self;
}
impl Engine for MctsEngine { ... }
```

| WASM | `chooseMove(policy=4, seed)` |
|------|------------------------------|
| `POLICY_MCTS` | `4` |
| `PlayMode` | `'vs-mcts'` |
| Label | Medium (MCTS) |
| Default | still vs Greedy |

## Acceptance

- [x] Extra-turn: a capturing child has the same `current_player` as its parent.
- [x] Same seed + same position ⇒ same chosen edge.
- [x] `chooseMove(4, seed)` legal and does not apply; Worker unchanged besides policy 4.
- [x] HUD Medium (MCTS) auto-plays P2 (browser check).
- [x] Arena MCTS vs Random 2×2 ≥ 65% decisive; vs Greedy ≥ 60%.
- [x] `cargo test -p dab-core`; `pnpm build:wasm`; web lint / tsc / test.

## Handoff

| Next | Uses this |
|------|-----------|
| [KET-21](https://linear.app/sharma01ketan/issue/KET-21) | Overlay on the same vs-AI board; Medium is a playable rung |

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-mcts.md` | This spec |
| `core/src/mcts.rs` | UCT engine |
| `wasm/src/lib.rs` | `POLICY_MCTS` |
| `web/src/game/store.ts` | `vs-mcts` mode |
| `web/src/lib/wasmGame.ts` | re-export |
