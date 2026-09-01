import init, {
  WasmGame,
  POLICY_RANDOM,
  POLICY_GREEDY,
  POLICY_CGT,
  POLICY_PERFECT,
  POLICY_MCTS,
  POLICY_AZ,
  isPerfectHudSize,
} from '@dab/dab-wasm';
import { createWasmInit } from './wasmInit';

const runInit = createWasmInit(() => init());

/** Initialize the WASM module once. Safe to call multiple times. */
export function initWasm(): Promise<void> {
  return runInit();
}

export {
  WasmGame,
  POLICY_RANDOM,
  POLICY_GREEDY,
  POLICY_CGT,
  POLICY_PERFECT,
  POLICY_MCTS,
  POLICY_AZ,
  isPerfectHudSize,
};

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
