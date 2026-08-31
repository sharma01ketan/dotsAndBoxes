import { useEffect, useRef } from 'react';
import { Application, Container, Graphics, Text } from 'pixi.js';
import type { GameSnapshot, PlayOutcome } from '../game/store';
import type { AnalysisSnapshot } from '../game/analysis';
import {
  COLORS,
  boxCenter,
  computeLayout,
  hitPad,
  horizontalEdgeEnds,
  verticalEdgeEnds,
  type BoardLayout,
  type Point,
} from './layout';
import { animate, delayMs, prefersReducedMotion, type TweenHandle } from './motion';
import { createHoverSfxController } from './edgeHoverSfx';

type Props = {
  snap: GameSnapshot;
  lastMove: PlayOutcome | null;
  /** Bumped on newGame — forces a full board rebuild even when size is unchanged. */
  gameGeneration: number;
  /** When false, undrawn edges are not clickable (AI turn / busy). */
  inputEnabled?: boolean;
  analysis?: AnalysisSnapshot | null;
  hintEdgeId?: number | null;
  onEdgeClick: (edgeId: number) => void;
  onEdgeHover?: (edgeId: number) => void;
  edgeCoord: (edgeId: number) => [number, number, number] | null;
};

type EdgeGfx = {
  id: number;
  group: Container;
  line: Graphics;
  hit: Graphics;
  a: Point;
  b: Point;
  drawn: boolean;
  owner: number;
};

type BoxGfx = {
  id: number;
  group: Container;
  fill: Graphics;
  label: Text;
  owner: number;
  cx: number;
  cy: number;
  size: number;
};

type Layers = {
  overlay: Container;
  boxes: Container;
  edges: Container;
  dots: Container;
};

type BoardRuntime = {
  layout: BoardLayout | null;
  rows: number;
  cols: number;
  gameGeneration: number;
  edges: Map<number, EdgeGfx>;
  boxes: Map<number, BoxGfx>;
  tweens: TweenHandle[];
  lastMoveKey: string | null;
  /** Last `currentPlayer` the hover preview was wired for. */
  wiredPlayer: number;
};

const EDGE_DRAW_MS = 180;
const HOVER_EDGE_ALPHA = 0.4;
const UNDRAWN_ALPHA = 0.55;
const BOX_CLAIM_MS = 250;
const CLAIM_STAGGER_MS = 70;
const WIN_PULSE_MS = 200;

function strokeColor(owner: number, drawn: boolean): number {
  if (!drawn) return COLORS.undrawn;
  return owner === 0 ? COLORS.ok : COLORS.accent;
}

function hoverColor(currentPlayer: number): number {
  return currentPlayer === 0 ? COLORS.ok : COLORS.accent;
}

function drawEdgeLine(
  g: Graphics,
  a: Point,
  b: Point,
  color: number,
  width: number,
  alpha: number,
) {
  g.clear();
  g.moveTo(a.x, a.y);
  g.lineTo(b.x, b.y);
  g.stroke({ width, color, alpha, cap: 'round' });
}

function drawHitPad(g: Graphics, a: Point, b: Point, pad: number) {
  g.clear();
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  const nx = -dy / len;
  const ny = dx / len;
  const hx = nx * (pad / 2);
  const hy = ny * (pad / 2);
  g.poly([
    a.x + hx,
    a.y + hy,
    b.x + hx,
    b.y + hy,
    b.x - hx,
    b.y - hy,
    a.x - hx,
    a.y - hy,
  ]);
  g.fill({ color: 0xffffff, alpha: 0.001 });
}

function edgeEnds(
  layout: BoardLayout,
  orient: number,
  row: number,
  col: number,
): [Point, Point] {
  return orient === 0
    ? horizontalEdgeEnds(layout, row, col)
    : verticalEdgeEnds(layout, row, col);
}

function cancelTweens(rt: BoardRuntime) {
  for (const tw of rt.tweens) tw.cancel();
  rt.tweens = [];
}

function trackTween(rt: BoardRuntime, tw: TweenHandle) {
  rt.tweens.push(tw);
}

function clearLayer(container: Container) {
  container.removeChildren().forEach((c) => c.destroy({ children: true }));
}

