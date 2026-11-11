import { describe, expect, it } from "vitest";

import { MAX_ZOOM, MIN_ZOOM, zoomIn, zoomLabel, zoomOut } from "./image-zoom";

describe("image zoom", () => {
  it("steps in and out by the same factor", () => {
    expect(zoomIn(1)).toBe(1.25);
    expect(zoomOut(1.25)).toBe(1);
    expect(zoomOut(1)).toBe(0.8);
  });

  it("stays within the zoom limits", () => {
    expect(zoomIn(MAX_ZOOM)).toBe(MAX_ZOOM);
    expect(zoomIn(MAX_ZOOM - 0.01)).toBe(MAX_ZOOM);
    expect(zoomOut(MIN_ZOOM)).toBe(MIN_ZOOM);
    expect(zoomOut(MIN_ZOOM + 0.01)).toBe(MIN_ZOOM);
  });

  it("labels a zoom level as a percentage and no zoom level as Fit", () => {
    expect(zoomLabel(null)).toBe("Fit");
    expect(zoomLabel(1)).toBe("100%");
    expect(zoomLabel(0.8)).toBe("80%");
    expect(zoomLabel(1.5625)).toBe("156%");
  });
});
