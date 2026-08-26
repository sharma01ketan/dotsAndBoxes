import { useEffect, useRef, useSyncExternalStore } from 'react';
import useSound from 'use-sound';
import type { PlayOutcome } from '../game/store';
import {
  HOVER_GAP_MS,
  SOUNDS,
  TERMINAL_DELAY_MS,
  sfxForPlay,
  withinGap,
  type SfxKind,
} from './sounds';

const STORAGE_KEY = 'dab.soundEnabled';

function readEnabled(): boolean {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw === null) return true;
    return raw === '1' || raw === 'true';
  } catch {
    return true;
  }
}

let soundOn = typeof window !== 'undefined' ? readEnabled() : true;
const listeners = new Set<() => void>();

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

function getSoundOn() {
  return soundOn;
}

function setSoundOn(on: boolean) {
  soundOn = on;
  try {
    localStorage.setItem(STORAGE_KEY, on ? '1' : '0');
  } catch {
    /* private mode */
  }
  for (const listener of listeners) listener();
}

/** Shared mute flag (localStorage). Used by the HUD toggle and SFX hook. */
export function useSoundMute() {
  const enabled = useSyncExternalStore(subscribe, getSoundOn, () => true);
  return {
    enabled,
    toggle: () => setSoundOn(!getSoundOn()),
  };
}

function useSfxPlayer(kind: SfxKind, enabled: boolean) {
  const spec = SOUNDS[kind];
  const [play] = useSound(spec.src, {
    volume: spec.volume,
    playbackRate: spec.playbackRate,
    soundEnabled: enabled,
  });
  return play;
}

/**
 * Game SFX via use-sound. Hover is throttled; win/tie are slightly delayed.
 */
export function useGameSounds() {
  const { enabled } = useSoundMute();
  const enabledRef = useRef(enabled);
  enabledRef.current = enabled;

  const lastHoverAt = useRef(0);
  const cancelTerminal = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(
    () => () => {
      if (cancelTerminal.current !== null) clearTimeout(cancelTerminal.current);
    },
    [],
  );

  const playHoverRaw = useSfxPlayer('hover', enabled);
  const playDraw = useSfxPlayer('draw', enabled);
  const playClaim = useSfxPlayer('claim', enabled);
  const playWin = useSfxPlayer('win', enabled);
  const playTie = useSfxPlayer('tie', enabled);
  const playNewGameRaw = useSfxPlayer('newGame', enabled);

  const play = (kind: SfxKind) => {
    if (!enabledRef.current) return;
    switch (kind) {
      case 'hover':
        playHoverRaw();
        break;
      case 'draw':
        playDraw();
        break;
      case 'claim':
        playClaim();
        break;
      case 'win':
        playWin();
        break;
      case 'tie':
        playTie();
        break;
      case 'newGame':
        playNewGameRaw();
        break;
    }
  };

  return {
    playHover() {
      if (!enabledRef.current) return;
      const now = performance.now();
      if (withinGap(lastHoverAt.current, now, HOVER_GAP_MS)) return;
      lastHoverAt.current = now;
      play('hover');
    },
    playNewGame() {
      if (cancelTerminal.current !== null) clearTimeout(cancelTerminal.current);
      play('newGame');
    },
    playMove(outcome: PlayOutcome) {
      if (cancelTerminal.current !== null) clearTimeout(cancelTerminal.current);
      for (const kind of sfxForPlay(outcome)) {
        if (kind === 'win' || kind === 'tie') {
          cancelTerminal.current = setTimeout(
            () => play(kind),
            TERMINAL_DELAY_MS,
          );
        } else {
          play(kind);
        }
      }
    },
  };
}
