/** Cache `load()` once; clear on reject so the next call retries. */
export function createWasmInit(load: () => Promise<unknown>): () => Promise<void> {
  let ready: Promise<void> | null = null;
  return () => {
    if (!ready) {
      ready = load().then(
        () => undefined,
        (err: unknown) => {
          ready = null;
          throw err;
        },
      );
    }
    return ready;
  };
}
