const ISO_DATE = /^\d{4}-\d{2}-\d{2}/;

/**
 * Opening instructions for a prompt that hands work to a coding agent. The
 * agent must restate the work and question the user before it implements.
 */
export function workInstructions(source: "ticket" | "note"): string {
  return (
    `Read the ${source} below and investigate the relevant code before changing anything. ` +
    `Then tell me what you understand the ${source} to ask for and how you plan to do it.\n\n` +
    "Before you implement, grill me on anything you are not sure about: requirements, edge cases, scope, " +
    "and how the result will be verified. Ask one question at a time and do not guess. " +
    "Start the work only once we both agree on what needs doing, then implement and verify it."
  );
}

/** Returns the `YYYY-MM-DD` part of an ISO timestamp, or null for other text. */
export function isoDate(value: string): string | null {
  return ISO_DATE.exec(value)?.[0] ?? null;
}
