# dab-wasm

Browser-facing WASM bindings for `dab-core`.

## Build

From the repo root:

```bash
pnpm build:wasm
# or:
wasm-pack build wasm --target web --out-dir pkg --scope dab
```

Output lands in `wasm/pkg/` as the pnpm package `@dab/dab-wasm`.

## JS API (data-oriented)

```ts
import init, { WasmGame } from '@dab/dab-wasm';

await init();
const game = new WasmGame(2, 2);

game.currentPlayer(); // 0 | 1
game.scoreP1();
game.scoreP2();
game.legalMoves(); // Uint16Array of edge ids
game.play(edgeId); // Uint16Array: [extraTurn, count, ...boxIds]
game.isTerminal();
game.winner(); // -1 in progress, 0 P1, 1 P2, 2 draw
game.edgeIsDrawn(id);
game.boxOwner(boxId); // -1 none, 0 P1, 1 P2
game.edgeCoord(id); // [orient, row, col]  orient: 0=H, 1=V
game.edgeId(orient, row, col);
```

Rebuild whenever `core/` or `wasm/src` changes, then reinstall if needed:

```bash
pnpm build:wasm && pnpm install
```
