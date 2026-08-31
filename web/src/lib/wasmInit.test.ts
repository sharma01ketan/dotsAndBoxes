import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { createWasmInit } from './wasmInit.ts';

describe('createWasmInit', () => {
  it('retries after a rejected init', async () => {
    let calls = 0;
    const run = createWasmInit(async () => {
      calls += 1;
      if (calls === 1) throw new Error('load failed');
    });

    await assert.rejects(run, /load failed/);
    assert.equal(calls, 1);

    await run();
    assert.equal(calls, 2);

    await run();
    assert.equal(calls, 2);
  });
});
