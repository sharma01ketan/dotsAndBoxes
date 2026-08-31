# Spec: Phase 2 CGT heuristic engine (KET-17)

Classic **Medium** engine: keep chain control with double-cross (all-but-two) and
all-but-four on loops. Uses KET-16 analysis; does not solve.

Linear: [KET-17](https://linear.app/sharma01ketan/issue/KET-17/dotsandboxes-ai-cgt-heuristic-engine-with-double-cross-all-but-two).

Depends on: [`phase2-cgt-endgame-analysis.md`](./phase2-cgt-endgame-analysis.md) (KET-16).

## Goals

- `CgtEngine` implements `Engine` (same contract as Random/Greedy).
- Double-cross: decline the last two boxes of a long chain when another control
  region remains.
- All-but-four: decline emptying a 4-loop while another long chain/loop remains.
- Midgame: greedy capture/safe ladder. Parity is overlay-only (KET-21); it
  does not steer `CgtEngine`.
- Playable in the browser: WASM `policy = 2`, HUD **Hard (CGT)** (id `vs-cgt`). Default stays vs Greedy.

## Non-goals

- Exact solver / Perfect → [`phase2-exact-solver.md`](./phase2-exact-solver.md) (KET-18).
- MCTS → [KET-19](https://linear.app/sharma01ketan/issue/KET-19).
- Theory overlay → [KET-21](https://linear.app/sharma01ketan/issue/KET-21).
- Loony nimber / control-value tables beyond this heuristic.
- Changing Random/Greedy policy.

## Policy (exact order)

Callers must not invoke `choose` on a terminal game. Probe on a **copy** of
`Game`. Ties: uniform via `XorShift64`.

1. **Captures exist**
   - If **refuse** (below), open another remaining long chain or loop (not the
     remnant attached to the takeable). If no such opening edge, fall through.
   - Else take a capturing move that **maximizes** completed boxes (same as Greedy).
2. **Decomposed, no capture** — open the shortest remaining region: long chains
   first (by length), then loops, then short chains.
3. **Midgame, no capture** — Greedy safe moves (do not leave a 3-sided box).
   If every remaining move gives a 3-sided box, dump a formed corridor (shortest
   region first) instead of a random gift. Do **not** steer by `parity_ok` here:
   partial corridors make `L%2` a noisy target and it loses to Greedy in arena.

When **forced to open** (no capture): dump the smallest gift — shortest region
first (short chains before long chains). When **refusing a capture** (control):
prefer shortest other long chain, then loop, then short chain, skipping remnant `R`.

### When to refuse a capture

Takeable boxes are unclaimed with 3 sides. They are **not** region members.
`R` is the unique degree-2 (corridor) neighbor’s region, if any.

After opening a long chain of 3, analysis shows 1 takeable + a **short chain of
length 2**. After opening a 4-loop, 2 takeables + a short chain of 2.

Refuse iff:

- `R` is a short chain of length **2**, and `long_chain_count + loop_count > 0`
  (another control region still exists), or
- `R` is a **loop of length 4**, and another long chain exists or there is more
  than one loop.

Otherwise take (short chains of 1–2, finishing the last region, long corridors
still longer than 2).

### Opening a region

- **Chain:** a **ground** string — undrawn edge of a region box that does **not**
  touch another box in that region.
- **Loop:** an undrawn **internal** string (between two boxes in the loop).

Prefer shortest other long chain, then shortest loop, then a short chain.
When refusing, skip `R` (the remnant).

## API

[`core/src/engine.rs`](../../core/src/engine.rs):

```rust
pub struct CgtEngine { /* XorShift64 */ }
impl Engine for CgtEngine { ... }
```

Helpers in [`core/src/cgt.rs`](../../core/src/cgt.rs): region attached to a
takeable, opening edges. No heap in `EndgameAnalysis`; engines may `Vec` for
legal-move lists (same as Greedy).

**KET-58 remnant.** `refuse_remnant(game, analysis) -> Option<Region>` uses the
**same** predicate as refuse (short-2 with another control region, or 4-loop
with another long/loop). `should_refuse_capture` is `.is_some()`.
`refuse_skip_region` is a thin wrapper. `CgtEngine::choose` analyzes once and
threads `&EndgameAnalysis`. Opening edges that would complete a box
(`completed_count > 0`) are dropped so refuse cannot capture while “opening.”

| Contract | Detail |
|----------|--------|
| Input | `&Game` |
| Output | Legal `EdgeId` |
| Mutation | None on caller |
| WASM | `chooseMove(policy=2, seed)` |

## WASM / HUD

| Piece | Value |
|-------|--------|
| `POLICY_CGT` | `2` |
| `PlayMode` | `'vs-cgt'` |
| Label | Hard (CGT) (was vs CGT; id `vs-cgt`) |
| Title | You vs Hard (CGT) |
| Score P2 | CPU (Hard) |
| Default mode | Unchanged: vs Greedy |

HUD copy for Hard (CGT) ships with [`phase2-exact-solver.md`](./phase2-exact-solver.md) (KET-18).

## Acceptance

- [x] Opened long chain of 3 + a second long chain: does **not** take the last two; opens the other chain.
- [x] Same with only one region left: **does** take.
- [x] Opened 4-loop + a long chain: refuses to take the loop takeables; opens the chain.
- [x] Midgame: still takes a free box when one exists.
- [x] Same seed + position ⇒ same edge.
- [x] Arena vs Greedy, 3×3, ≥200 games, seats swapped: CGT win rate **≥ 60%** of decisive games.
- [x] `chooseMove(2, seed)` legal and does not apply; HUD vs CGT auto-plays P2.
- [x] `cargo test -p dab-core` and web typecheck pass.
- [ ] KET-58: two takeables / takeable touching two regions — skip is the remnant that triggered refuse; opening does not complete a box.

## Handoff

| Next | Uses this |
|------|-----------|
| [KET-18](./phase2-exact-solver.md) | Perfect search; CGT for **move ordering only**, never values |
| KET-20 | Worker for Perfect / `chooseMove` — shipped |
| KET-21 | Overlay can sit on the same Hard (CGT) board |

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-cgt-heuristic.md` | This spec |
| `core/src/cgt.rs` | Opening / refuse helpers |
| `core/src/engine.rs` | `CgtEngine` |
| `core/src/lib.rs` | Re-export |
| `wasm/src/lib.rs` | `policy = 2` |
| `web/src/game/store.ts` | `vs-cgt` mode |
