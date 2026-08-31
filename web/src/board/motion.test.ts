import assert from 'node:assert/strict';
import { afterEach, describe, it } from 'node:test';
import { animate } from './motion.ts';

type MatchMediaFn = (query: string) => { matches: boolean };

describe('animate reduced motion', () => {
  const g = globalThis as typeof globalThis & { matchMedia?: MatchMediaFn };
  const prev = g.matchMedia;

  afterEach(() => {
    if (prev === undefined) {
      delete g.matchMedia;
    } else {
      g.matchMedia = prev;
    }
  });

  it('calls onUpdate(1) and onDone synchronously when reduce is preferred', () => {
    g.matchMedia = (query: string) => ({
      matches: query === '(prefers-reduced-motion: reduce)',
    });

    let update = -1;
    let done = false;
    animate(
      180,
      (t) => {
        update = t;
      },
      () => {
        done = true;
      },
    );

    assert.equal(update, 1);
    assert.equal(done, true);
  });
});
