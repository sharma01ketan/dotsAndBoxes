# Dots and Boxes

Realtime multiplayer **Dots and Boxes** with a CGT endgame solver and an
AlphaZero-style AI. Full design: [PLAN.md](./PLAN.md).

## Layout

```
dotsAndBoxes/
├─ PLAN.md
├─ core/            # Rust: shared game engine + solver
├─ wasm/            # Rust → WASM bindings (package @dab/dab-wasm)
├─ cli/             # Rust: terminal hotseat playground
├─ server/          # Rust: axum WebSocket backend
├─ gpu/             # Rust: wgpu/WGSL compute kernels
├─ web/             # React + TypeScript + PixiJS frontend
├─ ai/              # Python: training, self-play, ONNX export
├─ proto/           # shared protocol definitions
├─ infra/           # Docker, CI, deploy, Grafana
└─ docs/            # architecture notes, CGT writeups
```

| Area | Path | Status |
|------|------|--------|
| Shared game core | `core/` | Rules + bitboards (playable via CLI) |
| WASM bindings | `wasm/` | `@dab/dab-wasm` for the browser |
| Terminal playground | `cli/` | Hotseat REPL to validate the core |
| Realtime server | `server/` | Stub (Rust) |
| GPU kernels | `gpu/` | Stub (Rust) |
| Web UI | `web/` | Hotseat PixiJS board + WASM |
| AI training | `ai/` | Stub (Python) |
| Protocol | `proto/` | Placeholder |
| Infra | `infra/` | Placeholder |
| Docs | `docs/` | Placeholder |

## Prerequisites

- **Rust** stable + `wasm32-unknown-unknown` + [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
- **Node.js** 20+ and **pnpm** 9+
- **Python** 3.11+ (optional [uv](https://github.com/astral-sh/uv))

## Quick start

```bash
# Rust workspace
cargo build
cargo test
cargo run -p dab-server

# Terminal hotseat playground (validates dab-core)
cargo run -p dab-cli
cargo run -p dab-cli -- --rows 3 --cols 3

# WASM + web (browser can call the core)
pnpm build:wasm
pnpm install
pnpm --filter @dab/web build
pnpm dev
```

Open http://localhost:5173 — you should see scores, legal move buttons, and plays updating via WASM.

See [cli/README.md](./cli/README.md) and [wasm/README.md](./wasm/README.md).

## Tooling

- Rust: `rustfmt.toml`, Cargo workspace (`core`, `cli`, `wasm`, `server`, `gpu`)
- JS/TS: pnpm workspace (`web`, `wasm/pkg`), Prettier, ESLint
- Python: `ai/pyproject.toml` (hatchling, uv-compatible)

## License

MIT
