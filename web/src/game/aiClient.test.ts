import assert from 'node:assert/strict';
import { describe, it } from 'node:test';
import { AiEngine } from './aiClient.ts';
import type { AiWorkerRequest, AiWorkerResponse } from './aiWorkerProtocol.ts';

/**
 * CI stand-in for “empty 3×3 Perfect is a Worker round-trip”: search RPCs
 * serialize on the client so play() always lands before chooseMove.
 */
class MockWorker {
  onmessage: ((event: { data: AiWorkerResponse }) => void) | null = null;
  onerror: ((event: { message: string }) => void) | null = null;
  readonly sent: AiWorkerRequest[] = [];

  postMessage(data: AiWorkerRequest) {
    this.sent.push(data);
  }

  reply(id: number, value?: number) {
    this.onmessage?.({ data: { id, ok: true, value } });
  }
}

function engine(): { client: AiEngine; worker: MockWorker } {
  const worker = new MockWorker();
  const client = new AiEngine(worker as unknown as Worker);
  return { client, worker };
}

describe('AiEngine', () => {
  it('does not post the next RPC until the current one replies', async () => {
    const { client, worker } = engine();

    const first = client.chooseMove(3, 1n);
    await Promise.resolve();
    assert.equal(worker.sent.length, 1);
    assert.equal(worker.sent[0]?.type, 'chooseMove');
    assert.equal(worker.sent[0]?.seed, 1n);

    const second = client.perfectValue();
    await Promise.resolve();
    assert.equal(worker.sent.length, 1);

    worker.reply(worker.sent[0]!.id, 4);
    assert.equal(await first, 4);

    await Promise.resolve();
    assert.equal(worker.sent.length, 2);
    assert.equal(worker.sent[1]?.type, 'perfectValue');
    worker.reply(worker.sent[1]!.id, -3);
    assert.equal(await second, -3);
  });

  it('posts play before a queued chooseMove', async () => {
    const { client, worker } = engine();

    const played = client.play(7);
    const chosen = client.chooseMove(1, 2n);
    await Promise.resolve();
    assert.equal(worker.sent[0]?.type, 'play');
    assert.equal(worker.sent[0]?.type === 'play' && worker.sent[0].edge, 7);

    worker.reply(worker.sent[0]!.id);
    await played;
    await Promise.resolve();
    assert.equal(worker.sent[1]?.type, 'chooseMove');
    worker.reply(worker.sent[1]!.id, 9);
    assert.equal(await chosen, 9);
  });
});
