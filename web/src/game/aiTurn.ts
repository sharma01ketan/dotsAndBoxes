/** CPU extra-turn loop (KET-57). Search stays in WASM; this file only schedules. */

export type AiSnap = {
  isTerminal: boolean;
  currentPlayer: number;
};

export type AiPlayResult = {
  isTerminal: boolean;
};

export type AiTurnApi<T extends AiPlayResult = AiPlayResult> = {
  startedGeneration: number;
  startedMode: string;
  getGeneration: () => number;
  getMode: () => string;
  getSnap: () => AiSnap | null;
  getGame: () => {
    chooseMove: (
      policy: number,
      seed: bigint,
    ) => number | Promise<number>;
  } | null;
  getSeed: () => { gameSeed: number; moveCount: number };
  applyPlay: (edge: number) => T | null;
  onRecoverableFail: (message: string) => void;
  signal: AbortSignal;
};

export function isStale(api: AiTurnApi): boolean {
  return (
    api.getGeneration() !== api.startedGeneration ||
    api.getMode() !== api.startedMode
  );
}

export function cpuToMove(snap: AiSnap | null, policy: number | null): boolean {
  return (
    !!snap &&
    !snap.isTerminal &&
    snap.currentPlayer === 1 &&
    policy !== null
  );
}

export function sleep(ms: number, signal?: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal?.aborted) {
      resolve();
      return;
    }
    const t = setTimeout(resolve, ms);
    const onAbort = () => {
      clearTimeout(t);
      resolve();
    };
    signal?.addEventListener('abort', onAbort, { once: true });
  });
}

/**
 * Choose and apply CPU moves until P1 to move, terminal, abort, or failure.
 * Caller sets `aiBusy` around this. Does not clear busy.
 */
export async function runAiTurnLoop<T extends AiPlayResult>(
  api: AiTurnApi<T>,
  policy: number,
  pauseMs: number,
  onStep: (outcome: T) => void,
  wait: (ms: number, signal: AbortSignal) => Promise<void> = sleep,
): Promise<'done' | 'aborted' | 'error'> {
  const needsAi = () => cpuToMove(api.getSnap(), policy);

  while (needsAi()) {
    if (isStale(api) || api.signal.aborted) return 'aborted';

    const game = api.getGame();
    if (!game) return 'aborted';

    const { gameSeed, moveCount } = api.getSeed();
    const seed = BigInt(gameSeed) + BigInt(moveCount) * 0x100000001b3n;

    let edge: number;
    try {
      edge = await Promise.resolve(game.chooseMove(policy, seed));
    } catch (err) {
      api.onRecoverableFail(
        err instanceof Error ? err.message : String(err),
      );
      return 'error';
    }

    if (isStale(api) || api.signal.aborted) return 'aborted';

    const outcome = api.applyPlay(edge);
    if (!outcome) {
      api.onRecoverableFail('CPU move could not be applied');
      return 'error';
    }
    onStep(outcome);
    if (outcome.isTerminal) return 'done';

    await wait(pauseMs, api.signal);
  }

  if (isStale(api) || api.signal.aborted) return 'aborted';
  return 'done';
}
