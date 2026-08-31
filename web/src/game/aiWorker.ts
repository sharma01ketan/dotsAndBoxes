/**
 * Shadow WASM game for chooseMove / perfectValue (KET-20).
 * The UI thread keeps its own WasmGame for play + snapshot.
 */
import init, { WasmGame } from '@dab/dab-wasm';
import { createWasmInit } from '../lib/wasmInit';
import type { AiWorkerRequest, AiWorkerResponse } from './aiWorkerProtocol';

const ensureInit = createWasmInit(() => init());
let game: WasmGame | null = null;

function reply(msg: AiWorkerResponse) {
  postMessage(msg);
}

async function handle(msg: AiWorkerRequest) {
  try {
    await ensureInit();
    switch (msg.type) {
      case 'newGame': {
        game?.free();
        game = new WasmGame(msg.rows, msg.cols);
        reply({ id: msg.id, ok: true });
        return;
      }
      case 'play': {
        if (!game) throw new Error('no game');
        game.play(msg.edge);
        reply({ id: msg.id, ok: true });
        return;
      }
      case 'chooseMove': {
        if (!game) throw new Error('no game');
        const value = game.chooseMove(msg.policy, msg.seed);
        reply({ id: msg.id, ok: true, value });
        return;
      }
      case 'perfectValue': {
        if (!game) throw new Error('no game');
        const value = game.perfectValue();
        reply({ id: msg.id, ok: true, value });
        return;
      }
      default:
        throw new Error('unknown worker message');
    }
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err);
    reply({ id: msg.id, ok: false, error });
  }
}

let chain: Promise<void> = Promise.resolve();

onmessage = (event: MessageEvent<AiWorkerRequest>) => {
  chain = chain.then(() => handle(event.data));
};
