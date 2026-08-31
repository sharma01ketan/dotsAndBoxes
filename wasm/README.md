# dab-wasm

Browser-facing WASM bindings for `dab-core`.

## Build

From the repo root:

```bash
pnpm build:wasm
# or:
wasm-pack build wasm --target web --out-dir pkg --scope dab --no-opt
```

Output lands in `wasm/pkg/` as the pnpm package `@dab/dab-wasm`.

## JS API (data-oriented)

```ts
import init, {
  WasmGame,
  POLICY_GREEDY,
  isPerfectHudSize,
} from '@dab/dab-wasm';

await init();
const game = new WasmGame(2, 2);

game.currentPlayer(); // 0 | 1
game.scoreP1();
game.scoreP2();
game.legalMoves(); // Uint16Array of edge ids
game.play(edgeId); // Uint16Array: [extraTurn, count, ...boxIds]
game.analyze(); // Uint16Array CGT dump (does not play)
game.chooseMove(policy, seed); // legal edge, does not play
game.isTerminal();
game.winner(); // -1 in progress, 0 P1, 1 P2, 2 draw
game.edgeIsDrawn(id);
game.boxOwner(boxId); // -1 none or out of range, 0 P1, 1 P2
game.edgeCoord(id); // [orient, row, col]  orient: 0=H, 1=V
game.edgeId(orient, row, col);
POLICY_GREEDY(); // 1
POLICY_MCTS(); // 4
isPerfectHudSize(2, 2); // true
```

Rebuild whenever `core/` or `wasm/src` changes, then reinstall if needed:

```bash
pnpm build:wasm && pnpm install
```
