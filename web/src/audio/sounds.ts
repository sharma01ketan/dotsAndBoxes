/**
 * Soft UI SFX: catalog + pure helpers (no React / Howler).
 * Samples: see public/sounds/ATTRIBUTION.md
 */

import type { PlayOutcome } from '../game/store';

export type { PlayOutcome };
export type SfxKind = 'hover' | 'draw' | 'claim' | 'win' | 'tie' | 'newGame';

type Spec = { src: string; volume: number; playbackRate?: number };

export const SOUNDS: Record<SfxKind, Spec> = {
  hover: { src: '/sounds/hover.mp3', volume: 0.18 },
  draw: { src: '/sounds/draw.mp3', volume: 0.32 },
  claim: { src: '/sounds/claim.mp3', volume: 0.38, playbackRate: 1.12 },
  win: { src: '/sounds/win.mp3', volume: 0.45 },
  tie: { src: '/sounds/tie.mp3', volume: 0.35, playbackRate: 0.92 },
  newGame: { src: '/sounds/new-game.mp3', volume: 0.28 },
};

export const HOVER_GAP_MS = 140;
export const TERMINAL_DELAY_MS = 160;

/** Ordered cues for a move (terminal last). */
export function sfxForPlay(outcome: PlayOutcome): SfxKind[] {
  const kinds: SfxKind[] = [outcome.completed > 0 ? 'claim' : 'draw'];
  if (!outcome.isTerminal) return kinds;
  if (outcome.winner === 2) kinds.push('tie');
  else if (outcome.winner === 0 || outcome.winner === 1) kinds.push('win');
  return kinds;
}

/** Leading-edge throttle gate. */
export function withinGap(lastAt: number, now: number, gapMs: number): boolean {
  return now - lastAt < gapMs;
}
