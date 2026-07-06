export interface RetryOptions {
  /** Total number of attempts, including the first (must be >= 1). */
  attempts: number;
  /** Delay between attempts, in milliseconds. */
  delayMs: number;
  /** Injectable sleep, primarily for tests. Defaults to setTimeout. */
  sleep?: (ms: number) => Promise<void>;
  /** Return false to stop retrying a given error. Defaults to always retry. */
  shouldRetry?: (error: unknown) => boolean;
}

/**
 * Run `fn`, retrying on rejection up to `attempts` times with a fixed delay
 * between tries. Returns on the first success (no delay on the happy path);
 * rethrows the last error once attempts are exhausted or `shouldRetry` says stop.
 *
 * Intended for idempotent operations (e.g. read commands). Never wrap writes,
 * which could be applied more than once.
 */
export async function withRetry<T>(fn: () => Promise<T>, options: RetryOptions): Promise<T> {
  const sleep = options.sleep ?? ((ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms)));
  const shouldRetry = options.shouldRetry ?? (() => true);
  const attempts = Math.max(1, options.attempts);

  let lastError: unknown;
  for (let attempt = 1; attempt <= attempts; attempt++) {
    try {
      return await fn();
    } catch (error) {
      lastError = error;
      if (attempt >= attempts || !shouldRetry(error)) {
        break;
      }
      await sleep(options.delayMs);
    }
  }

  throw lastError;
}