function overlayColor(kind: 0 | 1 | 2): number {
  if (kind === 1) return COLORS.overlayLong;
  if (kind === 2) return COLORS.overlayLoop;
  return COLORS.overlayShort;
}

function paintOverlay(
  overlay: Container,
  snap: GameSnapshot,
  layout: BoardLayout,
  analysis: AnalysisSnapshot | null,
) {
  clearLayer(overlay);
  if (!analysis) return;
  const size = layout.cell * 0.82;
  for (const region of analysis.regions) {
    const color = overlayColor(region.kind);
    for (const id of region.boxes) {
      if ((snap.boxOwner[id] ?? -1) >= 0) continue;
      const r = Math.floor(id / snap.cols);
      const c = id % snap.cols;
      const center = boxCenter(layout, r, c);
      const g = new Graphics();
      g.roundRect(-size / 2, -size / 2, size, size, 6);
      g.fill({ color, alpha: 0.18 });
      g.position.set(center.x, center.y);
      overlay.addChild(g);
    }
  }
}

function paintHintOnOverlay(
  overlay: Container,
  rt: BoardRuntime,
  snap: GameSnapshot,
  layout: BoardLayout,
  hintEdgeId: number | null,
) {
  if (hintEdgeId === null) return;
  if ((snap.edgeOwner[hintEdgeId] ?? -1) >= 0) return;
  const edge = rt.edges.get(hintEdgeId);
  if (!edge) return;
  const lineW = Math.max(4, layout.cell * 0.08);
  const g = new Graphics();
  drawEdgeLine(
    g,
    edge.a,
    edge.b,
    hoverColor(snap.currentPlayer),
    lineW,
    0.95,
  );
  overlay.addChild(g);
}

function paintDots(dots: Container, snap: GameSnapshot, layout: BoardLayout) {
  clearLayer(dots);
  const dotR = Math.max(4, layout.cell * 0.08);
  for (let r = 0; r <= snap.rows; r++) {
    for (let c = 0; c <= snap.cols; c++) {
      const g = new Graphics();
      g.circle(
        layout.originX + c * layout.cell,
        layout.originY + r * layout.cell,
        dotR,
      );
      g.fill({ color: COLORS.ink });
      dots.addChild(g);
    }
  }
}

function paintBoxArt(
  fill: Graphics,
  label: Text,
  cx: number,
  cy: number,
  size: number,
  owner: number,
  alpha: number,
  scale: number,
) {
  fill.clear();
  fill.roundRect((-size / 2) * scale, (-size / 2) * scale, size * scale, size * scale, 6);
  fill.fill({
    color: owner === 0 ? COLORS.boxP1 : COLORS.boxP2,
    alpha: 0.22 * alpha,
  });
  fill.position.set(cx, cy);
  label.alpha = alpha;
  label.scale.set(scale);
  label.position.set(cx, cy);
}

function createBoxGfx(
  boxId: number,
  owner: number,
  layout: BoardLayout,
  cols: number,
): BoxGfx {
  const r = Math.floor(boxId / cols);
  const c = boxId % cols;
  const center = boxCenter(layout, r, c);
  const size = layout.cell * 0.82;
  const group = new Container();
  const fill = new Graphics();
  const label = new Text({
    text: owner === 0 ? '1' : '2',
    style: {
      fontFamily: 'Georgia, serif',
      fontSize: Math.max(14, layout.cell * 0.28),
      fill: owner === 0 ? COLORS.ok : COLORS.accent,
      fontWeight: '600',
    },
  });
  label.anchor.set(0.5);
  paintBoxArt(fill, label, center.x, center.y, size, owner, 1, 1);
  group.addChild(fill);
  group.addChild(label);
  return {
    id: boxId,
    group,
    fill,
    label,
    owner,
    cx: center.x,
    cy: center.y,
    size,
  };
}

function hitLive(
  edge: EdgeGfx,
  snap: GameSnapshot,
  inputEnabled: boolean,
): boolean {
  const drawn = (snap.edgeOwner[edge.id] ?? -1) >= 0;
  return inputEnabled && !drawn && !snap.isTerminal;
}

