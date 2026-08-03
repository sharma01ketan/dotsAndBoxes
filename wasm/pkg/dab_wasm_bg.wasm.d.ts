/* tslint:disable */
/* eslint-disable */
export const memory: WebAssembly.Memory;
export const __wbg_wasmgame_free: (a: number, b: number) => void;
export const init_panic_hook: () => void;
export const wasmgame_boxCount: (a: number) => number;
export const wasmgame_boxOwner: (a: number, b: number) => number;
export const wasmgame_cols: (a: number) => number;
export const wasmgame_currentPlayer: (a: number) => number;
export const wasmgame_edgeCoord: (a: number, b: number) => [number, number, number, number];
export const wasmgame_edgeCount: (a: number) => number;
export const wasmgame_edgeId: (a: number, b: number, c: number, d: number) => [number, number, number];
export const wasmgame_edgeIsDrawn: (a: number, b: number) => number;
export const wasmgame_isLegal: (a: number, b: number) => number;
export const wasmgame_isTerminal: (a: number) => number;
export const wasmgame_legalMoves: (a: number) => [number, number];
export const wasmgame_new: (a: number, b: number) => [number, number, number];
export const wasmgame_play: (a: number, b: number) => [number, number, number, number];
export const wasmgame_rows: (a: number) => number;
export const wasmgame_scoreP1: (a: number) => number;
export const wasmgame_scoreP2: (a: number) => number;
export const wasmgame_winner: (a: number) => number;
export const __wbindgen_externrefs: WebAssembly.Table;
export const __externref_table_dealloc: (a: number) => void;
export const __wbindgen_free: (a: number, b: number, c: number) => void;
export const __wbindgen_start: () => void;
