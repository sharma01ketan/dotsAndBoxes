import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { analysisLine, parseAnalysisDump } from './analysis.ts';

describe('parseAnalysisDump', () => {
  it('parses a 1×3 long-chain header and boxes', () => {
    const raw = [1, 1, 0, 0, 0, 1, 0, 1, 1, 3, 3, 0, 1, 2];
    const a = parseAnalysisDump(raw);
    assert.ok(a);
    assert.equal(a.decomposed, true);
    assert.equal(a.longChainCount, 1);
    assert.equal(a.regions.length, 1);
    assert.equal(a.regions[0]?.kind, 1);
    assert.deepEqual(a.regions[0]?.boxes, [0, 1, 2]);
    assert.deepEqual(a.takeables, []);
  });

  it('returns null on a truncated dump', () => {
    assert.equal(parseAnalysisDump([1, 1, 0]), null);
  });
});

describe('analysisLine', () => {
  it('names even target and whether to-move has it', () => {
    const a = parseAnalysisDump([1, 1, 0, 0, 0, 1, 0, 0])!;
    assert.equal(
      analysisLine(a),
      'L=1 · target even · to-move lacks this parity',
    );
  });
});
