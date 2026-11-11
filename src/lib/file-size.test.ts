import { describe, expect, it } from "vitest";

import { formatFileSize } from "./file-size";

describe("formatFileSize", () => {
  it("picks the largest unit that keeps the number at one or more", () => {
    expect(formatFileSize(0)).toBe("0 B");
    expect(formatFileSize(1023)).toBe("1023 B");
    expect(formatFileSize(1024)).toBe("1.0 KB");
    expect(formatFileSize(2048)).toBe("2.0 KB");
    expect(formatFileSize(15 * 1024)).toBe("15 KB");
    expect(formatFileSize(4.25 * 1024 * 1024)).toBe("4.3 MB");
    expect(formatFileSize(3 * 1024 ** 3)).toBe("3.0 GB");
    expect(formatFileSize(5000 * 1024 ** 3)).toBe("5000 GB");
  });
});
