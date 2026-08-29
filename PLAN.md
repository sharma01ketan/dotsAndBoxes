# Dots and Boxes — Architecture & Build Plan

A realtime, multiplayer **Dots and Boxes** web application with a GPU-trained
AlphaZero-style AI and a combinatorial-game-theory (CGT) engine that plays the
endgame provably optimally. Every layer is chosen and tuned for performance, and
the whole thing deploys as a web app.

This document is the plan of record. It is meant to be reviewed and edited before
we write code. Nothing here is built yet.

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
- **Exact solver.** For small boards (3×3 boxes fully; 4×4 with pruning + symmetry
  + transposition tables) we compute game-theoretic values via alpha-beta over the
  strings-and-coins graph. This backs "Perfect" difficulty.

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
│  ├─ Neural inference (ONNX Runtime Web + WebGPU) ── hard AI      │
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
| Board rendering | **WebGL via PixiJS** | GPU-composited sprites/particles for satisfying chain-capture animations; scales to flashy effects. |
| Local AI (easy/medium) | **WASM MCTS (Rust core)** | Zero server round-trip, instant hints, works offline. |
| Neural inference (hard AI) | **ONNX Runtime Web + WebGPU** | Runs the trained net in-browser on the M1's GPU (via Metal) and on users' GPUs. "Serverless AI" — scales for free. Server-side `ort` fallback for ranked play integrity. |
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
  - `to_features()` — tensor encoding for the neural net
  - `analyze_endgame()` — chains, loops, long-chain parity, control
  - `solve_exact(depth/size limits)` — alpha-beta + transposition table + symmetry
- **Correctness:** property tests (random playouts never violate invariants),
  and cross-checks between the exact solver and known values for small boards.

---

## 6. AI system

### 6.1 Difficulty ladder
1. **Random** — legal random move (baseline).
2. **Greedy** — take free boxes; avoid giving away when possible.
3. **CGT heuristic (Medium)** — long-chain parity + double-cross; classic strong play.
4. **AlphaZero (Hard)** — CNN policy/value + MCTS.
5. **Perfect (small boards)** — exact solver over strings-and-coins.

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
- **Deployment:** best net exported to **ONNX**, shipped to the client for
  WebGPU inference; optional server-side `ort` inference for ranked integrity.
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

- **Stack:** React + TypeScript, Vite build, PixiJS (WebGL) board layer.
- **State:** lightweight store (Zustand) for game/UI; server as source of truth in
  multiplayer.
- **Rendering:** PixiJS scene for dots/edges/boxes; particle + tween animations
  for chain captures; `requestAnimationFrame`-driven, GPU-composited.
- **Perf practices:** WASM core for validation/hints, memoized components, sprite
  batching, avoid layout thrash, off-main-thread heavy compute (Web Worker for
  MCTS/inference).
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
├─ server/          # Rust: axum WebSocket backend (links core)
├─ web/             # React + TS + PixiJS frontend (loads core WASM)
├─ ai/              # Python: training, self-play, ONNX export
├─ gpu/             # Rust: wgpu/WGSL batched rollout + eval kernels
├─ proto/           # shared protocol definitions
├─ infra/           # Docker, CI, deploy configs, Grafana dashboards
└─ docs/            # architecture notes, CGT writeups
```

Tooling: Cargo workspace for Rust crates; pnpm for the web app;
`wasm-pack`/`wasm-bindgen` for the WASM build; `uv`/`poetry` for Python.

---

## 12. Phased roadmap

Each phase ends with something demoable.

### Phase 1 — Playable core (target of first milestone)
- Rust core: board model, bitboards, legal moves, box completion, scoring.
- WASM build of the core with a clean TS binding.
- React + PixiJS board with local **Opponent** play (two humans, one screen; id `hotseat`).
- Basic UI: new game, board size, turn indicator, score, win screen.
- Tests for the core; smoke build of WASM.
- **Demo:** a polished, animated, playable game in the browser.

### Phase 2 — AI opponents + CGT
- Random, greedy, CGT-heuristic engines in the Rust core.
  Spec for baseline engines: [`docs/specs/phase2-random-greedy-engines.md`](./docs/specs/phase2-random-greedy-engines.md) (KET-15).
- Browser vs Random/Greedy (thin KET-20): [`docs/specs/phase2-vs-ai-hotseat.md`](./docs/specs/phase2-vs-ai-hotseat.md).
- Mode label **Opponent** (was Hotseat): [`docs/specs/opponent-mode-copy.md`](./docs/specs/opponent-mode-copy.md).
- CGT endgame analysis (chains/loops/parity): [`docs/specs/phase2-cgt-endgame-analysis.md`](./docs/specs/phase2-cgt-endgame-analysis.md) (KET-16).
- CGT heuristic / double-cross: [`docs/specs/phase2-cgt-heuristic.md`](./docs/specs/phase2-cgt-heuristic.md) (KET-17).
- Exact solver (alpha-beta + TT + symmetry) for small boards → "Perfect" mode.
- Local WASM MCTS for Medium/Hard-lite.
- Theory overlay (chains/loops/parity) in the UI.
- **Demo:** single-player vs a genuinely strong, explainable AI. *(This is the
  polished playable-vs-AI milestone you asked to prioritize.)*

### Phase 3 — Realtime multiplayer
- Rust axum WS server linking the core; binary protocol.
- Rooms, matchmaking (Redis), spectating, reconnection, timers.
- Postgres persistence: users, games, ratings (Elo), replays.
- **Demo:** two browsers playing a live ranked match; a third spectating.

### Phase 4 — AlphaZero + WebGPU
- CNN policy/value net; self-play loop using the core.
- WebGPU/WGSL batched rollout + eval kernels via `wgpu`.
- PyTorch MPS training on the M1; arena gating; ONNX export.
- ONNX Runtime Web + WebGPU inference in-browser for "Hard" AI.
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
- **WASM/JS interop friction.** Keep the WASM API small and data-oriented
  (indices and typed arrays, not rich objects).
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
1. Default board size for v1 (3×3 vs 5×5 boxes)? Affects solver + net scope.
2. Auth: anonymous handles first, add real auth later? Or require login for ranked?
3. Rating system: Elo (simple) vs Glicko-2 (better, more work)?
4. Server-side vs client-only neural inference for ranked integrity?
5. Hosting targets (Fly.io vs Railway vs VPS) — pick once we deploy.
```
