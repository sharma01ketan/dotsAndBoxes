import { useCallback, useEffect, useState } from 'react';
import { createPortal } from 'react-dom';
import { useGameSounds, useSoundMute } from './audio/useGameSounds';
import PixiBoard from './board/PixiBoard';
import { getAiEngine } from './game/aiClient';
import {
  MAX_BOARD,
  MIN_BOARD,
  PLAY_MODES,
  analysisLine,
  hintPolicyForMode,
  isPerfectHudSize,
  modeLede,
  modeTitle,
  perfectSaysLine,
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
  const analysis = useGameStore((s) => s.analysis);
  const perfectMargin = useGameStore((s) => s.perfectMargin);
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
  const [overlayOn, setOverlayOn] = useState(false);
  const [hintEdgeId, setHintEdgeId] = useState<number | null>(null);
  const [pendingMode, setPendingMode] = useState<PlayMode | null>(null);
  const [pendingSize, setPendingSize] = useState<number | null>(null);

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

  useEffect(() => {
    if (mode === 'hotseat') return;
    if (!snap || snap.isTerminal || snap.currentPlayer !== 1) return;
    kickAi();
  }, [mode, snap, gameGeneration, kickAi]);

  useEffect(() => {
    setHintEdgeId(null);
  }, [gameGeneration, lastMove]);

  const onEdgeClick = useCallback(
    (edgeId: number) => {
      const outcome = play(edgeId);
      if (!outcome) return;
      setLastMove(outcome);
      playMove(outcome);
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

  const confirmSizeChange = useCallback(() => {
    if (pendingSize === null) return;
    setBoardSize(pendingSize);
    setMode('vs-cgt');
    setLastMove(null);
    setPendingSize(null);
    playNewGame();
  }, [pendingSize, setBoardSize, setMode, playNewGame]);

  const cancelSizeChange = useCallback(() => {
    setPendingSize(null);
  }, []);

  const pendingModeLabel =
    PLAY_MODES.find((m) => m.id === pendingMode)?.label ?? pendingMode;

  const humanCanPlay =
    !!snap &&
    !snap.isTerminal &&
    !aiBusy &&
    (mode === 'hotseat' || snap.currentPlayer === 0);

  const hintPolicy = status === 'ready' ? hintPolicyForMode(mode) : null;
  const hintDisabled =
    status !== 'ready' ||
    !snap ||
    snap.isTerminal ||
    aiBusy ||
    hintPolicy === null ||
    (mode === 'vs-perfect' && !isPerfectHudSize(boardSize));

  const onHint = useCallback(async () => {
    const policy = hintPolicyForMode(mode);
    const { snap: live, gameGeneration: gen, gameSeed, moveCount } =
      useGameStore.getState();
    if (
      policy === null ||
      !live ||
      live.isTerminal ||
      (mode === 'vs-perfect' && !isPerfectHudSize(boardSize))
    ) {
      return;
    }
    const seed = BigInt(gameSeed) + BigInt(moveCount) * 0x100000001b3n;
    try {
      const edge = await getAiEngine().chooseMove(policy, seed);
      const st = useGameStore.getState();
      if (st.gameGeneration !== gen || st.moveCount !== moveCount) return;
      if (!st.snap?.legalMoves.includes(edge)) return;
      setHintEdgeId(edge);
    } catch (err) {
      useGameStore.setState({
        message: err instanceof Error ? err.message : String(err),
      });
    }
  }, [mode, boardSize]);

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
            {mode === 'vs-perfect' &&
              snap &&
              !snap.isTerminal &&
              perfectMargin != null && (
                <p className="status ok">
                  {perfectMargin === 'computing'
                    ? 'Perfect is solving…'
                    : perfectSaysLine(perfectMargin, snap.currentPlayer)}
                </p>
              )}
            {overlayOn && analysis && (
              <p className="theory-line">{analysisLine(analysis)}</p>
            )}

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
                    <option
                      key={m.id}
                      value={m.id}
                      disabled={m.id === 'vs-perfect' && !isPerfectHudSize(boardSize)}
                    >
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
                  value={pendingSize ?? boardSize}
                  onChange={(e) => {
                    const n = Number(e.target.value);
                    if (mode === 'vs-perfect' && !isPerfectHudSize(n)) {
                      setPendingSize(n);
                      return;
                    }
                    onNewGame(n);
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
              <button
                type="button"
                aria-pressed={overlayOn}
                onClick={() => setOverlayOn((on) => !on)}
              >
                Overlay
              </button>
              <button
                type="button"
                disabled={hintDisabled}
                onClick={() => {
                  void onHint();
                }}
              >
                Hint
              </button>
            </div>
          </section>

          <PixiBoard
            snap={snap}
            lastMove={lastMove}
            gameGeneration={gameGeneration}
            inputEnabled={humanCanPlay}
            analysis={overlayOn ? analysis : null}
            hintEdgeId={hintEdgeId}
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
      {pendingSize !== null &&
        createPortal(
          <div
            className="modal-backdrop"
            role="presentation"
            onClick={cancelSizeChange}
          >
            <div
              className="modal"
              role="dialog"
              aria-modal="true"
              aria-labelledby="size-switch-title"
              onClick={(e) => e.stopPropagation()}
            >
              <h2 id="size-switch-title">Switch to Hard (CGT)?</h2>
              <p>
                Perfect only plays 2×2 and 3×3. Change to{' '}
                <strong>Hard (CGT)</strong> at{' '}
                <strong>
                  {pendingSize}×{pendingSize}
                </strong>{' '}
                and start a new game? The current board will be reset.
              </p>
              <div className="modal-actions">
                <button type="button" onClick={cancelSizeChange}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="modal-confirm"
                  onClick={confirmSizeChange}
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
