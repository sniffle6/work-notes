const BYTES_PER_UNIT = 1024;
const UNITS = ["B", "KB", "MB", "GB"];

/** Formats a byte count for display, such as `2.0 KB` or `15 MB`. */
export function formatFileSize(bytes: number): string {
  let value = bytes;
  let unit = 0;
  while (value >= BYTES_PER_UNIT && unit < UNITS.length - 1) {
    value /= BYTES_PER_UNIT;
    unit += 1;
  }

  const digits = unit > 0 && value < 10 ? 1 : 0;
  return `${value.toFixed(digits)} ${UNITS[unit]}`;
}
