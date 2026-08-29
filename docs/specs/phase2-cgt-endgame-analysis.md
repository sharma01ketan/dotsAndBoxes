# Spec: Phase 2 CGT endgame analysis (KET-16)

Structural **strings-and-coins** analysis of a Dots and Boxes position:
chains, loops, long-chain count, and target parity for the player to move.

Linear: [KET-16](https://linear.app/sharma01ketan/issue/KET-16/dotsandboxes-ai-cgt-endgame-analysis-chains-loops-long-chain-parity).

Depends on: existing `dab-core` rules (`Position` / `Game`). No engine policy.

## Goals

- Build the strings-and-coins dual from packed board state (on demand).
- Detect **chains** (short = 1–2 boxes, long ≥ 3) and **loops**.
- Report **long-chain count** and **target parity** for the player to move.
- Expose `analyze_endgame()` for later engines (KET-17) and the theory overlay (KET-21).



## Non-goals

- Double-cross / all-but-two move selection → [KET-17](https://linear.app/sharma01ketan/issue/KET-17).
- Loony-value / control-value arithmetic beyond parity → KET-17 / [KET-18](https://linear.app/sharma01ketan/issue/KET-18).
- Exact solver → KET-18.
- WASM binding / Pixi overlay → [KET-21](https://linear.app/sharma01ketan/issue/KET-21).
- Changing Random/Greedy (KET-15).



## Dual (strings and coins)

Derived from `Position`; claimed-box ownership is irrelevant.


| Dual                 | Board                                                          |
| -------------------- | -------------------------------------------------------------- |
| Coin                 | Unclaimed box                                                  |
| String               | Undrawn side of an unclaimed box                               |
| String between coins | Undrawn edge shared by two unclaimed boxes                     |
| String to ground     | Undrawn boundary side (the other face is not an unclaimed box) |


A claimed neighbor already has all four sides drawn, so an undrawn shared edge never
touches a claimed box.

**Degree** of a coin = remaining sides = `4 - sides_drawn`.

## What counts as a chain or loop

Only coins with **degree 2** (exactly two remaining sides) form corridors.
Those two strings each go to another degree-2 coin or to ground.


| Component of degree-2 coins            | Kind        | Length            |
| -------------------------------------- | ----------- | ----------------- |
| Isolated coin (both strings to ground) | Short chain | 1                 |
| Path of 2 coins                        | Short chain | 2                 |
| Path of ≥ 3 coins                      | Long chain  | n                 |
| Cycle                                  | Loop        | n (≥ 4 on a grid) |


Endpoints of a path have one coin-neighbor (G-degree 1); interiors have two.
A loop is a component where every vertex has two coin-neighbors.

**Not** chains/loops:


| Remaining sides | Role                                             |
| --------------- | ------------------------------------------------ |
| 1               | Takeable (greedy capture); counted, not a region |
| 3 or 4          | Open / joint / midgame leftover                  |


`decomposed` is true iff no unclaimed box has 3 or 4 remaining sides
(only takeables + chains + loops). Empty and terminal boards are decomposed.

## Long-chain parity

Let `N` = total boxes on the board (`rows * cols`), `L` = number of long chains.

**P1 target parity** (classic long-chain rule):

- P1 wants `L` **odd** when `N` is even
- P1 wants `L` **even** when `N` is odd

Equivalent: `p1_target = (N + 1) % 2`.

**Player to move** wants that value if they are P1, the opposite if they are P2.


| Field               | Meaning                                     |
| ------------------- | ------------------------------------------- |
| `long_chain_parity` | `L % 2`                                     |
| `target_parity`     | What the player to move wants `L % 2` to be |
| `parity_ok`         | `long_chain_parity == target_parity`        |


Loops are detected and counted; they do **not** change this simple target-parity
formula (full loop/control values wait for KET-17).

Parity is a teaching / heuristic signal even when `decomposed` is false
(`L` is the count of long chains already formed).

## API

New module `[core/src/cgt.rs](../../core/src/cgt.rs)`, re-exported from
`[core/src/lib.rs](../../core/src/lib.rs)`:

```rust
pub fn analyze_endgame(game: &Game) -> EndgameAnalysis;

#[derive(Clone, Copy)]
pub struct EndgameAnalysis {
    pub long_chain_count: u8,
    pub short_chain_count: u8,
    pub loop_count: u8,
    pub takeable_count: u8,
    pub long_chain_parity: u8, // 0 or 1
    pub target_parity: u8,     // 0 or 1, player to move
    pub decomposed: bool,
    // regions() -> &[Region]
}

pub enum RegionKind { ShortChain, LongChain, Loop }

pub struct Region {
    pub kind: RegionKind,
    pub length: u8,
    pub boxes: BoxBits,
}
```




| Contract   | Detail                                                 |
| ---------- | ------------------------------------------------------ |
| Input      | `&Game` (position + player to move). Pure; no mutation |
| Terminal   | All counts 0, `decomposed == true`                     |
| Allocation | No heap; `Copy` result                                 |
| WASM / JS  | None in this ticket                                    |


`Game` / `Position` stay the rules source; this module only reads them.

## Worked examples (tests)

Construct by drawing every horizontal (or outer) edge listed; leave the corridor
verticals (or internals) undrawn.

1. **1×3 long chain** — all top and bottom horizontals drawn, all verticals open:
  one long chain of length 3, `L = 1`, `decomposed`, no loops.
2. **1×2 short chain** — same pattern: one short chain of length 2, `L = 0`.
3. **1×1 degree-2** — top and bottom drawn: short chain of length 1.
4. **2×2 loop** — all outer edges drawn, four internal edges open: one loop of
  length 4, `L = 0`.
5. **2×3 double corridor** — all three horizontal rows drawn, all verticals open:
  two long chains of length 3, `L = 2`.
6. **Empty 2×2** — no degree-2 corridors: `L = 0`, `decomposed == false`.
7. **Parity** — on (1) `N = 3` odd so P1 target is even (`0`); with `L = 1` and
  P1 to move, `parity_ok == false`. P2 to move wants odd → `parity_ok == true`.



## Acceptance

- [x] `analyze_endgame` matches the worked examples above (membership + counts).
- [x] Long-chain rule: P1 target is `(N + 1) % 2`; P2 wants the other bit.
- [x] Terminal position: empty analysis, `decomposed`.
- [x] Caller `Game` is unchanged (pure).
- [x] `cargo test -p dab-core` passes.
- [x] No web / WASM changes required for Done.



## Handoff


| Next   | Uses this for                                  |
| ------ | ---------------------------------------------- |
| KET-17 | Double-cross heuristic reads `EndgameAnalysis`; HUD **vs CGT** |
| KET-18 | Exact solver over the same dual                |
| KET-21 | Overlay paints `Region.boxes` + parity         |




## Files


| Path                                        | Role                               |
| ------------------------------------------- | ---------------------------------- |
| `docs/specs/phase2-cgt-endgame-analysis.md` | This spec                          |
| `core/src/cgt.rs`                           | Dual walk + `analyze_endgame`      |
| `core/src/lib.rs`                           | Re-export `cgt`                    |
| `core/src/board.rs` / `game.rs`             | Existing APIs (unchanged contract) |


