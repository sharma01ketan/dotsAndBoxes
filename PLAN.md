# Dots and Boxes — Architecture & Build Plan

A realtime, multiplayer **Dots and Boxes** web application with a GPU-trained
AlphaZero-style AI and a combinatorial-game-theory (CGT) engine that plays the
endgame provably optimally. Every layer is chosen and tuned for performance, and
the whole thing deploys as a web app.

This document is the plan of record for the **full** product (local vs-AI,
then multiplayer, then AlphaZero). Phases are independently demoable.

**Shipped (2026-08).** Phase 1 and Phase 2 are in git: WASM rules, Pixi
Opponent / vs-AI board (Random, Greedy, Medium MCTS, Hard CGT, Perfect on
2×2 and 3×3), Worker search (KET-20), theory overlay (KET-21), hints (KET-22).
Phase 3 started: `server/` health + JSON WebSocket sessions (KET-23, KET-24).
What is *not* shipped: binary protocol, rooms, AlphaZero, GPU kernels.

**Next.** [KET-25](https://linear.app/sharma01ketan/issue/KET-25) (binary
protocol). Parallel: in-WASM AlphaZero slice A
([`docs/specs/phase4-in-wasm-az.md`](docs/specs/phase4-in-wasm-az.md)).
Do not recreate `gpu/` / `proto/` / `infra/`. Recreate `ai/` for Phase 4 training
(slice D).

**Parallel (does not block KET-25).** JS-shell diet so the WASM core is not
hidden behind a 500 kB Pixi+React parse:
[`docs/specs/wasm-first-client.md`](docs/specs/wasm-first-client.md).

---

## 1. Goals & non-goals

### Goals
- **Deployable web app.** Frontend is a static SPA on a CDN; backend is a
  containerized Rust service. Everything runs in a browser, no install.
- **Realtime multiplayer.** Authoritative server, matchmaking, spectating,
  reconnection, ELO ratings, replays.
- **Strong AI.** An AlphaZero-style neural engine, plus a CGT engine that is
  *provably optimal* in the endgame and on small boards. A difficulty ladder from
  "random" to "perfect".
- **Performance engineering, visible.** Bitboard core, WASM on the client,
  WebGPU compute for batched search, and a metrics dashboard (p50/p99 latency,
  inference throughput, concurrent games).
- **CGT as a feature, not just an implementation detail.** A "theory overlay"
  that visualizes chains, loops, long-chain parity, and control — turning the
  math from *Lessons in Play* into a teaching tool.

### Non-goals (for v1)
- Native mobile apps (the web app will be responsive/touch-friendly instead).
- Boards larger than what the exact solver can handle (we cap "perfect" mode).
- Account federation / OAuth zoo — start with a single lightweight auth method.

---

## 2. Combinatorial game theory background (why this project is special)

Dots and Boxes is deceptively deep. The theory we lean on:

- **Strings-and-coins dual.** Every Dots and Boxes position maps to a
  strings-and-coins graph. This dual is the natural representation for the solver.
- **Chains and loops.** The endgame decomposes into long chains (length ≥ 3) and
  loops. Who is forced to "open" the first long chain usually loses.
- **Long-chain rule / parity.** With `n` initial dots, the first player wants the
  number of long chains to have a specific parity. This is a concrete, teachable
  heuristic that the Medium AI will follow.
- **Double-cross (all-but-two) strategy.** The core sacrifice technique: decline
  the last two boxes of a chain to keep control. The engine implements this.
- **Loony endgame theory (Berlekamp).** Exact evaluation of the "all long chains
  and loops" endgame via controlled-value / loony-move analysis.
- **Exact solver.** For 2×2 and 3×3 boxes we search the real `Game` graph
  (negamax + TT + D4/D2 symmetry). Grid D&B is not Demaine–Diomidov
  strings-and-coins; the dual is for CGT *analysis*, not for Perfect values.
  4×4+ stays off Perfect (PSPACE-complete). Empty 3×3 is a second-player win
  by three (Barker & Korf).

**Resume angle:** most game-AI projects are "MCTS on a board". This one combines a
learned AlphaZero policy *and* a classical, provably-correct CGT endgame solver,
and shows the theory to the user. That contrast is the story.

---

## 3. High-level architecture

