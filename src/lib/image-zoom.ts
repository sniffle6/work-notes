/** Smallest and largest scale of an image relative to its real size. */
export const MIN_ZOOM = 0.1;
export const MAX_ZOOM = 8;
const ZOOM_STEP = 1.25;
// Keeps repeated steps from drifting, so stepping in then out returns to the same scale.
const SCALE_PRECISION = 10_000;

function clamp(scale: number): number {
  const rounded = Math.round(scale * SCALE_PRECISION) / SCALE_PRECISION;
  return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, rounded));
}

/** The next larger scale, up to `MAX_ZOOM`. */
export function zoomIn(scale: number): number {
  return clamp(scale * ZOOM_STEP);
}

/** The next smaller scale, down to `MIN_ZOOM`. */
export function zoomOut(scale: number): number {
  return clamp(scale / ZOOM_STEP);
}

/** Describes a scale as a percentage. `null` means the image is fitted to its window. */
export function zoomLabel(scale: number | null): string {
  return scale === null ? "Fit" : `${Math.round(scale * 100)}%`;
}