function shouldRewireHit(
  edge: EdgeGfx,
  snap: GameSnapshot,
  inputEnabled: boolean,
  wiredPlayer: number,
): boolean {
  const live = hitLive(edge, snap, inputEnabled);
  const wantMode = live ? 'static' : 'none';
  const wantCursor = live ? 'pointer' : 'default';
  if (edge.hit.eventMode !== wantMode || edge.hit.cursor !== wantCursor) {
    return true;
  }
  // Hover preview color is closed over currentPlayer at wire time.
  return live && wiredPlayer !== snap.currentPlayer;
}

function wireEdgeHit(
  edge: EdgeGfx,
  snap: GameSnapshot,
  lineW: number,
  undrawnW: number,
  edgesMap: Map<number, EdgeGfx>,
  onEdgeClick: (id: number) => void,
  hoverSfx: ReturnType<typeof createHoverSfxController>,
  inputEnabled: boolean,
) {
  const { hit, a, b, id: edgeId } = edge;
  hit.removeAllListeners();
  const live = hitLive(edge, snap, inputEnabled);
  hit.eventMode = live ? 'static' : 'none';
  hit.cursor = live ? 'pointer' : 'default';
  const preview = hoverColor(snap.currentPlayer);

  hit.on('pointerover', () => {
    const cur = edgesMap.get(edgeId);
    if (!cur || cur.drawn) return;
    drawEdgeLine(cur.line, a, b, preview, lineW, HOVER_EDGE_ALPHA);
    hoverSfx.enter(edgeId);
  });
  hit.on('pointerout', () => {
    const cur = edgesMap.get(edgeId);
    if (!cur || cur.drawn) return;
    drawEdgeLine(cur.line, a, b, COLORS.undrawn, undrawnW, UNDRAWN_ALPHA);
    hoverSfx.leave(edgeId);
  });
  hit.on('pointerdown', () => {
    hoverSfx.press();
  });
  hit.on('pointertap', () => {
    hoverSfx.press();
    onEdgeClick(edgeId);
  });
}

function fullRebuild(
  layers: Layers,
  rt: BoardRuntime,
  snap: GameSnapshot,
  layout: BoardLayout,
  edgeCoord: (edgeId: number) => [number, number, number] | null,
  onEdgeClick: (edgeId: number) => void,
  hoverSfx: ReturnType<typeof createHoverSfxController>,
  inputEnabled: boolean,
) {
  hoverSfx.rebuild();
  rt.wiredPlayer = snap.currentPlayer;
  cancelTweens(rt);
  clearLayer(layers.overlay);
  clearLayer(layers.boxes);
  clearLayer(layers.edges);
  clearLayer(layers.dots);
  rt.edges.clear();
  rt.boxes.clear();
  rt.layout = layout;
  rt.rows = snap.rows;
  rt.cols = snap.cols;

  const pad = hitPad(layout);
  const lineW = Math.max(3, layout.cell * 0.06);
  const undrawnW = Math.max(2, layout.cell * 0.04);

  for (let id = 0; id < snap.edgeCount; id++) {
    const coord = edgeCoord(id);
    if (!coord) continue;
    const [orient, row, col] = coord;
    const [a, b] = edgeEnds(layout, orient, row, col);
    const owner = snap.edgeOwner[id] ?? -1;
    const drawn = owner >= 0;
    const line = new Graphics();
    drawEdgeLine(
      line,
      a,
      b,
      strokeColor(owner, drawn),
      drawn ? lineW : undrawnW,
      drawn ? 1 : UNDRAWN_ALPHA,
    );
    const hit = new Graphics();
    drawHitPad(hit, a, b, pad);
    const group = new Container();
    group.addChild(line);
    group.addChild(hit);
    layers.edges.addChild(group);
    const edge: EdgeGfx = { id, group, line, hit, a, b, drawn, owner };
    rt.edges.set(id, edge);
    wireEdgeHit(
      edge,
      snap,
      lineW,
      undrawnW,
      rt.edges,
      onEdgeClick,
      hoverSfx,
      inputEnabled,
    );
  }

  for (let id = 0; id < snap.boxCount; id++) {
    const owner = snap.boxOwner[id] ?? -1;
    if (owner < 0) continue;
    const box = createBoxGfx(id, owner, layout, snap.cols);
    layers.boxes.addChild(box.group);
    rt.boxes.set(id, box);
  }

  paintDots(layers.dots, snap, layout);
}

