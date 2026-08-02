# Dots and Boxes

Realtime multiplayer **Dots and Boxes** with a CGT endgame solver and an
AlphaZero-style AI. Full design: [PLAN.md](./PLAN.md).

## Layout

```
dotsAndBoxes/
├─ PLAN.md
├─ core/            # Rust: shared game engine + solver (native + wasm)
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
| Shared game core | `core/` | Stub (Rust) |
| Realtime server | `server/` | Stub (Rust) |
| GPU kernels | `gpu/` | Stub (Rust) |
| Web UI | `web/` | Stub (Vite + React + TS) |
| AI training | `ai/` | Stub (Python) |
| Protocol | `proto/` | Placeholder |
| Infra | `infra/` | Placeholder |
| Docs | `docs/` | Placeholder |

## Prerequisites

- **Rust** stable (edition 2021); add `wasm32-unknown-unknown` when WASM lands
- **Node.js** 20+ and **pnpm** 9+
- **Python** 3.11+ (optional [uv](https://github.com/astral-sh/uv))

## Quick start

```bash
# Rust workspace
cargo build
cargo test
cargo run -p dab-server

# Web app
pnpm install
pnpm --filter @dab/web build
pnpm dev

# AI package (scaffold)
cd ai && python3 -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]"
dab-ai
pytest
```

## Tooling

- Rust: `rustfmt.toml`, Cargo workspace (`core`, `server`, `gpu`)
- JS/TS: pnpm workspace, Prettier (root), ESLint (`web/`)
- Python: `ai/pyproject.toml` (hatchling, uv-compatible)

## License

MIT
