# Spec: Phase 2 random + greedy engines (KET-15)

Baseline opponents in **`dab-core`**: Random and Greedy move engines over the
existing rules API. Linear: [KET-15](https://linear.app/sharma01ketan/issue/KET-15/dotsandboxes-ai-random-greedy-engines).

These are rungs 1–2 of the difficulty ladder in [`PLAN.md`](../../PLAN.md) §6.1.

## Goals

- **Random** — uniform legal move (seeded, deterministic).
- **Greedy** — take free boxes; otherwise avoid handing a box when a safe move exists.
- Engines live in Rust core; arena tests prove greedy consistently beats random.
- Callers pass `&Game`; engines never invent legality (only select among `legal_moves`).

## Non-goals

- Web UI, WASM `chooseMove`, difficulty ladder wiring → [KET-20](https://linear.app/sharma01ketan/issue/KET-20).
- CGT heuristic / double-cross → [KET-17](https://linear.app/sharma01ketan/issue/KET-17).
- Exact solver / Perfect → [KET-18](https://linear.app/sharma01ketan/issue/KET-18).
- Local MCTS → [KET-19](https://linear.app/sharma01ketan/issue/KET-19).
- Theory overlay → [KET-21](https://linear.app/sharma01ketan/issue/KET-21).
- CLI `--p2 random|greedy` flags (nice-to-have; not required for Done).

## Module & API

New module [`core/src/engine.rs`](../../core/src/engine.rs), re-exported from
[`core/src/lib.rs`](../../core/src/lib.rs):

```rust
pub trait Engine {
    /// Returns a legal `EdgeId`. Caller must not invoke on a terminal game.
    fn choose(&mut self, game: &Game) -> EdgeId;
}

pub struct RandomEngine { /* seeded PRNG */ }
pub struct GreedyEngine { /* same PRNG for ties */ }
```

| Contract | Detail |
|----------|--------|
| Input | Immutable `&Game` |
| Output | Always a **legal** `EdgeId` |
| Terminal | Undefined / panic — callers check `is_terminal` first |
| Mutation | Do not mutate the caller's `Game`. Probe on a **copy**, or use side-count helpers on `Position` |
| WASM / JS | None in this ticket |

`Game` / `Position` are `Copy` — copies are cheap for probes.

## RNG / determinism

- Small deterministic PRNG in core (e.g. XorShift64, matching the pattern already
  used in `moves` tests). No `rand` crate required for KET-15.
- Engines are constructed with an explicit `u64` seed.
- Same seed + same game path ⇒ same chosen move sequence.
- Ties (multiple equally good moves) are broken by uniform sample via that PRNG.

## Random policy

1. Collect `game.legal_moves()` (non-empty when not terminal).
2. Pick one uniformly at random via the engine RNG.
3. Return that `EdgeId`.

## Greedy policy

Exact priority — implement in this order:

1. **Take free boxes** — Among legal moves that complete ≥1 box, prefer those that
   maximize completed count (`2` over `1`). Break remaining ties with RNG.
2. **Safe moves** — Among legal moves that do **not** complete a box, keep only
   those that do **not** leave any adjacent box with **exactly 3 sides drawn**
   after the move (i.e. do not hand the opponent an immediate free box). If any
   such safe moves exist, pick one uniformly at random.
3. **Forced** — If every remaining move hands over a box, pick uniformly at random
   among **all** legal moves. No deeper sacrifice / all-but-two logic (that is KET-17).

### Probing helpers

`dab-core` has no built-in “sides drawn on box” API today. Implementation may:

- Count sides via `BoardGeom::box_edges` + `edge_is_drawn` after hypothetically
  drawing the candidate edge, and/or
- `play` / `undo` (or `apply_move` / `undo`) on a **copy** of `Game` / `Position`
  and inspect completed boxes or resulting side counts.

Either approach is fine if it matches the policy above and leaves the caller’s
state unchanged.

## Acceptance

### Unit (construct tiny positions)

- [ ] Greedy **always** chooses a completing move when at least one exists.
- [ ] When a completing move can claim **two** boxes and another claims one, greedy
      prefers the double claim (when both are legal).
- [ ] Greedy **never** chooses an unsafe move when at least one safe non-completing
      move exists.
- [ ] Random always returns a legal edge; same seed reproduces the same choice on
      the same position.

### Arena (native tests in `dab-core`)

| Parameter | Value |
|-----------|--------|
| Boards | At least **2×2** and **3×3** boxes |
| Games per board | **≥ 200** |
| Seeding | Fixed base seed; alternate who starts (or swap engine seats) |
| Matchup | Greedy vs Random |
| Pass bar | Greedy win rate **≥ 65%** of decisive games (ties excluded), on each listed board size |

- [ ] Arena test(s) pass under `cargo test -p dab-core`.
- [ ] No WASM / web changes required for Done.

## Handoff

| Next ticket | Uses this work for |
|-------------|-------------------|
| Thin KET-20 | [`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) — WASM `chooseMove` + HUD vs Random/Greedy |
| KET-17 | Stronger heuristic; may share `Engine` trait |
| Full KET-20 | Web Worker + full difficulty ladder (CGT/MCTS/Perfect) |
| CLI (optional) | `--p2 random\|greedy` auto-play in `dab-cli` |

## Files

| Path | Role |
|------|------|
| `docs/specs/phase2-random-greedy-engines.md` | This spec |
| `core/src/engine.rs` | Engines + RNG (implementation) |
| `core/src/lib.rs` | Re-export `engine` |
| `core/src/game.rs` / `moves.rs` / `board.rs` | Existing rules APIs (unchanged contract) |
