import type {
  AiWorkerBody,
  AiWorkerRequest,
  AiWorkerResponse,
} from './aiWorkerProtocol';

type Pending = {
  resolve: (value: number | undefined) => void;
  reject: (err: Error) => void;
};

/**
 * Main-thread queue over the AI worker. Calls are serial so play() always
 * lands before the next chooseMove / perfectValue.
 */
export class AiEngine {
  private readonly worker: Worker;
  private nextId = 1;
  private readonly pending = new Map<number, Pending>();
  private tail: Promise<unknown> = Promise.resolve();

  constructor(worker?: Worker) {
    this.worker =
      worker ??
      new Worker(new URL('./aiWorker.ts', import.meta.url), {
        type: 'module',
      });
    this.worker.onmessage = (event: MessageEvent<AiWorkerResponse>) => {
      const msg = event.data;
      const wait = this.pending.get(msg.id);
      if (!wait) return;
      this.pending.delete(msg.id);
      if (msg.ok) wait.resolve(msg.value);
      else wait.reject(new Error(msg.error));
    };
    this.worker.onerror = (event) => {
      const err = new Error(event.message || 'AI worker failed');
      for (const wait of this.pending.values()) wait.reject(err);
      this.pending.clear();
    };
  }

  newGame(size: number): Promise<void> {
    return this.call({
      type: 'newGame',
      rows: size,
      cols: size,
    }).then(() => undefined);
  }

  play(edge: number): Promise<void> {
    return this.call({ type: 'play', edge }).then(() => undefined);
  }

  async chooseMove(policy: number, seed: bigint): Promise<number> {
    const value = await this.call({ type: 'chooseMove', policy, seed });
    if (value === undefined) throw new Error('chooseMove returned no edge');
    return value;
  }

  async perfectValue(): Promise<number> {
    const value = await this.call({ type: 'perfectValue' });
    if (value === undefined) throw new Error('perfectValue returned no margin');
    return value;
  }

  private call(body: AiWorkerBody): Promise<number | undefined> {
    const run = () =>
      new Promise<number | undefined>((resolve, reject) => {
        const id = this.nextId++;
        this.pending.set(id, { resolve, reject });
        this.worker.postMessage({ ...body, id } satisfies AiWorkerRequest);
      });
    const result = this.tail.then(run, run);
    this.tail = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  }
}

let engine: AiEngine | null = null;

export async function initAiEngine(): Promise<void> {
  if (!engine) {
    engine = new AiEngine();
  }
}

export function getAiEngine(): AiEngine {
  if (!engine) {
    throw new Error('AI worker is not initialized');
  }
  return engine;
}
