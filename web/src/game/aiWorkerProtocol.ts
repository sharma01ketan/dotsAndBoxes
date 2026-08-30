export type AiWorkerBody =
  | { type: 'newGame'; rows: number; cols: number }
  | { type: 'play'; edge: number }
  | { type: 'chooseMove'; policy: number; seed: bigint }
  | { type: 'perfectValue' };

export type AiWorkerRequest = AiWorkerBody & { id: number };

export type AiWorkerResponse =
  | { id: number; ok: true; value?: number }
  | { id: number; ok: false; error: string };
