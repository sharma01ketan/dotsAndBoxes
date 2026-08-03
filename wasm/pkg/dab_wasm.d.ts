/* tslint:disable */
/* eslint-disable */

/**
 * Browser-facing game handle wrapping [`Game`].
 */
export class WasmGame {
    free(): void;
    [Symbol.dispose](): void;
    boxCount(): number;
    /**
     * `-1` unclaimed, `0` P1, `1` P2.
     */
    boxOwner(box_id: number): number;
    /**
     * Current player: `0` = P1, `1` = P2.
     */
    currentPlayer(): number;
    /**
     * Resolve edge id → `[orientation, row, col]` (`orientation`: 0=H, 1=V).
     */
    edgeCoord(edge: number): Uint16Array;
    edgeCount(): number;
    /**
     * Resolve `(orientation, row, col)` → edge id (`orientation`: 0=H, 1=V).
     */
    edgeId(orientation: number, row: number, col: number): number;
    edgeIsDrawn(edge: number): boolean;
    isLegal(edge: number): boolean;
    isTerminal(): boolean;
    /**
     * Undrawn edge ids (JS: `Uint16Array`).
     */
    legalMoves(): Uint16Array;
    /**
     * Create a new game with `rows × cols` boxes.
     */
    constructor(rows: number, cols: number);
    /**
     * Play an edge. Returns `[extraTurn (0/1), completedCount, ...completedBoxIds]`.
     */
    play(edge: number): Uint16Array;
    scoreP1(): number;
    scoreP2(): number;
    /**
     * `-1` in progress, `0` P1 wins, `1` P2 wins, `2` draw.
     */
    winner(): number;
    readonly cols: number;
    readonly rows: number;
}

export function init_panic_hook(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_wasmgame_free: (a: number, b: number) => void;
    readonly init_panic_hook: () => void;
    readonly wasmgame_boxCount: (a: number) => number;
    readonly wasmgame_boxOwner: (a: number, b: number) => number;
    readonly wasmgame_cols: (a: number) => number;
    readonly wasmgame_currentPlayer: (a: number) => number;
    readonly wasmgame_edgeCoord: (a: number, b: number) => [number, number, number, number];
    readonly wasmgame_edgeCount: (a: number) => number;
    readonly wasmgame_edgeId: (a: number, b: number, c: number, d: number) => [number, number, number];
    readonly wasmgame_edgeIsDrawn: (a: number, b: number) => number;
    readonly wasmgame_isLegal: (a: number, b: number) => number;
    readonly wasmgame_isTerminal: (a: number) => number;
    readonly wasmgame_legalMoves: (a: number) => [number, number];
    readonly wasmgame_new: (a: number, b: number) => [number, number, number];
    readonly wasmgame_play: (a: number, b: number) => [number, number, number, number];
    readonly wasmgame_rows: (a: number) => number;
    readonly wasmgame_scoreP1: (a: number) => number;
    readonly wasmgame_scoreP2: (a: number) => number;
    readonly wasmgame_winner: (a: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
