import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  HOVER_SFX_DEBOUNCE_MS,
  initialHoverSfxState,
  reduceHoverSfx,
} from './edgeHoverSfx.ts';

function apply(
  state: ReturnType<typeof initialHoverSfxState>,
  event: Parameters<typeof reduceHoverSfx>[1],
) {
  return reduceHoverSfx(state, event, 90, 220);
}

describe('reduceHoverSfx', () => {
  it('plays after debounce on first enter', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 0, now: 1000 });
    assert.equal(r.state.phase.kind, 'pending');
    assert.ok(r.effects.some((e) => e.type === 'schedule' && e.edgeId === 0));

    r = apply(r.state, { type: 'tick', now: 1000 + HOVER_SFX_DEBOUNCE_MS });
    assert.equal(r.state.phase.kind, 'armed');
    assert.deepEqual(r.effects, [{ type: 'play', edgeId: 0 }]);
  });

  it('leave during debounce cancels play (no orphan tick play)', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 3, now: 0 });
    r = apply(r.state, { type: 'leave', edgeId: 3, now: 40 });
    assert.equal(r.state.phase.kind, 'idle');
    assert.ok(r.effects.some((e) => e.type === 'cancelSchedule'));

    // Stale tick must not play.
    r = apply(r.state, { type: 'tick', now: 90 });
    assert.equal(r.effects.some((e) => e.type === 'play'), false);
  });

  it('rebuild during pending returns to idle so a new enter can arm', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 1, now: 0 });
    r = apply(r.state, { type: 'rebuild', now: 30 });
    assert.equal(r.state.phase.kind, 'idle');

    r = apply(r.state, { type: 'enter', edgeId: 1, now: 31 });
    assert.equal(r.state.phase.kind, 'pending');
    r = apply(r.state, { type: 'tick', now: 121 });
    assert.deepEqual(r.effects, [{ type: 'play', edgeId: 1 }]);
  });

  it('press suppresses hover so click does not also play hover', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 2, now: 0 });
    r = apply(r.state, { type: 'press', now: 10 });
    assert.equal(r.state.phase.kind, 'suppressed');

    r = apply(r.state, { type: 'enter', edgeId: 2, now: 50 });
    assert.equal(r.state.phase.kind, 'suppressed');
    assert.equal(r.effects.length, 0);

    // After suppress window, enter arms again.
    r = apply(r.state, { type: 'enter', edgeId: 2, now: 250 });
    assert.equal(r.state.phase.kind, 'pending');
  });

  it('rebuild while armed returns to idle so a new enter can arm', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 4, now: 0 });
    r = apply(r.state, { type: 'tick', now: HOVER_SFX_DEBOUNCE_MS });
    assert.equal(r.state.phase.kind, 'armed');

    r = apply(r.state, { type: 'rebuild', now: 100 });
    assert.equal(r.state.phase.kind, 'idle');

    r = apply(r.state, { type: 'enter', edgeId: 4, now: 101 });
    assert.equal(r.state.phase.kind, 'pending');
    r = apply(r.state, { type: 'tick', now: 101 + HOVER_SFX_DEBOUNCE_MS });
    assert.deepEqual(r.effects, [{ type: 'play', edgeId: 4 }]);
  });

  it('re-enter same armed edge does not re-schedule', () => {
    const s = initialHoverSfxState();
    let r = apply(s, { type: 'enter', edgeId: 5, now: 0 });
    r = apply(r.state, { type: 'tick', now: 90 });
    assert.equal(r.state.phase.kind, 'armed');

    r = apply(r.state, { type: 'enter', edgeId: 5, now: 100 });
    assert.equal(r.state.phase.kind, 'armed');
    assert.equal(r.effects.length, 0);
  });
});
