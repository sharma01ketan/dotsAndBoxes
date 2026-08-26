/**
 * Edge hover SFX gate — one clear session model so rebuild/click races
 * cannot permanently silence the first hover.
 *
 * Why a state machine (not ad-hoc flags):
 * - Pixi rewires hit targets on paint; that can fire leave/enter or drop
 *   listeners mid-debounce.
 * - A click is pointerover → pointerdown → pointertap (and often a rebuild
 *   that re-fires over). Hover SFX must not stack with draw SFX.
 * - Overloading one `edgeId` as both "hovered" and "already armed" caused
 *   leave-during-debounce to cancel the play (timer still fired but id was
 *   cleared), and stuck ids to skip the next real hover.
 *
 * Events: enter / leave / press / rebuild / tick.
 */

export const HOVER_SFX_DEBOUNCE_MS = 90;
export const HOVER_SFX_SUPPRESS_AFTER_PRESS_MS = 220;

export type HoverSfxPhase =
  | { kind: 'idle' }
  | { kind: 'pending'; edgeId: number; playAt: number }
  | { kind: 'armed'; edgeId: number }
  | { kind: 'suppressed'; until: number };

export type HoverSfxState = {
  phase: HoverSfxPhase;
};

export type HoverSfxEvent =
  | { type: 'enter'; edgeId: number; now: number }
  | { type: 'leave'; edgeId: number; now: number }
  | { type: 'press'; now: number }
  | { type: 'rebuild'; now: number }
  | { type: 'tick'; now: number };

export type HoverSfxEffect =
  | { type: 'schedule'; edgeId: number; delayMs: number }
  | { type: 'cancelSchedule' }
  | { type: 'play'; edgeId: number };

export function initialHoverSfxState(): HoverSfxState {
  return { phase: { kind: 'idle' } };
}

/**
 * Pure transition. Caller applies `effects` (timers / play) and stores `state`.
 */
export function reduceHoverSfx(
  state: HoverSfxState,
  event: HoverSfxEvent,
  debounceMs: number = HOVER_SFX_DEBOUNCE_MS,
  suppressMs: number = HOVER_SFX_SUPPRESS_AFTER_PRESS_MS,
): { state: HoverSfxState; effects: HoverSfxEffect[] } {
  const effects: HoverSfxEffect[] = [];
  let phase = state.phase;
  const now = event.now;

  if (phase.kind === 'suppressed' && now >= phase.until) {
    phase = { kind: 'idle' };
  }

  if (phase.kind === 'suppressed') {
    if (event.type === 'press') {
      return {
        state: { phase: { kind: 'suppressed', until: now + suppressMs } },
        effects: [{ type: 'cancelSchedule' }],
      };
    }
    if (event.type === 'rebuild') {
      return {
        state: { phase },
        effects: [{ type: 'cancelSchedule' }],
      };
    }
    return { state: { phase }, effects };
  }

  switch (event.type) {
    case 'enter': {
      if (phase.kind === 'pending' && phase.edgeId === event.edgeId) {
        return { state: { phase }, effects };
      }
      if (phase.kind === 'armed' && phase.edgeId === event.edgeId) {
        return { state: { phase }, effects };
      }
      effects.push({ type: 'cancelSchedule' });
      effects.push({
        type: 'schedule',
        edgeId: event.edgeId,
        delayMs: debounceMs,
      });
      return {
        state: {
          phase: {
            kind: 'pending',
            edgeId: event.edgeId,
            playAt: now + debounceMs,
          },
        },
        effects,
      };
    }
    case 'leave': {
      if (
        (phase.kind === 'pending' || phase.kind === 'armed') &&
        phase.edgeId === event.edgeId
      ) {
        effects.push({ type: 'cancelSchedule' });
        return { state: { phase: { kind: 'idle' } }, effects };
      }
      return { state: { phase }, effects };
    }
    case 'press': {
      effects.push({ type: 'cancelSchedule' });
      return {
        state: { phase: { kind: 'suppressed', until: now + suppressMs } },
        effects,
      };
    }
    case 'rebuild': {
      // Drop pending so a leave-during-rewire cannot orphan a timer that
      // then no-ops because edgeId was cleared. Idle also lets a genuine
      // re-fired pointerover arm a new hover after paint.
      if (phase.kind === 'pending' || phase.kind === 'armed') {
        effects.push({ type: 'cancelSchedule' });
        return { state: { phase: { kind: 'idle' } }, effects };
      }
      return { state: { phase }, effects };
    }
    case 'tick': {
      if (phase.kind !== 'pending') return { state: { phase }, effects };
      effects.push({ type: 'play', edgeId: phase.edgeId });
      return {
        state: { phase: { kind: 'armed', edgeId: phase.edgeId } },
        effects,
      };
    }
    default:
      return { state: { phase }, effects };
  }
}

/** Runtime adapter: owns timer handle; Pixi only dispatches events. */
export function createHoverSfxController(
  play: (edgeId: number) => void,
  opts?: { debounceMs?: number; suppressMs?: number; now?: () => number },
) {
  const debounceMs = opts?.debounceMs ?? HOVER_SFX_DEBOUNCE_MS;
  const suppressMs = opts?.suppressMs ?? HOVER_SFX_SUPPRESS_AFTER_PRESS_MS;
  const nowFn = opts?.now ?? (() => performance.now());

  let state = initialHoverSfxState();
  let timer: ReturnType<typeof setTimeout> | null = null;

  const clearTimer = () => {
    if (timer !== null) {
      clearTimeout(timer);
      timer = null;
    }
  };

  const apply = (event: HoverSfxEvent) => {
    const result = reduceHoverSfx(state, event, debounceMs, suppressMs);
    state = result.state;
    for (const effect of result.effects) {
      if (effect.type === 'cancelSchedule') {
        clearTimer();
      } else if (effect.type === 'schedule') {
        clearTimer();
        timer = setTimeout(() => {
          timer = null;
          apply({ type: 'tick', now: nowFn() });
        }, effect.delayMs);
      } else if (effect.type === 'play') {
        play(effect.edgeId);
      }
    }
  };

  return {
    enter(edgeId: number) {
      apply({ type: 'enter', edgeId, now: nowFn() });
    },
    leave(edgeId: number) {
      apply({ type: 'leave', edgeId, now: nowFn() });
    },
    press() {
      apply({ type: 'press', now: nowFn() });
    },
    rebuild() {
      apply({ type: 'rebuild', now: nowFn() });
    },
    dispose() {
      clearTimer();
      state = initialHoverSfxState();
    },
    getState: () => state,
  };
}