function animateEdgeDraw(
  rt: BoardRuntime,
  edge: EdgeGfx,
  mover: number,
  lineW: number,
) {
  const color = mover === 0 ? COLORS.ok : COLORS.accent;
  edge.drawn = true;
  edge.owner = mover;
  edge.hit.eventMode = 'none';
  edge.hit.cursor = 'default';

  // Full stroke already; commit by fading opacity up (no grow-along-segment).
  const fromAlpha = HOVER_EDGE_ALPHA;
  drawEdgeLine(edge.line, edge.a, edge.b, color, lineW, fromAlpha);
  const tw = animate(
    EDGE_DRAW_MS,
    (t) => {
      const alpha = fromAlpha + (1 - fromAlpha) * t;
      drawEdgeLine(edge.line, edge.a, edge.b, color, lineW, alpha);
    },
    () => {
      drawEdgeLine(edge.line, edge.a, edge.b, color, lineW, 1);
    },
  );
  trackTween(rt, tw);
}

function instantFillBox(
  rt: BoardRuntime,
  layers: Layers,
  boxId: number,
  owner: number,
  layout: BoardLayout,
  cols: number,
) {
  if (rt.boxes.has(boxId)) return;
  const box = createBoxGfx(boxId, owner, layout, cols);
  layers.boxes.addChild(box.group);
  rt.boxes.set(boxId, box);
}

function animateBoxClaim(
  rt: BoardRuntime,
  layers: Layers,
  boxId: number,
  owner: number,
  layout: BoardLayout,
  cols: number,
  delay: number,
) {
  const run = () => {
    if (rt.boxes.has(boxId)) return;
    const box = createBoxGfx(boxId, owner, layout, cols);
    layers.boxes.addChild(box.group);
    rt.boxes.set(boxId, box);
    paintBoxArt(box.fill, box.label, box.cx, box.cy, box.size, owner, 0, 0.92);
    const tw = animate(BOX_CLAIM_MS, (t) => {
      const alpha = t;
      const scale = 0.92 + 0.08 * t;
      paintBoxArt(box.fill, box.label, box.cx, box.cy, box.size, owner, alpha, scale);
    });
    trackTween(rt, tw);
  };

  if (delay <= 0 || prefersReducedMotion()) {
    run();
    return;
  }
  trackTween(rt, delayMs(delay, run));
}

function pulseWinnerBoxes(rt: BoardRuntime, winner: number) {
  const targets = [...rt.boxes.values()].filter((b) => b.owner === winner);
  for (const box of targets) {
    const tw = animate(WIN_PULSE_MS, (t) => {
      // 1 → 1.06 → 1
      const pulse = t < 0.5 ? 1 + 0.06 * (t * 2) : 1.06 - 0.06 * ((t - 0.5) * 2);
      paintBoxArt(
        box.fill,
        box.label,
        box.cx,
        box.cy,
        box.size,
        box.owner,
        1,
        pulse,
      );
    }, () => {
      paintBoxArt(box.fill, box.label, box.cx, box.cy, box.size, box.owner, 1, 1);
    });
    trackTween(rt, tw);
  }
}

