# Spec: WASM-first client runtime

Push the “stupid fast, zero-latency” ethos **through the layers we already
chose**, not by importing someone else’s stack. The 1Password / Figma / Rerun
lesson for this repo is: **Rust owns truth and search; the JS shell stays
thin.** We already did the hard half. The remaining waste is the shell.

Does not block [KET-25](https://linear.app/sharma01ketan/issue/KET-25)
(binary protocol). Do not recreate `gpu/` / `ai/` / `proto/` for this.

Depends on: [`PLAN.md`](../../PLAN.md) §§4, 8, 12;
[`phase2-wasm-boundary.md`](./phase2-wasm-boundary.md) (KET-59);
[`phase2-vs-ai-hotseat.md`](./phase2-vs-ai-hotseat.md) (KET-20 Worker);
chunk audit (Vite entry ~504 kB minified, ~56% Pixi + deps, ~38% React DOM).

## Why

The browser tab already runs a Rust engine (`dab-core` → `@dab/dab-wasm`), a
second `WasmGame` in a Worker for `chooseMove` / Perfect / Hint, and a
presentational Pixi board. That **is** the 1Password pattern: one core,
compiled to WASM, no rules in TypeScript.

What it is **not**: a 504 kB JS parse before “Loading WASM…” can even fetch
the 123 kB `.wasm`. Vite warns because `App` statically imports `PixiBoard`,
which statically imports the `pixi.js` barrel. React DOM is the other ~38%.
The board on screen is still lines, dots, fills, and `1`/`2` labels, plus a
custom rAF tween in `motion.ts`. PLAN.md bought Pixi for later particles;
phase 1 polish deferred those particles.

`wasm-pack` is invoked with `--no-opt`. Release WASM is therefore unoptimized
binary, not “Rust is slow.”

## Mapping: the list → this repo

Adopt the *lesson*. Do not adopt the *product*.

| Source | Lesson | Here |
|--------|--------|------|
| [1Password](https://blog.1password.com/1password-rust-webassembly/) | One Rust engine for crypto/sync/rules; JS is a shell | **Already shipped.** Grow the core (KET-25 uses the same `dab-core` on the server). Do not add a second rules implementation in TS. |
| [Figma WASM case](https://www.figma.com/blog/webassembly-technology-study-case/) | The document is not the DOM | **Already shipped** (canvas board). Figma needed an engine for 10⁵ vector paths. We have ≤60 edges. Do not clone their renderer. |
| [Rerun](https://rerun.io/blog) / egui | Zero-copy buffers; WebGPU when the data is huge | **Phase 4** for the net (`gpu/`, ONNX WebGPU). Not for drawing a 5×5 grid. Keep the WASM API indices + typed arrays ([PLAN.md](../../PLAN.md) §13). |
| Makepad | UI as a game engine, no CSS/VDOM | **Reject for the HUD.** Mute, selects, dialogs, a11y, Vercel SPA stay React. A Makepad rewrite is a different app. |
| [egui](https://github.com/emilk/egui) | Immediate-mode, reconstruct UI every frame | **Reject for the HUD.** Optional later *board-only* experiment if Pixi particles never ship and Canvas2D is not enough — not a React replacement. |
| [Leptos](https://leptos.dev) | Fine-grained DOM signals, no VDOM | **Reject.** The board is not DOM. Replacing React does not remove Pixi, which is most of the warned chunk. |
| [Loro](https://loro.dev) / Automerge / Yrs | CRDT local-first, 10–100× JS | **Reject for gameplay.** PLAN.md §7 is authoritative server + optimistic apply. Turn-based D&B is not a CRDT document. Revisit only for a future shared scratchpad / replay annotation — never for `play(edge)`. |
| Polars WASM | Arrow analytics in a Worker | **Reject.** No client-side warehouse. |
| Biome playground | Rust AST in WASM on every keystroke | **Analog only:** Perfect/MCTS already belong in the Worker. Do not put more search on the main thread. |
| Zed / GPUI | Native GPU UI + WASM plugins | **Out of scope.** We are a browser SPA. |

## Goals

- Keep **one** game engine: `dab-core` native + WASM. JS never invents legality.
- Search stays in the **Worker** (KET-20, KET-57). Overlay `analyze()` may stay
  on the main thread (≤25 boxes, KET-21).
- Make the **JS shell** match that: the entry chunk must not include Pixi.
- Production WASM is **optimized** (`wasm-opt`); `--no-opt` is a local/CI
  speed knob, not the deploy artifact.
- Network path (Phase 3) gets the same ethos: **compact binary frames**
  (KET-25), not JSON forever, not a CRDT.

## Non-goals

- Rewriting `web/` in Makepad, egui, or Leptos.
- Dropping React for the HUD in this track.
- Recreating `gpu/` / `ai/` / `proto/` / `infra/` (KET-61).
- SharedArrayBuffer / `wasm-bindgen-rayon` until Phase 4 batched search
  actually needs threads beyond one Worker.
- Raising `build.chunkSizeWarningLimit` to silence Vercel.
- Shipping Pixi particles in this spec (still PLAN.md Phase 5 polish, if ever).

## Tracks (do not reorder past KET-25)

KET-25 stays the product next step. Tracks A and B are small enough to land
beside it. C *is* KET-25. D is Phase 4.

### Track A — Shell diet (do now; does not change renderer)

The architecture already has a loading gate (`status === 'ready'`). Use it.

1. **`import()` PixiBoard** (and only PixiBoard) after WASM init, or behind
   `status === 'ready'`. HUD HTML/CSS/React can parse without the 286 kB
   Pixi share. This is the Figma lesson applied cheaply: the shell paints
   first; the document engine loads second.
2. **Release `wasm-opt`.** Split scripts: `build:wasm` may stay `--no-opt`
   for iteration; `pnpm build` / Vercel must run wasm-pack **without**
   `--no-opt` (or `-a` / `--release` + wasm-opt). Measure
   `dab_wasm_bg.wasm` before/after.
3. **`app.init({ preference: 'webgl' })`.** Does not shrink the entry chunk
   (renderers are already split). Prevents a WebGPU fallback download if
   Pixi’s default order changes. One-line, keep.
4. Leave Howler lazy (use-sound already `import('howler')`). Do not
   statically import it.

Acceptance:

- [ ] Entry JS chunk **under 500 kB** minified without raising the Vite
      limit. Pixi lives in a dynamic chunk.
- [ ] First paint of the HUD does not wait on `pixi.js`.
- [ ] Production `.wasm` is wasm-opt’d; size recorded in the PR.
- [ ] Worker still loads; KET-20 / KET-57 behavior unchanged.
- [ ] `pnpm --filter @dab/web lint` + `build` + tests green.

### Track B — Board renderer (decide, then one path)

Pixi is a **platform bet**, not a requirement of Dots and Boxes. Pick one:

| Option | When | Cost |
|--------|------|------|
| **B1. Keep Pixi** | We still intend Phase 5 chain-capture particles / sprite spectacle | Track A only. Pixi stays; it just must not be in the entry chunk. |
| **B2. Canvas 2D (or SVG) board** | We admit particles are not the next year of work | Delete `pixi.js`. Reuse `layout.ts`, `motion.ts`, hit pads, overlay/hint paint. Same `edgeId` callbacks. Largest byte win (~286 kB + WebGL chunks). |

Do **not** insert egui/Makepad as a third renderer “for craft.” That is a
rewrite of input, text, resize, StrictMode teardown, and hover SFX — to draw
the same 5×5 grid.

Decision rule: if the next visual work is not GPU particles, choose **B2**.
If it is, choose **B1** and live with Pixi after Track A.

Acceptance (B2 only):

- [ ] No `pixi.js` dependency. Board still presentational: snapshot in,
      `edgeId` out.
- [ ] Hover / claim / extra-turn / overlay / hint behavior preserved
      ([`phase1-board-polish.md`](./phase1-board-polish.md),
      [`phase2-pixi-incremental.md`](./phase2-pixi-incremental.md),
      [`phase2-theory-overlay.md`](./phase2-theory-overlay.md),
      [`phase2-hints.md`](./phase2-hints.md)).
- [ ] Architecture learning: PixiBoard → `Board.tsx` (or equivalent); update
      `.cursor/rules/architecture-learnings.mdc`.

### Track C — Same core on the wire (this *is* the list, applied to Phase 3)

1Password’s win was **one engine on every client**. Ours is one engine on
**browser WASM + server native**. KET-25 (binary protocol) is the
network analog of Loro’s compact encoding — without CRDTs.

- Clients send intents; server validates with `dab-core`; broadcast
  authoritative frames.
- Optimistic local `play()` + resync (PLAN.md §7). Not Yrs.
- Keep JSON only until the codec exists; do not add a second TS decoder that
  diverges from the Rust types.

No extra Linear work beyond KET-25 for this track.

### Track D — WebGPU where the data is huge (Phase 4)

Rerun/Makepad use the GPU because they push gigabytes or 120 FPS UI. Our
gigabyte-class work is **batched rollouts and the policy net**, not box
fills.

- `gpu/` + ONNX Runtime Web + WebGPU as already in PLAN.md Phase 4.
- WASM-CPU fallback for inference (PLAN.md §13).
- Still no search on the UI thread; Worker or `gpu` path only.

## Fence that must not move

From [`architecture-learnings`](../../.cursor/rules/architecture-learnings.mdc):

- Rules in WASM. Board does not invent legality.
- `runAiTurn` bound to `gameGeneration`.
- Perfect / `chooseMove` / Hint in the Worker.
- Audio never imports the store (types only).
- Do not grow the WASM JS API into rich objects. Indices and packed
  snapshots stay the interop currency (PLAN.md §13). If overlay dumps get
  large, switch `analyze()` to typed arrays — that is the Rerun zero-copy
  lesson at our scale.

## Files

| Path | Role |
|------|------|
| `docs/specs/wasm-first-client.md` | This spec |
| `web/src/App.tsx` | Dynamic import of the board (Track A) |
| `web/vite.config.ts` | Do not raise `chunkSizeWarningLimit` |
| `package.json` / CI | Release wasm-opt vs `--no-opt` for iterate |
| `web/src/board/PixiBoard.tsx` | Stay (B1) or replace (B2) |
| `PLAN.md` §8, §12 | Pointer; Pixi row becomes “current, under review” |

## Order of work

1. Track A (lazy board + wasm-opt + WebGL preference).
2. Explicit B1 vs B2 decision (one sentence in PLAN.md §15).
3. Track C = KET-25 as already scheduled.
4. Track D with Phase 4, not before.