```
┌────────────────────────────────────────────────────────────────┐
│                          Browser (SPA)                          │
│                                                                  │
│  React + TypeScript UI                                           │
│  ├─ WebGL board renderer (PixiJS)  ── 60fps chain animations     │
│  ├─ Game core (Rust → WASM)        ── rules, bitboards, hints    │
│  ├─ Local AI (WASM MCTS)           ── easy/medium, zero-latency  │
│  ├─ Neural inference (tract CPU, lazy wasm-az) ── Hard (AZ) v1     │
│  └─ WebSocket client               ── realtime multiplayer       │
└───────────────┬──────────────────────────────────────────────┘
                │ WSS (binary protocol)
┌───────────────▼──────────────────────────────────────────────┐
│                    Backend (Rust: axum + tokio)                │
│  ├─ WebSocket gateway + session mgmt                           │
│  ├─ Authoritative game engine (shared Rust core, native)      │
│  ├─ Matchmaking + rooms                                        │
│  ├─ ELO/rating service                                         │
│  ├─ CGT solver service (perfect play, small boards)           │
│  └─ Optional server-side neural inference (ort/ONNX Runtime)   │
└───────┬───────────────────────┬───────────────────────────────┘
        │                       │
   ┌────▼────┐            ┌─────▼─────┐
   │ Postgres│            │   Redis   │
   │ users,  │            │ matchmake,│
   │ ratings,│            │ pub/sub,  │
   │ replays │            │ presence  │
   └─────────┘            └───────────┘

        (offline, not in request path)
┌────────────────────────────────────────────────────────────────┐
│              Training pipeline (Python + PyTorch/MPS)           │
│  ├─ Self-play (uses shared Rust core via bindings)             │
│  ├─ WebGPU/wgpu batched rollouts + board eval (custom kernels) │
│  ├─ CNN policy/value net, trained on Apple Silicon (MPS)       │
│  └─ Export → ONNX → shipped to client + optional server        │
└────────────────────────────────────────────────────────────────┘
```

---

## 4. Tool-by-tool decisions (the "what goes where" rationale)

| Concern | Choice | Why |
|---|---|---|
| Shared game core (rules, bitboards, solver) | **Rust → native + WASM** | One source of truth. Server links natively; browser loads WASM. No client/server rules desync. Fast bitboard ops. |
| Frontend UI | **React + TypeScript** | Productive, huge ecosystem, easy state management, familiar to you. |
| Board rendering | **WebGL via PixiJS** (under review) | GPU-composited sprites/particles for later chain-capture spectacle. Today the board is lines/dots/fills. Lazy-load or replace: [`wasm-first-client.md`](docs/specs/wasm-first-client.md). |
| Local AI (easy/medium) | **WASM MCTS (Rust core)** | Zero server round-trip, instant hints, works offline. |
| Neural inference (hard AI) | **tract (CPU, `simd128`) in lazily-loaded `@dab/dab-wasm-az`** | v1. Spec: [`docs/specs/phase4-in-wasm-az.md`](docs/specs/phase4-in-wasm-az.md). ONNX Runtime Web + WebGPU is the later fast path, not this slice. Server-side `ort` still optional for ranked integrity. |
| Backend / realtime | **Rust: axum + tokio + tungstenite** | Low-latency authoritative server, safe concurrency, links the shared core natively. |
| Persistence | **Postgres** | Users, ratings, replays; relational + JSONB for move logs. |
| Ephemeral state | **Redis** | Matchmaking queue, pub/sub across server instances, presence. |
| GPU compute / custom kernels | **WebGPU (WGSL) via `wgpu`** | Batched MCTS rollouts + vectorized board eval. Runs on M1 (Metal), Linux (Vulkan), and in-browser — portable and deployable, unlike CUDA. |
| Model training | **Python + PyTorch (MPS)** | Small board + small CNN ⇒ feasible on M1. Standard, hireable ML stack. |

Rust is used exactly where it wins (shared core, backend, GPU-compute engine),
TypeScript where iteration speed and UI ecosystem win, Python where the ML
tooling lives.

---

## 5. Game core design (Rust)

- **Board model.** Configurable `rows × cols` of boxes. Edges indexed
  canonically; a move toggles one edge. Completing a box grants another turn.
- **Bitboard representation.** Horizontal edges, vertical edges, and captured
  boxes each packed into fixed-width integer arrays (`u64` words) for
  cache-friendly, branch-light move generation and box-completion checks.
- **Strings-and-coins dual.** Derived on demand for the solver: coins = boxes
  (+ ground), strings = edges.
- **APIs (shared by server, client-WASM, and training bindings):**
  - `legal_moves()`, `apply_move()`, `undo()`, `is_terminal()`, `score()`
  - `to_features()` — tensor encoding for the neural net (frozen in
    [`docs/specs/phase4-in-wasm-az.md`](docs/specs/phase4-in-wasm-az.md))
  - `analyze_endgame()` — chains, loops, long-chain parity, control
  - `solve_exact(depth/size limits)` — alpha-beta + transposition table + symmetry
