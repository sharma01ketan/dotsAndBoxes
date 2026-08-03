import init, { WasmGame } from '@dab/dab-wasm';

let ready: Promise<void> | null = null;

/** Initialize the WASM module once. Safe to call multiple times. */
export function initWasm(): Promise<void> {
  if (!ready) {
    ready = init().then(() => undefined);
  }
  return ready;
}

export { WasmGame };

export function playerLabel(player: number): string {
  return player === 0 ? 'P1' : 'P2';
}

export function winnerLabel(code: number): string {
  switch (code) {
    case -1:
      return 'in progress';
    case 0:
      return 'P1';
    case 1:
      return 'P2';
    case 2:
      return 'Draw';
    default:
      return `unknown(${code})`;
  }
}
