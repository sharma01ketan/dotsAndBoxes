# dab-wasm-az

Lazily-fetched WASM for AlphaZero inference (`tract`, CPU SIMD). Base
`@dab/dab-wasm` stays tract-free. Spec:
[`docs/specs/phase4-in-wasm-az.md`](../docs/specs/phase4-in-wasm-az.md).

## Build

From the repo root (separate from `pnpm build:wasm`):

```bash
pnpm build:wasm-az
```

`RUSTFLAGS=-C target-feature=+simd128`. Output: `wasm-az/pkg` as `@dab/dab-wasm-az`.

Slice B: `AzGame.chooseMoveAz` runs `AzEngine` PUCT. No HUD (slice E).
Endgame Perfect/CGT cutoff is slice C.
