# Dots and Boxes

Realtime multiplayer **Dots and Boxes** with a CGT endgame solver and an
AlphaZero-style AI. See [PLAN.md](./PLAN.md) for the full architecture.

## Status

Scaffolding the monorepo ([KET-5](https://linear.app/sharma01ketan/issue/KET-5)).

| Area | Path | Status |
|------|------|--------|
| Shared game core | `core/` | Stub (Rust) |
| Realtime server | `server/` | Stub (Rust) |
| GPU kernels | `gpu/` | Stub (Rust) |
| Web UI | `web/` | Stub (Vite + React + TS) |
| AI training | `ai/` | Coming in KET-5.3 |
| Protocol | `proto/` | Placeholder |
| Infra | `infra/` | Placeholder |
| Docs | `docs/` | Placeholder |

## Layout

```
dotsAndBoxes/
├─ PLAN.md
├─ core/            # Rust: shared game engine + solver (native + wasm)
├─ server/          # Rust: axum WebSocket backend
├─ gpu/             # Rust: wgpu/WGSL compute kernels
├─ web/             # React + TypeScript + PixiJS
├─ ai/              # Python training (next)
├─ proto/           # shared protocol definitions
├─ infra/           # Docker, CI, deploy, Grafana
└─ docs/            # architecture notes, CGT writeups
```

## Prerequisites

- Rust stable (edition 2021; `wasm32-unknown-unknown` target for later WASM work)
- Node.js 20+ and pnpm 9+
- Python 3.11+ (ai — KET-5.3)

## Build (Rust)

```bash
cargo build
cargo test
cargo run -p dab-server
```

## Build (Web)

```bash
pnpm install
pnpm --filter @dab/web build
pnpm dev
```

## License

MIT