- **Correctness:** property tests (random playouts never violate invariants),
  and cross-checks between the exact solver and known values for small boards.

---

## 6. AI system

### 6.1 Difficulty ladder
1. **Random** — legal random move (baseline). **Shipped.**
2. **Greedy** — take free boxes; avoid giving away when possible. **Shipped.** Default HUD mode.
3. **CGT heuristic** — double-cross / all-but-four. PLAN originally called this
   Medium. The HUD label is **Hard (CGT)** because AlphaZero is unbuilt. **Shipped.**
   Parity is analyzed for the overlay (KET-21), not used to steer `CgtEngine`.
4. **AlphaZero (Hard)** — small ResNet policy/value + PUCT in WASM. **Phase 4**
   ([`docs/specs/phase4-in-wasm-az.md`](docs/specs/phase4-in-wasm-az.md)). HUD
   **Hard (AZ)** beside **Hard (CGT)**; MCTS stays **Medium (MCTS)**. Slice A
   is plumbing only (no HUD until a trained net). Local WASM MCTS (KET-19) remains
   the Medium rung: `policy = 4`.
5. **Perfect (2×2 / 3×3)** — exact `Game` search. **Shipped.** HUD search
   runs in a Web Worker (KET-20).

### 6.2 AlphaZero pipeline
- **Network:** small residual CNN over the edge/box grid → (policy over edges,
  value ∈ [-1, 1]). Board is small, so the net is small and fast.
- **Self-play:** MCTS + net generates games; states use the shared Rust core.
- **GPU acceleration (the showcase):** WebGPU/WGSL compute kernels via `wgpu`
  run **batched rollouts and vectorized board evaluation** — thousands of
  parallel games on the GPU. Same kernels run on the M1 for training and can run
  in-browser for strong local play.
- **Training:** PyTorch on MPS (Apple Silicon). Loss = policy cross-entropy +
  value MSE. Checkpoints gated by win-rate vs. previous best (Elo-style arena).
- **Deployment:** best net exported to **ONNX**, fetched as a static
  `/models/*.onnx` into `@dab/dab-wasm-az` (tract CPU, `simd128`). WebGPU /
  ONNX Runtime Web is a later fast path behind the same `Evaluate` trait.
  Optional server-side `ort` for ranked integrity.
- **Evaluation:** arena vs. the CGT engine and the exact solver on small boards —
  gives concrete, quotable strength numbers.

---

## 7. Realtime multiplayer

- **Transport:** WebSocket (WSS), **binary protocol** (compact move/state
  frames). Consider MessagePack or a hand-rolled format.
- **Authority:** server owns game state; clients send intents, server validates
  with the shared core and broadcasts authoritative updates. Client does
  optimistic local apply + reconciliation for snappy UX.
- **Rooms & matchmaking:** Redis-backed queue with rating-based pairing; private
  rooms via share code; spectators subscribe read-only.
- **Resilience:** reconnection with state resync; per-move timers; graceful
  handling of disconnects (pause/forfeit rules).
- **Ratings:** Elo (or Glicko-2) updated on game end, persisted in Postgres.
- **Replays:** full move log stored; deterministic re-simulation via the core.

---

## 8. Frontend

- **Stack:** React + TypeScript HUD, Vite build, PixiJS board (not in the
  entry chunk — [`wasm-first-client.md`](docs/specs/wasm-first-client.md)).
- **State:** lightweight store (Zustand) for game/UI; server as source of truth in
  multiplayer.
- **Rendering:** PixiJS scene for dots/edges/boxes; custom rAF tweens today;
  particles only if Phase 5 actually ships them. Alternative: Canvas 2D board
  (Track B2 in the spec) if that spectacle is deferred.
- **Perf practices:** WASM core for rules/search, Worker for Perfect /
  `chooseMove` / Hint, wasm-opt on the deploy artifact, no extra search on
  the UI thread. Do not raise Vite’s chunk warning to hide Pixi.
- **Theory overlay:** toggle to highlight chains/loops, show parity and predicted
  controller — the CGT teaching feature.
- **Modes:** local hotseat, vs AI (pick difficulty), online ranked/casual,
  spectate, replay viewer.

---

## 9. Data model (Postgres, first cut)

- `users(id, handle, created_at, rating, games_played, ...)`
- `games(id, board_rows, board_cols, player1, player2, winner, started_at, ended_at, mode)`
- `moves(game_id, ply, edge_index, player, ts)` (or JSONB move log on `games`)
- `ratings_history(user_id, game_id, rating_before, rating_after, ts)`

