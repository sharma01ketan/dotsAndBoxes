import { useEffect, useState } from 'react';
import {
  initWasm,
  playerLabel,
  WasmGame,
  winnerLabel,
} from './lib/wasmGame';

type Snapshot = {
  rows: number;
  cols: number;
  currentPlayer: number;
  scoreP1: number;
  scoreP2: number;
  legalMoves: number[];
  isTerminal: boolean;
  winner: number;
  lastMessage: string;
};

function snapshotFrom(game: WasmGame, lastMessage: string): Snapshot {
  return {
    rows: game.rows,
    cols: game.cols,
    currentPlayer: game.currentPlayer(),
    scoreP1: game.scoreP1(),
    scoreP2: game.scoreP2(),
    legalMoves: Array.from(game.legalMoves()),
    isTerminal: game.isTerminal(),
    winner: game.winner(),
    lastMessage,
  };
}

export default function App() {
  const [status, setStatus] = useState<'loading' | 'ready' | 'error'>('loading');
  const [error, setError] = useState<string | null>(null);
  const [game, setGame] = useState<WasmGame | null>(null);
  const [snap, setSnap] = useState<Snapshot | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        await initWasm();
        if (cancelled) return;
        const g = new WasmGame(2, 2);
        setGame(g);
        setSnap(snapshotFrom(g, 'WASM ready — 2×2 game created'));
        setStatus('ready');
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
        setStatus('error');
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  function playEdge(edge: number) {
    if (!game) return;
    try {
      const result = game.play(edge);
      const extraTurn = result[0] === 1;
      const completed = result[1] ?? 0;
      const msg = extraTurn
        ? `Played #${edge} — claimed ${completed} box(es), extra turn`
        : `Played #${edge}`;
      setSnap(snapshotFrom(game, msg));
    } catch (err) {
      setSnap((prev) =>
        prev
          ? {
              ...prev,
              lastMessage: err instanceof Error ? err.message : String(err),
            }
          : prev,
      );
    }
  }

  function newGame() {
    const g = new WasmGame(2, 2);
    setGame(g);
    setSnap(snapshotFrom(g, 'New 2×2 game'));
  }

  return (
    <main className="app">
      <p className="eyebrow">Dots and Boxes</p>
      <h1>WASM core smoke test</h1>
      <p className="lede">
        Minimal UI that loads <code>@dab/dab-wasm</code>, creates a game, lists legal
        moves, and applies plays. PixiJS board comes next.
      </p>

      {status === 'loading' && <p className="status">Loading WASM…</p>}
      {status === 'error' && <p className="status error">Failed: {error}</p>}

      {status === 'ready' && snap && (
        <section className="panel">
          <p className="status ok">{snap.lastMessage}</p>
          <dl className="stats">
            <div>
              <dt>Board</dt>
              <dd>
                {snap.rows}×{snap.cols}
              </dd>
            </div>
            <div>
              <dt>Turn</dt>
              <dd>{playerLabel(snap.currentPlayer)}</dd>
            </div>
            <div>
              <dt>Score</dt>
              <dd>
                P1 {snap.scoreP1} — P2 {snap.scoreP2}
              </dd>
            </div>
            <div>
              <dt>Winner</dt>
              <dd>{winnerLabel(snap.winner)}</dd>
            </div>
          </dl>

          <div className="actions">
            <button type="button" onClick={newGame}>
              New game
            </button>
          </div>

          <h2>Legal moves</h2>
          {snap.isTerminal ? (
            <p className="status">Game over — {winnerLabel(snap.winner)}</p>
          ) : (
            <ul className="moves">
              {snap.legalMoves.map((edge) => (
                <li key={edge}>
                  <button type="button" onClick={() => playEdge(edge)}>
                    #{edge}
                  </button>
                </li>
              ))}
            </ul>
          )}
        </section>
      )}
    </main>
  );
}
