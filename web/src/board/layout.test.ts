import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import {
  dotPosition,
  horizontalEdgeEnds,
  verticalEdgeEnds,
  type BoardLayout,
} from './layout.ts';

const layout: BoardLayout = {
  rows: 2,
  cols: 2,
  originX: 10,
  originY: 10,
  cell: 40,
  width: 80,
  height: 80,
  pad: 28,
};

describe('edge endpoints', () => {
  it('horizontalEdgeEnds match the two dots on that row', () => {
    const [a, b] = horizontalEdgeEnds(layout, 0, 1);
    assert.deepEqual(a, dotPosition(layout, 0, 1));
    assert.deepEqual(b, dotPosition(layout, 0, 2));
  });

  it('verticalEdgeEnds match the two dots on that column', () => {
    const [a, b] = verticalEdgeEnds(layout, 1, 0);
    assert.deepEqual(a, dotPosition(layout, 1, 0));
    assert.deepEqual(b, dotPosition(layout, 2, 0));
  });
});
