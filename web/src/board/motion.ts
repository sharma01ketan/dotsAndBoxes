/** Tiny rAF tween helper for board polish (KET-14). */

export function easeOutCubic(t: number): number {
  return 1 - (1 - t) ** 3;
}

export type TweenHandle = { cancel: () => void };

export function prefersReducedMotion(): boolean {
  const g = globalThis as typeof globalThis & {
    matchMedia?: (query: string) => { matches: boolean };
  };
  if (typeof g.matchMedia !== 'function') return false;
  return g.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

/**
 * Animate from 0→1 over `durationMs`. Returns a cancel handle.
 * If reduced motion is preferred, jumps to 1 immediately.
 */
export function animate(
  durationMs: number,
  onUpdate: (t: number) => void,
  onDone?: () => void,
): TweenHandle {
  if (prefersReducedMotion() || durationMs <= 0) {
    onUpdate(1);
    onDone?.();
    return { cancel() {} };
  }

  let raf = 0;
  let cancelled = false;
  const start = performance.now();

  const frame = (now: number) => {
    if (cancelled) return;
    const raw = Math.min(1, (now - start) / durationMs);
    onUpdate(easeOutCubic(raw));
    if (raw < 1) {
      raf = requestAnimationFrame(frame);
    } else {
      onDone?.();
    }
  };

  raf = requestAnimationFrame(frame);
  return {
    cancel() {
      cancelled = true;
      cancelAnimationFrame(raf);
    },
  };
}

export function delayMs(ms: number, fn: () => void): TweenHandle {
  if (prefersReducedMotion() || ms <= 0) {
    fn();
    return { cancel() {} };
  }
  const id = setTimeout(fn, ms);
  return {
    cancel() {
      clearTimeout(id);
    },
  };
}