Redis: `matchmaking:queue`, `presence:*`, `room:*` pub/sub channels.

---

## 10. Deployment & infra

- **Frontend:** static build → CDN (Cloudflare Pages / Vercel / Netlify).
- **Backend:** Docker container on a host that supports long-lived WebSockets
  (Fly.io / Railway / Render / a small VPS). Horizontal scale via Redis pub/sub.
- **Postgres:** managed (Neon / Supabase / Fly Postgres).
- **Redis:** managed (Upstash / Fly Redis).
- **AI:** trained offline on the M1; ONNX artifact shipped with the frontend
  (and optionally loaded server-side). No GPU needed in production request path.
- **CI/CD:** GitHub Actions — build/test Rust + TS, build WASM, build/push image,
  deploy.
- **Observability:** Prometheus metrics from the Rust server + Grafana dashboard
  (move latency p50/p99, active games, WS connections, inference throughput).

---

## 11. Repo structure (proposed monorepo)

```
dotsAndBoxes/
├─ PLAN.md
├─ core/            # Rust: shared game engine + solver (native + wasm targets)
├─ wasm/            # @dab/dab-wasm (rules, engines)
├─ wasm-az/         # @dab/dab-wasm-az (tract AZ; lazy)
├─ server/          # Rust: axum health + WS sessions (links core)
├─ web/             # React + TS + PixiJS frontend (loads core WASM)
├─ ai/              # Python: training, self-play, ONNX export — Phase 4
├─ gpu/             # Rust: wgpu/WGSL batched rollout + eval kernels — Phase 4
├─ proto/           # shared protocol definitions — Phase 3
├─ infra/           # Docker, CI, deploy configs, Grafana dashboards — Phase 5
└─ docs/            # architecture notes, CGT writeups
```

`gpu/`, `proto/`, and `infra/` stay out of git until those phases
(KET-61). `ai/` is recreated for Phase 4 training (slice D). Cargo workspace is
`core`, `cli`, `wasm`, `wasm-az`, `server`. Deploy config
is `vercel.json` at repo root (frontend only).

Tooling: Cargo workspace for Rust crates; pnpm for the web app;
`wasm-pack`/`wasm-bindgen` for the WASM build; `uv`/`poetry` for Python
when Phase 4 starts.

---

## 12. Phased roadmap

Each phase ends with something demoable.

### Phase 1 — Playable core — **done**
- Rust core: board model, bitboards, legal moves, box completion, scoring.
- WASM build of the core with a clean TS binding.
- React + PixiJS board with local **Opponent** play (two humans, one screen; id `hotseat`).
- Basic UI: new game, board size, turn indicator, score, win screen.
- Tests for the core; smoke build of WASM.
- **Demo:** a polished, animated, playable game in the browser.

### Phase 2 — AI opponents + CGT
**Shipped in core + HUD (KET-15–18 Done).**
- Random, greedy, CGT-heuristic engines in the Rust core.
  Spec: [`docs/specs/phase2-random-greedy-engines.md`](./docs/specs/phase2-random-greedy-engines.md) (KET-15).
- Browser vs Random/Greedy/Hard/Perfect (thin HUD): [`docs/specs/phase2-vs-ai-hotseat.md`](./docs/specs/phase2-vs-ai-hotseat.md).
- Mode label **Opponent** (was Hotseat): [`docs/specs/opponent-mode-copy.md`](./docs/specs/opponent-mode-copy.md).
- CGT endgame analysis (chains/loops/parity): [`docs/specs/phase2-cgt-endgame-analysis.md`](./docs/specs/phase2-cgt-endgame-analysis.md) (KET-16).
- CGT heuristic / double-cross: [`docs/specs/phase2-cgt-heuristic.md`](./docs/specs/phase2-cgt-heuristic.md) (KET-17). Refuse/skip remnant (KET-58). **Done.**
- Exact solver for 2×2 / 3×3 → **vs Perfect**: [`docs/specs/phase2-exact-solver.md`](./docs/specs/phase2-exact-solver.md) (KET-18).
- Theory overlay: [`docs/specs/phase2-theory-overlay.md`](./docs/specs/phase2-theory-overlay.md) (KET-21). **Done.**
- Hints: [`docs/specs/phase2-hints.md`](./docs/specs/phase2-hints.md) (KET-22). **Done.**

**Still this phase (do these before Phase 3).**
- Cancel vs-AI loop on New game; unstick `chooseMove` errors (KET-57). **Done.**
- Web Worker so Perfect / CPU search never freeze the tab (KET-20). **Done.**
- WASM boundary: `boxOwner` range check, panic hook, retry init (KET-59). **Done.**
- Pixi incremental claimed boxes + hover rewire (KET-60). **Done.**
  Spec: [`docs/specs/phase2-pixi-incremental.md`](./docs/specs/phase2-pixi-incremental.md).
