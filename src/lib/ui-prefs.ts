/**
 * Layout choices that should survive a restart, such as a collapsed panel.
 * They live in browser storage because they belong to this window, not to
 * the user's notes. Reads and writes never throw; storage can be unavailable.
 */
export function readFlag(key: string, fallback: boolean): boolean {
  try {
    const stored = localStorage.getItem(key);
    return stored === null ? fallback : stored === "true";
  } catch {
    return fallback;
  }
}

export function writeFlag(key: string, value: boolean): void {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    // The choice still applies until the window closes.
  }
}

/** True when a key press is meant for a text field, so shortcuts should not fire. */
export function isTypingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  return target.isContentEditable || ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName);
}
