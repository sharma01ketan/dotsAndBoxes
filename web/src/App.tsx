import { useCallback, useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useGameSounds, useSoundMute } from './audio/useGameSounds';
import PixiBoard from './board/PixiBoard';
import {
  MAX_BOARD,
  MIN_BOARD,
  PLAY_MODES,
  modeLede,
  modeTitle,
  scoreLabelP1,
  scoreLabelP2,
  useGameStore,
  winnerLabel,
  type PlayMode,
  type PlayOutcome,
} from './game/store';

const AI_MOVE_PAUSE_MS = 220;

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
  const mode = useGameStore((s) => s.mode);
  const aiBusy = useGameStore((s) => s.aiBusy);
  const snap = useGameStore((s) => s.snap);
  const gameGeneration = useGameStore((s) => s.gameGeneration);
  const game = useGameStore((s) => s.game);
  const init = useGameStore((s) => s.init);
  const newGame = useGameStore((s) => s.newGame);
  const setBoardSize = useGameStore((s) => s.setBoardSize);
  const setMode = useGameStore((s) => s.setMode);
  const play = useGameStore((s) => s.play);
  const runAiTurn = useGameStore((s) => s.runAiTurn);
  const { playHover, playMove, playNewGame } = useGameSounds();
  const [lastMove, setLastMove] = useState<PlayOutcome | null>(null);
  const [pendingMode, setPendingMode] = useState<PlayMode | null>(null);

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

  const kickAi = useCallback(() => {
    void runAiTurn(AI_MOVE_PAUSE_MS, (outcome) => {
      setLastMove(outcome);
      playMove(outcome);
    });
  }, [runAiTurn, playMove]);

  const onEdgeClick = useCallback(
    (edgeId: number) => {
      const outcome = play(edgeId);
      if (!outcome) return;
      setLastMove(outcome);
      playMove(outcome);
      if (
        !outcome.isTerminal &&
        mode !== 'hotseat' &&
        useGameStore.getState().snap?.currentPlayer === 1
      ) {
        // Let claim/draw animation start before CPU moves.
        window.setTimeout(kickAi, 0);
      }
    },
    [play, playMove, mode, kickAi],
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

  const confirmModeChange = useCallback(() => {
    if (!pendingMode) return;
    setMode(pendingMode);
    setLastMove(null);
    setPendingMode(null);
    playNewGame();
  }, [pendingMode, setMode, playNewGame]);

  const cancelModeChange = useCallback(() => {
    setPendingMode(null);
  }, []);

  const pendingModeLabel =
    PLAY_MODES.find((m) => m.id === pendingMode)?.label ?? pendingMode;

  const humanCanPlay =
    !!snap &&
    !snap.isTerminal &&
    !aiBusy &&
    (mode === 'hotseat' || snap.currentPlayer === 0);

  const turnLabel = (() => {
    if (!snap) return '';
    if (snap.isTerminal) return 'Game over';
    if (aiBusy) return 'CPU thinking…';
    if (mode === 'hotseat') {
      return snap.currentPlayer === 0 ? 'P1' : 'P2';
    }
    return snap.currentPlayer === 0 ? 'Your turn' : 'CPU thinking…';
  })();

  return (
    <main className="app">
      <MuteToggle />
      <p className="eyebrow">Dots and Boxes</p>
      <h1>{modeTitle(mode)}</h1>
      <p className="lede">{modeLede(mode)}</p>

      {status === 'loading' && <p className="status">Loading WASM…</p>}
      {status === 'error' && <p className="status error">Failed: {error}</p>}

      {status === 'ready' && snap && (
        <>
          <section className="hud" aria-live="polite">
            <dl className="stats">
              <div>
                <dt>Turn</dt>
                <dd>{turnLabel}</dd>
              </div>
              <div>
                <dt>Score</dt>
                <dd>
                  <span className="score p1">
                    {scoreLabelP1(mode)} {snap.scoreP1}
                  </span>
                  <span className="score-sep">·</span>
                  <span className="score p2">
                    {scoreLabelP2(mode)} {snap.scoreP2}
                  </span>
                </dd>
              </div>
            </dl>

            {snap.isTerminal && (
              <p className="banner">
                {snap.winner === 2
                  ? 'Draw!'
                  : mode === 'hotseat'
                    ? `${winnerLabel(snap.winner)} wins`
                    : snap.winner === 0
                      ? 'You win!'
                      : snap.winner === 1
                        ? 'CPU wins'
                        : `${winnerLabel(snap.winner)} wins`}
              </p>
            )}

            <p className="status ok">{message}</p>

            <div className="actions">
              <label className="size-label" htmlFor="play-mode">
                Mode
                <select
                  id="play-mode"
                  aria-label="Play mode"
                  value={pendingMode ?? mode}
                  onChange={(e) => {
                    const next = e.target.value as PlayMode;
                    if (next === mode) {
                      setPendingMode(null);
                      return;
                    }
                    setPendingMode(next);
                  }}
                >
                  {PLAY_MODES.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.label}
                    </option>
                  ))}
                </select>
              </label>
              <label className="size-label" htmlFor="board-size">
                Size
                <select
                  id="board-size"
                  aria-label="Board size"
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
            gameGeneration={gameGeneration}
            inputEnabled={humanCanPlay}
            onEdgeClick={onEdgeClick}
            onEdgeHover={playHover}
            edgeCoord={edgeCoord}
          />

          <p className="legend">
            <span className="swatch p1" /> {scoreLabelP1(mode)}
            <span className="swatch p2" /> {scoreLabelP2(mode)}
          </p>
        </>
      )}

      {pendingMode &&
        createPortal(
          <div
            className="modal-backdrop"
            role="presentation"
            onClick={cancelModeChange}
          >
            <div
              className="modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="mode-switch-title"
              onClick={(e) => e.stopPropagation()}
            >
              <h2 id="mode-switch-title">Switch mode?</h2>
              <p>
                Change to <strong>{pendingModeLabel}</strong> and start a new
                game? The current board will be reset.
              </p>
              <div className="modal-actions">
                <button type="button" onClick={cancelModeChange}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="modal-confirm"
                  onClick={confirmModeChange}
                >
                  Switch &amp; reset
                </button>
              </div>
            </div>
          </div>,
          document.body,
        )}
    </main>
  );
}