function syncBoard(
  layers: Layers,
  rt: BoardRuntime,
  snap: GameSnapshot,
  layout: BoardLayout,
  lastMove: PlayOutcome | null,
  gameGeneration: number,
  edgeCoord: (edgeId: number) => [number, number, number] | null,
  onEdgeClick: (edgeId: number) => void,
  hoverSfx: ReturnType<typeof createHoverSfxController>,
  inputEnabled: boolean,
  analysis: AnalysisSnapshot | null,
  hintEdgeId: number | null,
) {
  const sizeChanged =
    !rt.layout ||
    rt.rows !== snap.rows ||
    rt.cols !== snap.cols ||
    rt.layout.cell !== layout.cell ||
    rt.layout.originX !== layout.originX ||
    rt.layout.originY !== layout.originY;
  // Same-size newGame must rebuild — incremental sync never clears claimed boxes.
  const generationChanged = rt.gameGeneration !== gameGeneration;

  if (sizeChanged || generationChanged) {
    fullRebuild(
      layers,
      rt,
      snap,
      layout,
      edgeCoord,
      onEdgeClick,
      hoverSfx,
      inputEnabled,
    );
    rt.gameGeneration = gameGeneration;
    rt.lastMoveKey = null;
    // Still apply lastMove animation if present after rebuild.
  } else {
    const lineW = Math.max(3, layout.cell * 0.06);
    const undrawnW = Math.max(2, layout.cell * 0.04);
    let rewired = false;
    for (const edge of rt.edges.values()) {
      if (shouldRewireHit(edge, snap, inputEnabled, rt.wiredPlayer)) {
        if (!rewired) {
          hoverSfx.rebuild();
          rewired = true;
        }
        wireEdgeHit(
          edge,
          snap,
          lineW,
          undrawnW,
          rt.edges,
          onEdgeClick,
          hoverSfx,
          inputEnabled,
        );
      }
      const owner = snap.edgeOwner[edge.id] ?? -1;
      const drawn = owner >= 0;
      if (drawn === edge.drawn && owner === edge.owner) continue;
      // Non-animated sync (e.g. missed lastMove).
      if (!lastMove || lastMove.edgeId !== edge.id) {
        edge.drawn = drawn;
        edge.owner = owner;
        drawEdgeLine(
          edge.line,
          edge.a,
          edge.b,
          strokeColor(owner, drawn),
          drawn ? lineW : undrawnW,
          drawn ? 1 : UNDRAWN_ALPHA,
        );
      }
    }
    rt.wiredPlayer = snap.currentPlayer;
  }

  const moveKey = lastMove
    ? `${lastMove.edgeId}:${lastMove.boxIds.join(',')}:${lastMove.winner}`
    : null;
  const isNewMove = lastMove !== null && rt.lastMoveKey !== moveKey;

  if (isNewMove && lastMove) {
    rt.lastMoveKey = moveKey;
    const lineW = Math.max(3, layout.cell * 0.06);
    const edge = rt.edges.get(lastMove.edgeId);
    if (edge) {
      animateEdgeDraw(rt, edge, lastMove.mover, lineW);
    }
  }

  const animating = isNewMove && lastMove ? lastMove.boxIds : [];
  for (let id = 0; id < snap.boxCount; id++) {
    const owner = snap.boxOwner[id] ?? -1;
    if (owner < 0 || rt.boxes.has(id)) continue;
    const staggerIdx = animating.indexOf(id);
    if (staggerIdx >= 0) {
      animateBoxClaim(
        rt,
        layers,
        id,
        owner,
        layout,
        snap.cols,
        staggerIdx * CLAIM_STAGGER_MS,
      );
    } else {
      instantFillBox(rt, layers, id, owner, layout, snap.cols);
    }
  }

  if (
    isNewMove &&
    lastMove &&
    lastMove.isTerminal &&
    (lastMove.winner === 0 || lastMove.winner === 1)
  ) {
    const winner = lastMove.winner;
    const delay =
      lastMove.boxIds.length * CLAIM_STAGGER_MS + BOX_CLAIM_MS * 0.4;
    trackTween(
      rt,
      delayMs(delay, () => pulseWinnerBoxes(rt, winner)),
    );
  }

  paintOverlay(layers.overlay, snap, layout, analysis);
  paintHintOnOverlay(layers.overlay, rt, snap, layout, hintEdgeId);
}

