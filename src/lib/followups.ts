import type { FollowupItem, FollowupLane } from "./types";

export function resolveFollowupLane(followup: FollowupItem): string {
  const explicit = followup.followupLane?.trim();
  if (explicit) {
    return explicit;
  }

  const project = followup.tags.find((tag) => tag.kind === "project")?.name.trim();
  if (project) {
    return project;
  }

  const topic = followup.tags.find((tag) => tag.kind === "topic")?.name.trim();
  if (topic) {
    return topic;
  }

  return "Unsorted";
}

export function displayFollowupState(followup: FollowupItem): "Open" | "Waiting" | "Blocked" | "Done" {
  if (followup.status === "done") {
    return "Done";
  }

  switch (followup.followupState ?? "open") {
    case "waiting":
      return "Waiting";
    case "blocked":
      return "Blocked";
    case "open":
      return "Open";
  }
}

export function groupFollowupsByLane(followups: FollowupItem[]): FollowupLane[] {
  const lanes = new Map<string, FollowupItem[]>();

  for (const followup of followups) {
    const lane = resolveFollowupLane(followup);
    lanes.set(lane, [...(lanes.get(lane) ?? []), followup]);
  }

  return Array.from(lanes.entries())
    .map(([name, laneFollowups]) => ({
      name,
      activeCount: laneFollowups.filter((followup) => followup.status !== "done").length,
      followups: laneFollowups,
    }))
    .sort((left, right) => {
      if (left.name === right.name) {
        return 0;
      }
      if (left.name === "Unsorted") {
        return 1;
      }
      if (right.name === "Unsorted") {
        return -1;
      }
      return left.name.localeCompare(right.name);
    });
}
