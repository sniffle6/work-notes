/* global React, Icon */
// Sample notes — realistic, varied, drawn from coworker drive-by scenarios.

const NOW = new Date("2026-05-21T15:42:00");
const ago = (mins) => new Date(NOW.getTime() - mins * 60 * 1000);

const SAMPLE_NOTES = [
  {
    id: "n1",
    title: "Maria — Q3 dashboard owner unclear",
    captureSource: "Hotkey",
    when: ago(8),
    parseStatus: "parsed",
    reviewStatus: "needs_review",
    pinned: false,
    raw: "stopped by maria's desk, says q3 dashboard is broken since the migration, no clear owner since dan left. needs decision by fri. wants to know if we should rebuild on the new metrics service or patch the old one. she'll send the failing queries.",
    cleaned:
      "Maria stopped by: the Q3 dashboard has been broken since the metrics migration and has no clear owner since Dan left. She needs a decision by Friday — rebuild on the new metrics service, or patch the old one. She'll send the failing queries.",
    summary: "Q3 dashboard is unowned post-migration. Decision needed by Friday: rebuild or patch.",
    tags: [
      { name: "Maria", kind: "person" },
      { name: "Q3-dashboard", kind: "project" },
      { name: "metrics", kind: "topic" },
      { name: "blocker", kind: "urgency" },
    ],
    actions: [
      { id: "a1", text: "Decide: rebuild dashboard vs. patch", owner: "me", due: "Fri", status: "suggested" },
      { id: "a2", text: "Forward failing queries to platform team", owner: "Maria", due: null, status: "suggested" },
      { id: "a3", text: "Name an interim owner until backfill", owner: "me", due: "Mon", status: "suggested" },
    ],
  },
  {
    id: "n2",
    title: "Jin — needs OAuth scope review before launch",
    captureSource: "Quick capture",
    when: ago(34),
    parseStatus: "parsed",
    reviewStatus: "none",
    pinned: true,
    raw: "jin: oauth scopes for the new integration look too broad. wants security review before launch monday. ping kavi.",
    cleaned:
      "Jin flagged that OAuth scopes for the new integration look too broad and wants a security review before Monday's launch. Loop in Kavi.",
    summary: "Security review on OAuth scopes before Monday launch.",
    tags: [
      { name: "Jin", kind: "person" },
      { name: "Kavi", kind: "person" },
      { name: "integrations", kind: "project" },
    ],
    actions: [
      { id: "a4", text: "Schedule scope review with Kavi", owner: "me", due: "Today", status: "suggested" },
    ],
  },
  {
    id: "n3",
    title: "Standup: pricing page A/B is inconclusive",
    captureSource: "Quick capture",
    when: ago(96),
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    raw: "standup — pricing a/b inconclusive after 2 weeks. low traffic on the test cohort. eng suggests killing the test and trying a heavier tier-3 callout instead.",
    cleaned:
      "Pricing A/B is inconclusive after two weeks — the test cohort got low traffic. Eng suggests killing the test and trying a heavier tier-3 callout instead.",
    summary: "Pricing A/B inconclusive — consider replacing with tier-3 callout.",
    tags: [
      { name: "pricing", kind: "topic" },
      { name: "experiments", kind: "category" },
    ],
    actions: [],
  },
  {
    id: "n4",
    title: "Codex returned malformed JSON",
    captureSource: "Hotkey",
    when: ago(140),
    parseStatus: "failed",
    reviewStatus: "needs_review",
    raw: "called payroll. they need updated headcount by EOM. no other context.",
    cleaned: null,
    summary: null,
    parseError: "Parser output did not match parse-note schema: missing field `summary`.",
    tags: [],
    actions: [],
  },
  {
    id: "n5",
    title: "Anita — wants weekly customer pain digest",
    captureSource: "Hotkey",
    when: ago(220),
    parseStatus: "parsed",
    reviewStatus: "none",
    raw: "anita wants a weekly digest of customer pain from support + sales. happy to take it if eng feeds the data.",
    cleaned:
      "Anita wants a weekly digest of customer pain, pulled from support and sales. She'll take it if eng feeds her the data.",
    summary: "Anita will own weekly customer-pain digest if eng pipes data.",
    tags: [
      { name: "Anita", kind: "person" },
      { name: "voice-of-customer", kind: "project" },
    ],
    actions: [
      { id: "a5", text: "Spec the data feed", owner: "me", due: "Next week", status: "suggested" },
    ],
  },
  {
    id: "n6",
    title: "Sam — onboarding deck stale",
    captureSource: "Quick capture",
    when: ago(360),
    parseStatus: "parsing",
    reviewStatus: "none",
    raw: "sam — new-hire deck still references old org chart. wants someone to own it. low priority.",
    cleaned: null,
    summary: null,
    tags: [],
    actions: [],
  },
  {
    id: "n7",
    title: "Hallway: roadmap review moved to Thursday",
    captureSource: "Hotkey",
    when: ago(1280),
    parseStatus: "parsed",
    reviewStatus: "reviewed",
    raw: "ran into priya — roadmap review pushed to thursday 2pm. she'll send updated deck wednesday eod.",
    cleaned: "Roadmap review moved to Thursday 2pm. Priya will send the updated deck by Wednesday EOD.",
    summary: "Roadmap review is Thu 2pm; deck Wed EOD.",
    tags: [
      { name: "Priya", kind: "person" },
      { name: "roadmap", kind: "topic" },
    ],
    actions: [],
  },
];

window.SAMPLE_NOTES = SAMPLE_NOTES;
window.formatWhen = function formatWhen(date) {
  const diffMin = Math.round((NOW - date) / 60000);
  if (diffMin < 1) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  const diffHr = Math.round(diffMin / 60);
  if (diffHr < 24) return `${diffHr}h ago`;
  const diffDay = Math.round(diffHr / 24);
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
};
window.formatClock = function formatClock(date) {
  return date.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
};
window.NOW = NOW;