- CI: debug + **release** `cargo test`, wasm-pkg glue + source stamp (KET-47).
- Delete unused Phase 3/4 stub crates until those phases start (KET-61). **Done.**
- Local WASM MCTS for Medium/Hard-lite (KET-19). **Done.** Spec:
  [`docs/specs/phase2-mcts.md`](docs/specs/phase2-mcts.md).
- **Demo:** single-player vs a genuinely strong, explainable AI, **without jank**.

**Runtime diet (parallel, any time before Phase 5).** Spec:
[`docs/specs/wasm-first-client.md`](docs/specs/wasm-first-client.md).
Lazy-load the board, optimize production WASM, then keep Pixi **or** drop it
for Canvas 2D. Not a Makepad/egui/Leptos rewrite. Not CRDTs for `play()`.

### Phase 3 — Realtime multiplayer
- axum health + JSON WS sessions: [`docs/specs/phase3-server-ws.md`](./docs/specs/phase3-server-ws.md) (KET-23, KET-24). **Done.**
- Binary protocol (KET-25). **Next.**
- Rooms, matchmaking (Redis), spectating, reconnection, timers.
- Postgres persistence: users, games, ratings (Elo), replays.
- **Demo:** two browsers playing a live ranked match; a third spectating.

### Phase 4 — AlphaZero (in-WASM tract) + later WebGPU
- CNN/ResNet policy/value net; self-play via `dab-core`; spec:
  [`docs/specs/phase4-in-wasm-az.md`](docs/specs/phase4-in-wasm-az.md).
- **v1 inference:** tract CPU (`simd128`) in lazily-loaded `wasm-az/`.
- WebGPU/WGSL batched rollout + eval kernels via `wgpu` — later; do not recreate
  `gpu/` until that slice.
- PyTorch MPS training on the M1; arena gating; ONNX export + sidecar stamp.
- **Demo:** the learned net beating the heuristic engine, running in the browser.

### Phase 5 — Polish, observability, deploy
- Prometheus + Grafana dashboards; load test concurrent games.
- CI/CD; deploy frontend to CDN + backend container.
- Docs, replay sharing, final UX polish.
- **Demo:** the live public URL + a metrics dashboard.

---

## 13. Risks & mitigations
- **Scope creep.** Phases are independently demoable; we can stop after any phase
  and still have a strong project.
- **Off-thread search.** Perfect / `chooseMove` run in a Worker (KET-20). Keep
  `runAiTurn` bound to `gameGeneration` (KET-57). Do not add more search on
  the UI thread.
- **WASM/JS interop friction.** Keep the WASM API small and data-oriented
  (indices and typed arrays, not rich objects). Range-check every index at the
  binding (`boxOwner` returns −1 when out of range).
- **AlphaZero training cost on M1.** Board and net are small; start with the
  smallest board that is still interesting (e.g., 3×3 boxes) and scale up.
- **WebGPU browser support.** Provide a WASM-CPU fallback for the AI so the app
  works everywhere; WebGPU is the fast path.
- **Realtime edge cases.** Server is authoritative; deterministic core enables
  exact resync and replay.

---

## 14. Resume talking points (what this demonstrates)
- Cross-compiled Rust core (native + WASM) as a single source of truth.
- Custom GPU compute kernels (WebGPU/WGSL) for batched search.
- AlphaZero self-play RL trained on Apple Silicon, served in-browser via WebGPU.
- Provably-optimal endgame play from combinatorial game theory.
- Authoritative low-latency realtime netcode with reconciliation and replays.
- Production concerns: CI/CD, containerization, metrics/observability, load.

---

## 15. Open questions (to resolve before/along the way)
1. Default board size for v1 — **settled: 3×3 boxes** (`DEFAULT_BOARD`). Perfect
   HUD is 2×2/3×3 only. 5×5 is playable vs Greedy/Hard, not vs Perfect.
2. Auth: anonymous handles first, add real auth later? Or require login for ranked?
3. Rating system: Elo (simple) vs Glicko-2 (better, more work)?
4. Server-side vs client-only neural inference for ranked integrity?
5. Hosting targets (Fly.io vs Railway vs VPS) — pick once we deploy.
6. Board renderer after the shell diet: **keep Pixi** (B1, if Phase 5
   particles are real) vs **Canvas 2D** (B2, if they are not). Spec:
   [`docs/specs/wasm-first-client.md`](docs/specs/wasm-first-client.md).
```
