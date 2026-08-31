/** Pixel layout helpers for the Pixi board. */

export type Point = { x: number; y: number };

export type BoardLayout = {
  rows: number;
  cols: number;
  originX: number;
  originY: number;
  cell: number;
  width: number;
  height: number;
  pad: number;
};

export const COLORS = {
  ink: 0x1a1f1c,
  accent: 0xc45c26,
  muted: 0x5c665f,
  ok: 0x2f6f4e,
  undrawn: 0xb8b0a2,
  boxP1: 0x2f6f4e,
  boxP2: 0xc45c26,
  paper: 0xf3efe6,
  overlayShort: 0x8a8474,
  overlayLong: 0x2f6f4e,
  overlayLoop: 0x3d5a80,
} as const;

export function computeLayout(
  rows: number,
  cols: number,
  viewW: number,
  viewH: number,
): BoardLayout {
  const pad = 28;
  const usableW = Math.max(120, viewW - pad * 2);
  const usableH = Math.max(120, viewH - pad * 2);
  const cell = Math.min(usableW / cols, usableH / rows);
  const width = cell * cols;
  const height = cell * rows;
  const originX = (viewW - width) / 2;
  const originY = (viewH - height) / 2;
  return { rows, cols, originX, originY, cell, width, height, pad };
}

export function dotPosition(layout: BoardLayout, row: number, col: number): Point {
  return {
    x: layout.originX + col * layout.cell,
    y: layout.originY + row * layout.cell,
  };
}

/** Horizontal edge endpoints at grid row `r`, between cols `c` and `c+1`. */
export function horizontalEdgeEnds(
  layout: BoardLayout,
  r: number,
  c: number,
): [Point, Point] {
  return [dotPosition(layout, r, c), dotPosition(layout, r, c + 1)];
}

/** Vertical edge endpoints at grid col `c`, between rows `r` and `r+1`. */
export function verticalEdgeEnds(
  layout: BoardLayout,
  r: number,
  c: number,
): [Point, Point] {
  return [dotPosition(layout, r, c), dotPosition(layout, r + 1, c)];
}

export function boxCenter(layout: BoardLayout, row: number, col: number): Point {
  return {
    x: layout.originX + (col + 0.5) * layout.cell,
    y: layout.originY + (row + 0.5) * layout.cell,
  };
}

export function hitPad(layout: BoardLayout): number {
  return Math.max(24, layout.cell * 0.28);
}
