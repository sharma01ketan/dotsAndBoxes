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

/** Local Opponent (two humans) or human (P1) vs CPU (P2). Id `hotseat` is stable. */
export type PlayMode = 'hotseat' | 'vs-random' | 'vs-greedy' | 'vs-cgt';

export const PLAY_MODES: { id: PlayMode; label: string }[] = [
  { id: 'hotseat', label: 'Opponent' },
  { id: 'vs-random', label: 'vs Random' },
  { id: 'vs-greedy', label: 'vs Greedy' },
  { id: 'vs-cgt', label: 'vs CGT' },
];

const POLICY_RANDOM = 0;
const POLICY_GREEDY = 1;
const POLICY_CGT = 2;

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

function policyForMode(mode: PlayMode): number | null {
  if (mode === 'vs-random') return POLICY_RANDOM;
  if (mode === 'vs-greedy') return POLICY_GREEDY;
  if (mode === 'vs-cgt') return POLICY_CGT;
  return null;
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => {
    setTimeout(resolve, ms);
  });
}

type GameState = {
  status: GameStatus;
  error: string | null;
  message: string;
  boardSize: number;
  mode: PlayMode;
  /** True while the CPU is choosing/playing (blocks human input). */
  aiBusy: boolean;
  gameSeed: number;
  /** Bumped on every newGame so the board can full-rebuild (same size reset). */
  gameGeneration: number;
  moveCount: number;
  game: WasmGame | null;
  edgeOwner: Int8Array;
  snap: GameSnapshot | null;
  init: () => Promise<void>;
  newGame: (size?: number) => void;
  setBoardSize: (size: number) => void;
  setMode: (mode: PlayMode) => void;
  /** Human edge click. No-op during AI turn or when it is CPU's seat. */
  play: (edge: number) => PlayOutcome | null;
  /**
   * Run CPU turns until human to move or terminal.
   * Yields between moves so motion/SFX can start.
   */
  runAiTurn: (
    pauseMs: number,
    onStep: (outcome: PlayOutcome) => void,
  ) => Promise<void>;
};

function applyPlay(
  get: () => GameState,
  set: (
    partial:
      | Partial<GameState>
      | ((state: GameState) => Partial<GameState>),
  ) => void,
  edge: number,
): PlayOutcome | null {
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
      moveCount: get().moveCount + 1,
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
}

export const useGameStore = create<GameState>((set, get) => ({
  status: 'idle',
  error: null,
  message: '',
  boardSize: DEFAULT_BOARD,
  mode: 'vs-greedy',
  aiBusy: false,
  gameSeed: 1,
  gameGeneration: 0,
  moveCount: 0,
  game: null,
  edgeOwner: new Int8Array(0),
  snap: null,

  async init() {
    if (get().status === 'loading' || get().status === 'ready') return;
    set({ status: 'loading', error: null });
    try {
      await initWasm();
      get().newGame(get().boardSize);
      set({ status: 'ready', message: 'Ready — click an edge to play' });
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

  setMode(mode: PlayMode) {
    set({ mode });
    get().newGame();
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
    const mode = get().mode;
    set({
      boardSize: clamped,
      game,
      edgeOwner,
      snap: snapshotFrom(game, edgeOwner),
      message: `New ${clamped}×${clamped} ${PLAY_MODES.find((m) => m.id === mode)?.label ?? mode}`,
      error: null,
      aiBusy: false,
      gameSeed: (get().gameSeed + 0x9e37_79b9) >>> 0 || 1,
      gameGeneration: get().gameGeneration + 1,
      moveCount: 0,
    });
  },

  play(edge: number) {
    const { aiBusy, mode, snap } = get();
    if (aiBusy) return null;
    if (!snap || snap.isTerminal) return null;
    // In vs-AI, human is always P1.
    if (mode !== 'hotseat' && snap.currentPlayer !== 0) return null;
    return applyPlay(get, set, edge);
  },

  async runAiTurn(pauseMs, onStep) {
    const policy = policyForMode(get().mode);
    if (policy === null) return;
    if (get().aiBusy) return;

    const needsAi = () => {
      const s = get().snap;
      return (
        !!s &&
        !s.isTerminal &&
        s.currentPlayer === 1 &&
        policyForMode(get().mode) !== null
      );
    };

    if (!needsAi()) return;

    set({ aiBusy: true, message: 'CPU thinking…' });
    try {
      while (needsAi()) {
        const { game, gameSeed, moveCount } = get();
        if (!game) break;
        const seed = BigInt(gameSeed) + BigInt(moveCount) * 0x100000001b3n;
        let edge: number;
        try {
          edge = game.chooseMove(policy, seed);
        } catch (err) {
          set({
            message: err instanceof Error ? err.message : String(err),
          });
          break;
        }
        const outcome = applyPlay(get, set, edge);
        if (!outcome) break;
        onStep(outcome);
        if (outcome.isTerminal) break;
        await sleep(pauseMs);
      }
    } finally {
      set({ aiBusy: false });
    }
  },
}));

export function modeTitle(mode: PlayMode): string {
  switch (mode) {
    case 'vs-random':
      return 'You vs Random';
    case 'vs-greedy':
      return 'You vs Greedy';
    case 'vs-cgt':
      return 'You vs CGT';
    default:
      return 'Opponent';
  }
}

export function modeLede(mode: PlayMode): string {
  switch (mode) {
    case 'vs-random':
      return 'You are P1. The CPU picks legal edges at random.';
    case 'vs-greedy':
      return 'You are P1. The CPU takes free boxes and avoids giving them away.';
    case 'vs-cgt':
      return 'You are P1. The CPU keeps chain control (double-cross / all-but-four).';
    default:
      return 'Two players, one screen. Click an edge to draw.';
  }
}

export function scoreLabelP1(mode: PlayMode): string {
  return mode === 'hotseat' ? 'P1' : 'You';
}

export function scoreLabelP2(mode: PlayMode): string {
  switch (mode) {
    case 'vs-random':
      return 'CPU (Random)';
    case 'vs-greedy':
      return 'CPU (Greedy)';
    case 'vs-cgt':
      return 'CPU (CGT)';
    default:
      return 'P2';
  }
}

export { playerLabel, winnerLabel };
