/** Parsed `WasmGame.analyze()` dump. See docs/specs/phase2-theory-overlay.md. */

export type AnalysisRegion = {
  kind: 0 | 1 | 2;
  length: number;
  boxes: number[];
};

export type AnalysisSnapshot = {
  decomposed: boolean;
  longChainCount: number;
  shortChainCount: number;
  loopCount: number;
  takeableCount: number;
  longParity: number;
  targetParity: number;
  regions: AnalysisRegion[];
  takeables: number[];
};

export function parseAnalysisDump(
  raw: ArrayLike<number>,
): AnalysisSnapshot | null {
  if (raw.length < 8) return null;
  const decomposed = raw[0]! !== 0;
  const longChainCount = raw[1]!;
  const shortChainCount = raw[2]!;
  const loopCount = raw[3]!;
  const takeableCount = raw[4]!;
  const longParity = raw[5]!;
  const targetParity = raw[6]!;
  const regionCount = raw[7]!;
  let i = 8;
  const regions: AnalysisRegion[] = [];
  for (let r = 0; r < regionCount; r++) {
    if (i + 3 > raw.length) return null;
    const kind = raw[i]! as 0 | 1 | 2;
    const length = raw[i + 1]!;
    const n = raw[i + 2]!;
    i += 3;
    if (i + n > raw.length) return null;
    const boxes: number[] = [];
    for (let b = 0; b < n; b++) boxes.push(raw[i + b]!);
    i += n;
    regions.push({ kind, length, boxes });
  }
  if (i + takeableCount > raw.length) return null;
  const takeables: number[] = [];
  for (let t = 0; t < takeableCount; t++) takeables.push(raw[i + t]!);
  return {
    decomposed,
    longChainCount,
    shortChainCount,
    loopCount,
    takeableCount,
    longParity,
    targetParity,
    regions,
    takeables,
  };
}

export function analysisLine(a: AnalysisSnapshot): string {
  const target = a.targetParity === 0 ? 'even' : 'odd';
  const have =
    a.longParity === a.targetParity
      ? 'to-move has this parity'
      : 'to-move lacks this parity';
  return `L=${a.longChainCount} · target ${target} · ${have}`;
}