export default function PixiBoard({
  snap,
  lastMove,
  gameGeneration,
  inputEnabled = true,
  analysis = null,
  hintEdgeId = null,
  onEdgeClick,
  onEdgeHover,
  edgeCoord,
}: Props) {
  const hostRef = useRef<HTMLDivElement>(null);
  const appRef = useRef<Application | null>(null);
  const layersRef = useRef<Layers | null>(null);
  const rtRef = useRef<BoardRuntime>({
    layout: null,
    rows: 0,
    cols: 0,
    gameGeneration: -1,
    edges: new Map(),
    boxes: new Map(),
    tweens: [],
    lastMoveKey: null,
    wiredPlayer: -1,
  });
  const snapRef = useRef(snap);
  const lastMoveRef = useRef(lastMove);
  const gameGenerationRef = useRef(gameGeneration);
  const inputEnabledRef = useRef(inputEnabled);
  const analysisRef = useRef(analysis);
  const hintEdgeIdRef = useRef(hintEdgeId);
  const onClickRef = useRef(onEdgeClick);
  const onHoverRef = useRef(onEdgeHover);
  const edgeCoordRef = useRef(edgeCoord);
  const hoverSfxRef = useRef<ReturnType<
    typeof createHoverSfxController
  > | null>(null);

  snapRef.current = snap;
  lastMoveRef.current = lastMove;
  gameGenerationRef.current = gameGeneration;
  inputEnabledRef.current = inputEnabled;
  analysisRef.current = analysis;
  hintEdgeIdRef.current = hintEdgeId;
  onClickRef.current = onEdgeClick;
  onHoverRef.current = onEdgeHover;
  edgeCoordRef.current = edgeCoord;

  const paint = () => {
    const app = appRef.current;
    const host = hostRef.current;
    const layers = layersRef.current;
    const hoverSfx = hoverSfxRef.current;
    if (!app || !host || !layers || !hoverSfx) return;
    const s = snapRef.current;
    const w = host.clientWidth || 360;
    const h = host.clientHeight || 360;
    app.renderer.resize(w, h);
    const layout = computeLayout(s.rows, s.cols, w, h);
    syncBoard(
      layers,
      rtRef.current,
      s,
      layout,
      lastMoveRef.current,
      gameGenerationRef.current,
      (id) => edgeCoordRef.current(id),
      (id) => onClickRef.current(id),
      hoverSfx,
      inputEnabledRef.current,
      analysisRef.current,
      hintEdgeIdRef.current,
    );
  };

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;

    let cancelled = false;
    let resizeObserver: ResizeObserver | null = null;
    const overlay = new Container();
    const boxes = new Container();
    const edges = new Container();
    const dots = new Container();
    const hoverSfx = createHoverSfxController((edgeId) => {
      onHoverRef.current?.(edgeId);
    });
    hoverSfxRef.current = hoverSfx;

    const app = new Application();

    const destroyApp = (instance: Application) => {
      try {
        instance.destroy(true, { children: true });
      } catch {
        /* StrictMode mid-init */
      }
    };

    (async () => {
      try {
        await app.init({
          backgroundAlpha: 0,
          antialias: true,
          resolution: Math.min(window.devicePixelRatio || 1, 2),
          autoDensity: true,
          width: host.clientWidth || 360,
          height: host.clientHeight || 360,
        });
      } catch (err) {
        if (!cancelled) console.error('Pixi init failed', err);
        return;
      }

      if (cancelled) {
        destroyApp(app);
        return;
      }

      host.appendChild(app.canvas);
      app.stage.addChild(boxes);
      app.stage.addChild(edges);
      app.stage.addChild(overlay);
      app.stage.addChild(dots);
      appRef.current = app;
      layersRef.current = { overlay, boxes, edges, dots };
      paint();

      resizeObserver = new ResizeObserver(() => {
        // Force rebuild geometry on resize.
        rtRef.current.layout = null;
        paint();
      });
      resizeObserver.observe(host);
    })();

    const edgesMap = rtRef.current.edges;
    const boxesMap = rtRef.current.boxes;
    const runtime = rtRef.current;

    return () => {
      cancelled = true;
      hoverSfx.dispose();
      if (hoverSfxRef.current === hoverSfx) hoverSfxRef.current = null;
      cancelTweens(runtime);
      resizeObserver?.disconnect();
      if (appRef.current === app) {
        destroyApp(app);
        appRef.current = null;
      }
      layersRef.current = null;
      edgesMap.clear();
      boxesMap.clear();
      host.replaceChildren();
    };
  }, []);

  useEffect(() => {
    paint();
  }, [snap, lastMove, gameGeneration, inputEnabled, analysis, hintEdgeId]);

  return <div className="board-host" ref={hostRef} aria-label="Dots and Boxes board" />;
}
