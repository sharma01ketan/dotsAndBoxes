import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  cpuToMove,
  isStale,
  runAiTurnLoop,
  type AiTurnApi,
} from './aiTurn.ts';

function outcome(edgeId: number): {
  edgeId: number;
  mover: number;
  completed: number;
  boxIds: number[];
  extraTurn: boolean;
  isTerminal: boolean;
  winner: number;
} {
  return {
    edgeId,
    mover: 1,
    completed: 0,
    boxIds: [],
    extraTurn: false,
    isTerminal: false,
    winner: -1,
  };
}

function host(
  overrides: Partial<AiTurnApi> & Pick<AiTurnApi, 'getGame'>,
): AiTurnApi {
  const ac = new AbortController();
  return {
    startedGeneration: 1,
    startedMode: 'vs-greedy',
    getGeneration: () => 1,
    getMode: () => 'vs-greedy',
    getSnap: () => ({ isTerminal: false, currentPlayer: 1 }),
    getSeed: () => ({ gameSeed: 1, moveCount: 0 }),
    applyPlay: () => outcome(0),
    onRecoverableFail: () => {
      throw new Error('unexpected fail');
    },
    signal: ac.signal,
    ...overrides,
  };
}

describe('cpuToMove', () => {
  it('is true only for in-progress P2 vs-AI', () => {
    assert.equal(cpuToMove({ isTerminal: false, currentPlayer: 1 }, 1), true);
    assert.equal(cpuToMove({ isTerminal: false, currentPlayer: 0 }, 1), false);
    assert.equal(cpuToMove({ isTerminal: true, currentPlayer: 1 }, 1), false);
    assert.equal(cpuToMove(null, 1), false);
    assert.equal(cpuToMove({ isTerminal: false, currentPlayer: 1 }, null), false);
  });
});

describe('runAiTurnLoop', () => {
  it('keeps choosing while P2 has the extra-turn seat', async () => {
    const chosen: number[] = [];
    let player = 1;
    const api = host({
      getSnap: () => ({ isTerminal: false, currentPlayer: player }),
      getGame: () => ({
        chooseMove: () => {
          const edge = chosen.length === 0 ? 4 : 5;
          chosen.push(edge);
          return edge;
        },
      }),
      applyPlay: (edge) => {
        if (edge === 5) player = 0;
        return { ...outcome(edge), extraTurn: edge === 4 };
      },
    });

    const result = await runAiTurnLoop(api, 1, 0, () => {}, async () => {});
    assert.equal(result, 'done');
    assert.deepEqual(chosen, [4, 5]);
  });

  it('does not apply a move after New game mid-sleep', async () => {
    const applied: number[] = [];
    let generation = 1;
    let release: (() => void) | undefined;
    const ac = new AbortController();

    const api = host({
      startedGeneration: 1,
      getGeneration: () => generation,
      signal: ac.signal,
      getGame: () => ({
        chooseMove: () => 9,
      }),
      applyPlay: (edge) => {
        applied.push(edge);
        return outcome(edge);
      },
    });

    const running = runAiTurnLoop(api, 1, 50, () => {}, () => {
      return new Promise((resolve) => {
        release = resolve;
      });
    });

    await Promise.resolve();
    assert.deepEqual(applied, [9]);

    generation = 2;
    ac.abort();
    release?.();

    assert.equal(await running, 'aborted');
    assert.deepEqual(applied, [9]);
  });

  it('does not apply until async chooseMove resolves, and abort skips apply', async () => {
    const applied: number[] = [];
    let finish: ((edge: number) => void) | undefined;
    const ac = new AbortController();
    const api = host({
      signal: ac.signal,
      getGame: () => ({
        chooseMove: () =>
          new Promise<number>((resolve) => {
            finish = resolve;
          }),
      }),
      applyPlay: (edge) => {
        applied.push(edge);
        return outcome(edge);
      },
    });

    const running = runAiTurnLoop(api, 1, 0, () => {}, async () => {});
    await Promise.resolve();
    assert.deepEqual(applied, []);

    ac.abort();
    finish?.(6);
    assert.equal(await running, 'aborted');
    assert.deepEqual(applied, []);
  });

  it('chooseMove throw does not apply and reports recoverable fail', async () => {
    const applied: number[] = [];
    let fail = '';
    const api = host({
      getGame: () => ({
        chooseMove: () => {
          throw new Error('engine exploded');
        },
      }),
      applyPlay: (edge) => {
        applied.push(edge);
        return outcome(edge);
      },
      onRecoverableFail: (message) => {
        fail = message;
      },
    });

    const result = await runAiTurnLoop(api, 1, 0, () => {});
    assert.equal(result, 'error');
    assert.equal(fail, 'engine exploded');
    assert.deepEqual(applied, []);
  });

  it('isStale when generation or mode changes', () => {
    const api = host({
      getGame: () => ({ chooseMove: () => 0 }),
      getGeneration: () => 2,
    });
    assert.equal(isStale(api), true);
    const modeShift = host({
      getGame: () => ({ chooseMove: () => 0 }),
      getMode: () => 'vs-random',
    });
    assert.equal(isStale(modeShift), true);
  });
});
