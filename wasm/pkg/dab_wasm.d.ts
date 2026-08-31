/* tslint:disable */
/* eslint-disable */

export function POLICY_CGT(): number;

export function POLICY_GREEDY(): number;

export function POLICY_MCTS(): number;

export function POLICY_PERFECT(): number;

export function POLICY_RANDOM(): number;

/**
 * Browser-facing game handle wrapping [`Game`].
 */
export class WasmGame {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Compact CGT analysis dump (KET-21). Does not mutate the game.
     */
    analyze(): Uint16Array;
    boxCount(): number;
    /**
     * `-1` unclaimed or out of range, `0` P1, `1` P2.
     */
    boxOwner(box_id: number): number;
    /**
     * Choose a legal edge without applying it.
     *
     * `policy`: `0` = random, `1` = greedy, `2` = CGT, `3` = Perfect, `4` = MCTS.
     * `seed` seeds the engine RNG.
     */
    chooseMove(policy: number, seed: bigint): number;
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
     * Box-difference margin for the side to move (2×2 / 3×3 only).
     */
    perfectValue(): number;
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

/**
 * Square 2×2 / 3×3 only. HUD and `chooseMove(3)` share this.
 */
export function isPerfectHudSize(rows: number, cols: number): boolean;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly POLICY_CGT: () => number;
    readonly POLICY_GREEDY: () => number;
    readonly POLICY_MCTS: () => number;
    readonly POLICY_PERFECT: () => number;
    readonly POLICY_RANDOM: () => number;
    readonly __wbg_wasmgame_free: (a: number, b: number) => void;
    readonly isPerfectHudSize: (a: number, b: number) => number;
    readonly wasmgame_analyze: (a: number) => [number, number];
    readonly wasmgame_boxCount: (a: number) => number;
    readonly wasmgame_boxOwner: (a: number, b: number) => number;
    readonly wasmgame_chooseMove: (a: number, b: number, c: bigint) => [number, number, number];
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
    readonly wasmgame_perfectValue: (a: number) => [number, number, number];
    readonly wasmgame_play: (a: number, b: number) => [number, number, number, number];
    readonly wasmgame_rows: (a: number) => number;
    readonly wasmgame_scoreP1: (a: number) => number;
    readonly wasmgame_scoreP2: (a: number) => number;
    readonly wasmgame_winner: (a: number) => number;
    readonly init_panic_hook: () => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __externref_table_dealloc: (a: number) => void;
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
