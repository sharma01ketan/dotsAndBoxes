import { create } from 'zustand';
import {
  initWasm,
  playerLabel,
  winnerLabel,
  WasmGame,
} from '../lib/wasmGame';

export const MIN_BOARD = 2;
export const MAX_BOARD = 5;
export const DEFAULT_BOARD = 3;

export type GameStatus = 'idle' | 'loading' | 'ready' | 'error';

/** Result of a successful `play` — UI/SFX/motion consume this; rules stay in WASM. */
export type PlayOutcome = {
  edgeId: number;
  /** Player who drew the edge (0 P1, 1 P2). */
  mover: number;
  completed: number;
  /** Box indices completed by this move (may be empty). */
  boxIds: number[];
  extraTurn: boolean;
  isTerminal: boolean;
  /** -1 in progress, 0 P1, 1 P2, 2 draw */
  winner: number;
};

export type GameSnapshot = {
  rows: number;
  cols: number;
  currentPlayer: number;
  scoreP1: number;
  scoreP2: number;
  isTerminal: boolean;
  winner: number;
  edgeCount: number;
  boxCount: number;
  /** -1 undrawn, 0 P1, 1 P2 */
  edgeOwner: Int8Array;
  /** -1 unclaimed, 0 P1, 1 P2 */
  boxOwner: Int8Array;
  legalMoves: number[];
};

function emptyOwners(edgeCount: number, boxCount: number): {
  edgeOwner: Int8Array;
  boxOwner: Int8Array;
} {
  return {
    edgeOwner: new Int8Array(edgeCount).fill(-1),
    boxOwner: new Int8Array(boxCount).fill(-1),
  };
}

function readBoxOwners(game: WasmGame): Int8Array {
  const n = game.boxCount();
  const out = new Int8Array(n);
  for (let i = 0; i < n; i++) {
    out[i] = game.boxOwner(i);
  }
  return out;
}

function snapshotFrom(
  game: WasmGame,
  edgeOwner: Int8Array,
): GameSnapshot {
  return {
    rows: game.rows,
    cols: game.cols,
    currentPlayer: game.currentPlayer(),
    scoreP1: game.scoreP1(),
    scoreP2: game.scoreP2(),
    isTerminal: game.isTerminal(),
    winner: game.winner(),
    edgeCount: game.edgeCount(),
    boxCount: game.boxCount(),
    edgeOwner: new Int8Array(edgeOwner),
    boxOwner: readBoxOwners(game),
    legalMoves: Array.from(game.legalMoves()),
  };
}

type GameState = {
  status: GameStatus;
  error: string | null;
  message: string;
  boardSize: number;
  game: WasmGame | null;
  edgeOwner: Int8Array;
  snap: GameSnapshot | null;
  init: () => Promise<void>;
  newGame: (size?: number) => void;
  setBoardSize: (size: number) => void;
  /** Returns outcome for SFX layer; null if the move was ignored. */
  play: (edge: number) => PlayOutcome | null;
};

export const useGameStore = create<GameState>((set, get) => ({
  status: 'idle',
  error: null,
  message: '',
  boardSize: DEFAULT_BOARD,
  game: null,
  edgeOwner: new Int8Array(0),
  snap: null,

  async init() {
    if (get().status === 'loading' || get().status === 'ready') return;
    set({ status: 'loading', error: null });
    try {
      await initWasm();
      get().newGame(get().boardSize);
      set({ status: 'ready', message: 'Hotseat ready — click an edge to play' });
    } catch (err) {
      set({
        status: 'error',
        error: err instanceof Error ? err.message : String(err),
      });
    }
  },

  setBoardSize(size: number) {
    const clamped = Math.min(MAX_BOARD, Math.max(MIN_BOARD, Math.floor(size)));
    set({ boardSize: clamped });
  },

  newGame(size?: number) {
    const prev = get().game;
    if (prev) {
      prev.free();
    }
    const n = size ?? get().boardSize;
    const clamped = Math.min(MAX_BOARD, Math.max(MIN_BOARD, Math.floor(n)));
    const game = new WasmGame(clamped, clamped);
    const { edgeOwner } = emptyOwners(game.edgeCount(), game.boxCount());
    set({
      boardSize: clamped,
      game,
      edgeOwner,
      snap: snapshotFrom(game, edgeOwner),
      message: `New ${clamped}×${clamped} game`,
      error: null,
    });
  },

  play(edge: number) {
    const { game, edgeOwner, snap } = get();
    if (!game || !snap || snap.isTerminal) return null;
    if (!game.isLegal(edge)) {
      set({ message: `Edge #${edge} is not playable` });
      return null;
    }

    const mover = game.currentPlayer();
    try {
      const result = game.play(edge);
      const nextOwners = new Int8Array(edgeOwner);
      nextOwners[edge] = mover as 0 | 1;

      const extraTurn = result[0] === 1;
      const completed = result[1] ?? 0;
      const boxIds: number[] = [];
      for (let i = 0; i < completed; i++) {
        const id = result[2 + i];
        if (id !== undefined) boxIds.push(id);
      }
      const msg = extraTurn
        ? `${playerLabel(mover)} claimed ${completed} box(es) — extra turn`
        : `${playerLabel(mover)} drew edge #${edge}`;

      const nextSnap = snapshotFrom(game, nextOwners);
      let banner = msg;
      if (nextSnap.isTerminal) {
        banner = `${msg}. Winner: ${winnerLabel(nextSnap.winner)}`;
      }

      set({
        edgeOwner: nextOwners,
        snap: nextSnap,
        message: banner,
      });

      return {
        edgeId: edge,
        mover,
        completed,
        boxIds,
        extraTurn,
        isTerminal: nextSnap.isTerminal,
        winner: nextSnap.winner,
      };
    } catch (err) {
      set({
        message: err instanceof Error ? err.message : String(err),
      });
      return null;
    }
  },
}));

export { playerLabel, winnerLabel };
