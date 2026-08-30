# Dots and Boxes

Local **Opponent** and vs-AI Dots and Boxes in the browser. Rules live in Rust
(`dab-core`) and run in WASM. The board is PixiJS. Full roadmap (multiplayer,
AlphaZero, GPU) is in [PLAN.md](./PLAN.md). That document is the vision for
later phases, not a description of what `main` ships today.

## Layout

```
dotsAndBoxes/
├─ PLAN.md
├─ core/            # Rust: rules, engines, exact solver
├─ wasm/            # Rust → WASM (@dab/dab-wasm)
├─ cli/             # Optional ASCII playground
├─ server/          # Stub — Phase 3
├─ gpu/             # Stub — Phase 4
├─ web/             # React + TypeScript + PixiJS
├─ ai/              # Stub — Phase 4
├─ proto/           # Empty — Phase 3
├─ infra/           # Empty; deploy is root vercel.json
└─ docs/specs/      # Phase 1–2 specs
```

| Area | Path | Status |
|------|------|--------|
| Shared game core | `core/` | Rules, Random/Greedy/CGT/Perfect |
| WASM bindings | `wasm/` | `@dab/dab-wasm` |
| Terminal playground | `cli/` | Optional REPL (`cargo test -p dab-core` is the real check) |
| Realtime server | `server/` | Stub (Phase 3) |
| GPU kernels | `gpu/` | Stub (Phase 4) |
| Web UI | `web/` | Opponent / vs-AI Pixi board |
| AI training | `ai/` | Stub (Phase 4) |
| Protocol | `proto/` | Empty |
| Infra | `infra/` | Empty; Vercel at repo root |
| Docs | `docs/specs/` | Phase 1–2 specs |

## Prerequisites

- **Rust** stable + `wasm32-unknown-unknown` + [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
- **Node.js** 20+ and **pnpm** 9+
- **Python** 3.11+ only if you touch the Phase 4 stub

## Quick start

```bash
pnpm install
pnpm dev
```

Open http://localhost:5173 — Pixi board, scores, Opponent / vs-AI modes.

Rebuild WASM after `core/` or `wasm/` changes:

```bash
pnpm build:wasm
```

Core tests (debug is fast; release includes empty 3×3 Perfect):

```bash
cargo test --workspace
cargo test -p dab-core --release
```

Optional ASCII playground:

```bash
cargo run -p dab-cli -- --rows 3 --cols 3
```

See [cli/README.md](./cli/README.md) and [wasm/README.md](./wasm/README.md).

## Next

See PLAN.md §12 Phase 2 remaining, or Linear: [KET-57](https://linear.app/sharma01ketan/issue/KET-57) (AI loop), [KET-20](https://linear.app/sharma01ketan/issue/KET-20) (Worker), [KET-47](https://linear.app/sharma01ketan/issue/KET-47) (CI).

## Tooling

- Rust: `rustfmt.toml`, Cargo workspace (`core`, `cli`, `wasm`, `server`, `gpu`)
- JS/TS: pnpm workspace (`web`, `wasm/pkg`), Prettier, ESLint
- Python: `ai/pyproject.toml` (scaffold only)

## License

MIT
