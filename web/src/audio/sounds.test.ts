import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { sfxForPlay, withinGap } from './sounds.ts';

describe('sfxForPlay', () => {
  it('emits draw for a normal edge', () => {
    assert.deepEqual(
      sfxForPlay({
        completed: 0,
        extraTurn: false,
        isTerminal: false,
        winner: -1,
      }),
      ['draw'],
    );
  });

  it('emits claim then win on a finishing capture', () => {
    assert.deepEqual(
      sfxForPlay({
        completed: 2,
        extraTurn: true,
        isTerminal: true,
        winner: 0,
      }),
      ['claim', 'win'],
    );
  });

  it('emits draw then tie on a drawn game', () => {
    assert.deepEqual(
      sfxForPlay({
        completed: 0,
        extraTurn: false,
        isTerminal: true,
        winner: 2,
      }),
      ['draw', 'tie'],
    );
  });
});

describe('withinGap', () => {
  it('blocks calls inside the gap', () => {
    assert.equal(withinGap(0, 50, 100), true);
    assert.equal(withinGap(0, 100, 100), false);
  });
});
