/* tslint:disable */
/* eslint-disable */

export function AZ_CHANNELS(): number;

export function AZ_PLANE(): number;

export function AZ_POLICY(): number;

/**
 * Mirror of the Worker's `WasmGame`. Own state; does not share the base module.
 */
export class AzGame {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * PUCT search. Requires a loaded model. Does not apply the chosen edge.
     * Endgame Perfect/CGT handoff is slice C.
     */
    chooseMoveAz(last_move: number, sims: number, seed: bigint): number;
    constructor(rows: number, cols: number);
    /**
     * Keep the mirror in sync. Does not search.
     */
    play(edge: number): void;
    /**
     * Net policy argmax over legal moves. Requires a loaded model. No tree search.
     */
    policyArgmax(last_move: number): number;
}

/**
 * Loaded sidecar JSON, or `""` if none.
 */
export function azModelStamp(): string;

export function init_panic_hook(): void;

/**
 * Parse ONNX via tract, validate the sidecar stamp, store the model.
 *
 * Throws on any stamp mismatch. Slice A: a fixture net is enough; HUD is slice E.
 */
export function loadAzModel(onnx: Uint8Array, sidecar: string): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly AZ_CHANNELS: () => number;
    readonly AZ_PLANE: () => number;
    readonly AZ_POLICY: () => number;
    readonly __wbg_azgame_free: (a: number, b: number) => void;
    readonly azModelStamp: () => [number, number];
    readonly azgame_chooseMoveAz: (a: number, b: number, c: number, d: bigint) => [number, number, number];
    readonly azgame_new: (a: number, b: number) => [number, number, number];
    readonly azgame_play: (a: number, b: number) => [number, number];
    readonly azgame_policyArgmax: (a: number, b: number) => [number, number, number];
    readonly loadAzModel: (a: number, b: number, c: number, d: number) => [number, number];
    readonly init_panic_hook: () => void;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
