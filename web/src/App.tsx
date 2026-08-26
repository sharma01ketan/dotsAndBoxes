import { useCallback, useEffect, useState } from 'react';
import { useGameSounds, useSoundMute } from './audio/useGameSounds';
import PixiBoard from './board/PixiBoard';
import {
  MAX_BOARD,
  MIN_BOARD,
  playerLabel,
  useGameStore,
  winnerLabel,
  type PlayOutcome,
} from './game/store';

function MuteToggle() {
  const { enabled, toggle } = useSoundMute();
  return (
    <button
      type="button"
      className="mute-toggle"
      onClick={toggle}
      aria-pressed={!enabled}
      aria-label={enabled ? 'Mute sound effects' : 'Unmute sound effects'}
    >
      {enabled ? 'Sound on' : 'Sound off'}
    </button>
  );
}

export default function App() {
  const status = useGameStore((s) => s.status);
  const error = useGameStore((s) => s.error);
  const message = useGameStore((s) => s.message);
  const boardSize = useGameStore((s) => s.boardSize);
  const snap = useGameStore((s) => s.snap);
  const game = useGameStore((s) => s.game);
  const init = useGameStore((s) => s.init);
  const newGame = useGameStore((s) => s.newGame);
  const setBoardSize = useGameStore((s) => s.setBoardSize);
  const play = useGameStore((s) => s.play);
  const { playHover, playMove, playNewGame } = useGameSounds();
  const [lastMove, setLastMove] = useState<PlayOutcome | null>(null);

  useEffect(() => {
    void init();
  }, [init]);

  const edgeCoord = useCallback(
    (edgeId: number): [number, number, number] | null => {
      if (!game) return null;
      try {
        const c = game.edgeCoord(edgeId);
        return [c[0]!, c[1]!, c[2]!];
      } catch {
        return null;
      }
    },
    [game],
  );

  const onEdgeClick = useCallback(
    (edgeId: number) => {
      const outcome = play(edgeId);
      if (outcome) {
        setLastMove(outcome);
        playMove(outcome);
      }
    },
    [play, playMove],
  );

  const onNewGame = useCallback(
    (size?: number) => {
      if (size !== undefined) setBoardSize(size);
      newGame(size);
      setLastMove(null);
      playNewGame();
    },
    [newGame, playNewGame, setBoardSize],
  );

  return (
    <main className="app">
      <MuteToggle />
      <p className="eyebrow">Dots and Boxes</p>
      <h1>Hotseat</h1>
      <p className="lede">Two players, one screen. Click an edge to draw.</p>

      {status === 'loading' && <p className="status">Loading WASM…</p>}
      {status === 'error' && <p className="status error">Failed: {error}</p>}

      {status === 'ready' && snap && (
        <>
          <section className="hud" aria-live="polite">
            <dl className="stats">
              <div>
                <dt>Turn</dt>
                <dd>
                  {snap.isTerminal
                    ? 'Game over'
                    : playerLabel(snap.currentPlayer)}
                </dd>
              </div>
              <div>
                <dt>Score</dt>
                <dd>
                  <span className="score p1">P1 {snap.scoreP1}</span>
                  <span className="score-sep">·</span>
                  <span className="score p2">P2 {snap.scoreP2}</span>
                </dd>
              </div>
            </dl>

            {snap.isTerminal && (
              <p className="banner">
                {snap.winner === 2
                  ? 'Draw!'
                  : `${winnerLabel(snap.winner)} wins`}
              </p>
            )}

            <p className="status ok">{message}</p>

            <div className="actions">
              <label className="size-label">
                Size
                <select
                  value={boardSize}
                  onChange={(e) => {
                    onNewGame(Number(e.target.value));
                  }}
                >
                  {Array.from({ length: MAX_BOARD - MIN_BOARD + 1 }, (_, i) => {
                    const n = MIN_BOARD + i;
                    return (
                      <option key={n} value={n}>
                        {n}×{n}
                      </option>
                    );
                  })}
                </select>
              </label>
              <button type="button" onClick={() => onNewGame()}>
                New game
              </button>
            </div>
          </section>

          <PixiBoard
            snap={snap}
            lastMove={lastMove}
            onEdgeClick={onEdgeClick}
            onEdgeHover={playHover}
            edgeCoord={edgeCoord}
          />

          <p className="legend">
            <span className="swatch p1" /> P1
            <span className="swatch p2" /> P2
          </p>
        </>
      )}
    </main>
  );
}
