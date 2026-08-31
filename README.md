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
├─ web/             # React + TypeScript + PixiJS
├─ server/          # Rust: axum health + WS sessions
├─ docs/specs/      # Phase 1–3 specs
└─ vercel.json      # Deploy config (not infra/)
```

| Area | Path | Status |
|------|------|--------|
| Shared game core | `core/` | Rules, Random/Greedy/CGT/Perfect |
| WASM bindings | `wasm/` | `@dab/dab-wasm` |
| Terminal playground | `cli/` | Optional REPL (`cargo test -p dab-core` is the real check) |
| Web UI | `web/` | Opponent / vs-AI Pixi board |
| Server | `server/` | Health + JSON WebSocket sessions |
| Docs | `docs/specs/` | Phase 1–3 specs |

`gpu/`, `ai/`, `proto/`, and `infra/` are not in the tree until Phase 4–5.

## Prerequisites

- **Rust** stable + `wasm32-unknown-unknown` + [`wasm-pack`](https://rustwasm.github.io/wasm-pack/)
- **Node.js** 22+ and **pnpm** 9+

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

## CI

Push and pull requests run `.github/workflows/ci.yml`: rustfmt, clippy (`-D warnings`),
`cargo test --workspace`, `cargo test -p dab-core --release`, web lint / typecheck /
tests, and a wasm-pack rebuild that must match committed JS glue plus
`wasm/pkg/SOURCE_STAMP` (the `.wasm` blob is host-dependent; the stamp is a
hash of `core/` + `wasm/` Rust sources so a forgotten rebuild still fails).

## Next

See PLAN.md §12, or Linear: [KET-25](https://linear.app/sharma01ketan/issue/KET-25) (binary protocol).

## Tooling

- Rust: `rustfmt.toml`, Cargo workspace (`core`, `cli`, `wasm`, `server`)
- JS/TS: pnpm workspace (`web`, `wasm/pkg`), Prettier, ESLint (`no-console`)
- CI: `.github/workflows/ci.yml`
- Deploy: `vercel.json` at repo root

## License

MIT
