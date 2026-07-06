import { describe, expect, it, vi } from "vitest";

import { withRetry } from "./retry";

const noSleep = () => Promise.resolve();

describe("withRetry", () => {
  it("returns the result without retrying when the first attempt succeeds", async () => {
    const fn = vi.fn(async () => "ok");
    const sleep = vi.fn(noSleep);

    const result = await withRetry(fn, { attempts: 5, delayMs: 400, sleep });

    expect(result).toBe("ok");
    expect(fn).toHaveBeenCalledTimes(1);
    expect(sleep).not.toHaveBeenCalled();
  });

  it("retries after failures and returns once an attempt succeeds", async () => {
    const fn = vi
      .fn()
      .mockRejectedValueOnce(new Error("not ready"))
      .mockRejectedValueOnce(new Error("not ready"))
      .mockResolvedValueOnce("recovered");
    const sleep = vi.fn(noSleep);

    const result = await withRetry(fn, { attempts: 5, delayMs: 400, sleep });

    expect(result).toBe("recovered");
    expect(fn).toHaveBeenCalledTimes(3);
    // slept between the two failures and the success: 2 sleeps
    expect(sleep).toHaveBeenCalledTimes(2);
    expect(sleep).toHaveBeenCalledWith(400);
  });

  it("throws the last error after exhausting all attempts", async () => {
    const fn = vi.fn(async () => {
      throw new Error("still down");
    });
    const sleep = vi.fn(noSleep);

    await expect(withRetry(fn, { attempts: 3, delayMs: 10, sleep })).rejects.toThrow("still down");
    expect(fn).toHaveBeenCalledTimes(3);
    // sleeps only BETWEEN attempts, not after the final one
    expect(sleep).toHaveBeenCalledTimes(2);
  });

  it("does not retry when shouldRetry returns false", async () => {
    const fn = vi.fn(async () => {
      throw new Error("fatal");
    });
    const sleep = vi.fn(noSleep);

    await expect(
      withRetry(fn, { attempts: 5, delayMs: 400, sleep, shouldRetry: () => false }),
    ).rejects.toThrow("fatal");
    expect(fn).toHaveBeenCalledTimes(1);
    expect(sleep).not.toHaveBeenCalled();
  });

  it("passes the error to shouldRetry", async () => {
    const fn = vi.fn(async () => {
      throw new Error("nope");
    });
    const shouldRetry = vi.fn(() => false);

    await expect(
      withRetry(fn, { attempts: 5, delayMs: 400, sleep: noSleep, shouldRetry }),
    ).rejects.toThrow("nope");
    expect(shouldRetry).toHaveBeenCalledWith(expect.any(Error));
  });
});
