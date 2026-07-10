<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Check from "@lucide/svelte/icons/check";
  import Link2 from "@lucide/svelte/icons/link-2";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import {
    buildCalendarMonth,
    buildCalendarTasks,
    buildTaskLifecycle,
    calendarOccurrencesForDate,
    dateValueKey,
    formatMonthHeading,
    formatSelectedDate,
    localDateKey,
    type CalendarDay,
    type CalendarLifecycleMoment,
    type CalendarMomentKind,
    type CalendarOccurrence,
    type CalendarTask,
  } from "$lib/today";
  import type { ActionReviewItem, FollowupItem } from "$lib/types";

  type Props = {
    actions: ActionReviewItem[];
    followups: FollowupItem[];
    loadingActions?: boolean;
    busyActionId?: string | null;
    now?: Date;
  };

  let {
    actions,
    followups,
    loadingActions = false,
    busyActionId = null,
    now = new Date(),
  }: Props = $props();

  let visibleMonth = $state(new Date());
  let selectedDate = $state(new Date());
  let hoveredTaskId = $state<string | null>(null);
  let focusedTaskId = $state<string | null>(null);
  let initializedFromNow = false;

  $effect(() => {
    if (!initializedFromNow) {
      visibleMonth = new Date(now.getFullYear(), now.getMonth(), 1);
      selectedDate = new Date(now.getFullYear(), now.getMonth(), now.getDate());
      initializedFromNow = true;
    }
  });

  const tasks = $derived(buildCalendarTasks(actions, followups));
  const calendarDays = $derived(buildCalendarMonth(tasks, visibleMonth, now));
  const selectedIsToday = $derived(localDateKey(selectedDate) === localDateKey(now));
  const selectedOccurrences = $derived(calendarOccurrencesForDate(tasks, selectedDate));
  const activeTaskId = $derived(focusedTaskId ?? hoveredTaskId);
  const openCount = $derived(tasks.filter((task) => task.status !== "done").length);
  const monthDoneCount = $derived(
    calendarDays
      .filter((day) => day.isCurrentMonth)
      .reduce((total, day) => total + day.completedCount, 0),
  );

  const dispatch = createEventDispatcher<{
    openNote: string;
    accept: string;
    complete: string;
    reopen: string;
  }>();

  const weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

  function shiftMonth(offset: number): void {
    visibleMonth = new Date(visibleMonth.getFullYear(), visibleMonth.getMonth() + offset, 1);
    selectedDate = new Date(visibleMonth.getFullYear(), visibleMonth.getMonth(), 1);
  }

  function goToToday(): void {
    visibleMonth = new Date(now.getFullYear(), now.getMonth(), 1);
    selectedDate = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  }

  function selectDay(day: CalendarDay): void {
    hoveredTaskId = null;
    focusedTaskId = null;
    selectedDate = new Date(day.date.getFullYear(), day.date.getMonth(), day.date.getDate());
    if (!day.isCurrentMonth) {
      visibleMonth = new Date(day.date.getFullYear(), day.date.getMonth(), 1);
    }
  }

  function dayLabel(day: CalendarDay): string {
    const date = formatSelectedDate(day.date);
    return `Select ${date}, ${day.capturedCount} captured, ${day.dueCount} due, ${day.completedCount} completed`;
  }

  function completedTime(task: CalendarTask): string | null {
    if (!task.completedAt) return null;
    const date = new Date(task.completedAt);
    if (Number.isNaN(date.getTime())) return null;
    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }

  function momentLabel(kind: CalendarMomentKind): string {
    if (kind === "captured") return "Captured";
    if (kind === "due") return "Due";
    return "Completed";
  }

  function isCompletedOccurrence(occurrence: CalendarOccurrence): boolean {
    return occurrence.kinds.includes("completed");
  }

  function momentTone(occurrence: CalendarOccurrence): CalendarMomentKind {
    if (occurrence.kinds.includes("completed")) return "completed";
    if (occurrence.kinds.includes("due")) return "due";
    return "captured";
  }

  function shortDate(value: string | null | undefined): string | null {
    if (!value) return null;
    const dateOnly = /^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
    const date = dateOnly
      ? new Date(Number(dateOnly[1]), Number(dateOnly[2]) - 1, Number(dateOnly[3]))
      : new Date(value);
    if (Number.isNaN(date.getTime())) return null;
    return date.toLocaleDateString([], { month: "short", day: "numeric" });
  }

  function relatedMoments(occurrence: CalendarOccurrence): string[] {
    const { task } = occurrence;
    const moments: Array<[CalendarMomentKind, string | null | undefined]> = [
      ["captured", task.capturedAt],
      ["due", task.dueDate],
      ["completed", task.status === "done" ? task.completedAt : null],
    ];

    return moments.flatMap(([kind, value]) => {
      if (!value || occurrence.kinds.includes(kind) || dateValueKey(value) === occurrence.dateKey) return [];
      const date = shortDate(value);
      return date ? [`${momentLabel(kind)} ${date}`] : [];
    });
  }

  function taskState(task: CalendarTask): string {
    if (task.status === "done") return "Done";
    if (task.status === "suggested") return "Needs review";
    return task.dueDate ? "Open" : "Open · no deadline";
  }

  function visibleDayOccurrences(occurrences: CalendarOccurrence[]): CalendarOccurrence[] {
    const visible = occurrences.slice(0, 2);
    if (!activeTaskId || visible.some((occurrence) => occurrence.task.id === activeTaskId)) {
      return visible;
    }

    const active = occurrences.find((occurrence) => occurrence.task.id === activeTaskId);
    return active ? [active, ...visible.slice(0, 1)] : visible;
  }

  function lifecycleDateLabel(moment: CalendarLifecycleMoment): string {
    const date = shortDate(moment.dateKey) ?? moment.dateKey;
    const visibleMonthKey = `${visibleMonth.getFullYear()}-${`${visibleMonth.getMonth() + 1}`.padStart(2, "0")}`;
    const momentMonthKey = moment.dateKey.slice(0, 7);
    if (momentMonthKey < visibleMonthKey) return `← ${date}`;
    if (momentMonthKey > visibleMonthKey) return `${date} →`;
    return date;
  }

  function handleTaskFocusOut(event: FocusEvent, taskId: string): void {
    const row = event.currentTarget as HTMLElement;
    const nextTarget = event.relatedTarget as Node | null;
    if ((!nextTarget || !row.contains(nextTarget)) && focusedTaskId === taskId) {
      focusedTaskId = null;
    }
  }
</script>

{#snippet taskRow(occurrence: CalendarOccurrence)}
  {@const task = occurrence.task}
  {@const relations = relatedMoments(occurrence)}
  {@const lifecycle = buildTaskLifecycle(task, selectedDate, now)}
  <article
    class:done={task.status === "done"}
    class:link-active={activeTaskId === task.id}
    class="task-row"
    data-task-id={task.id}
    onmouseenter={() => hoveredTaskId = task.id}
    onmouseleave={() => hoveredTaskId = hoveredTaskId === task.id ? null : hoveredTaskId}
    onfocusin={() => focusedTaskId = task.id}
    onfocusout={(event) => handleTaskFocusOut(event, task.id)}
  >
    <button
      class="task-copy"
      type="button"
      aria-label={`Open note: ${task.noteTitle}`}
      onclick={() => dispatch("openNote", task.noteId)}
    >
      <span class="moment-badges">
        {#each occurrence.kinds as kind}
          <span class={`moment-badge ${kind}`}>{momentLabel(kind)}</span>
        {/each}
      </span>
      <span class="task-text">{task.text}</span>
      <span class="lifecycle-rail" aria-hidden="true">
        {#each lifecycle as moment, index}
          <span
            class:overdue={moment.isOverdue}
            class={`lifecycle-moment ${moment.kind} ${moment.position}`}
          >
            <span class="lifecycle-node">{moment.kind === "completed" ? "✓" : ""}</span>
            <span class="lifecycle-copy">
              <span>{momentLabel(moment.kind)}</span>
              <span>{lifecycleDateLabel(moment)}</span>
            </span>
          </span>
          {#if index < lifecycle.length - 1}<span class="lifecycle-connector"></span>{/if}
        {/each}
      </span>
      <span class="task-meta">
        {#if task.owner}<span class="owner">@{task.owner}</span>{/if}
        <span>{taskState(task)}</span>
        {#if occurrence.kinds.includes("completed") && completedTime(task)}
          <span>{completedTime(task)}</span>
        {/if}
        {#if relations.length > 0}
          <span class="task-relations"><Link2 size={10} />{relations.join(" · ")}</span>
        {:else if task.status === "done" && !task.completedAt}
          <span class="completion-unknown">Completion date unavailable</span>
        {/if}
        <span>from “{task.noteTitle}”</span>
      </span>
    </button>

    {#if task.status === "suggested"}
      <button
        class="task-action accept"
        type="button"
        disabled={loadingActions || busyActionId === task.id}
        aria-label={`Accept task: ${task.text}`}
        onclick={() => dispatch("accept", task.id)}
      >Accept</button>
    {:else if task.status === "accepted"}
      <button
        class="task-action complete"
        type="button"
        disabled={loadingActions || busyActionId === task.id}
        aria-label={`Mark done: ${task.text}`}
        onclick={() => dispatch("complete", task.id)}
      ><Check size={14} /> Done</button>
    {:else}
      <button
        class="task-action reopen"
        type="button"
        disabled={loadingActions || busyActionId === task.id}
        aria-label={`Reopen task: ${task.text}`}
        onclick={() => dispatch("reopen", task.id)}
      ><RotateCcw size={13} /> Reopen</button>
    {/if}
  </article>
{/snippet}

<section class="calendar-view" aria-label="Calendar">
  <header class="calendar-head">
    <div class="eyebrow">Calendar</div>
    <div class="month-navigation" role="group" aria-label="Calendar navigation">
      <button type="button" aria-label="Previous month" onclick={() => shiftMonth(-1)}>
        <ChevronLeft size={17} />
      </button>
      <h1>{formatMonthHeading(visibleMonth)}</h1>
      <button type="button" aria-label="Next month" onclick={() => shiftMonth(1)}>
        <ChevronRight size={17} />
      </button>
      <button class="today-button" type="button" onclick={goToToday}>Today</button>
    </div>
    <p>{openCount} open · {monthDoneCount} completed this month</p>
  </header>

  <div class="calendar-layout">
    <section class="month-panel" aria-label={formatMonthHeading(visibleMonth)}>
      <div class="weekday-row" aria-hidden="true">
        {#each weekdays as weekday}<span>{weekday}</span>{/each}
      </div>

      <div class="month-grid">
        {#each calendarDays as day}
          {@const dayOccurrences = calendarOccurrencesForDate(tasks, day.date)}
          <button
            type="button"
            class:outside={!day.isCurrentMonth}
            class:today={day.isToday}
            class:selected={localDateKey(selectedDate) === day.key}
            class="calendar-day"
            aria-label={dayLabel(day)}
            aria-pressed={localDateKey(selectedDate) === day.key}
            onclick={() => selectDay(day)}
          >
            <span class="day-number">{day.date.getDate()}</span>
            <span class="day-items" aria-hidden="true">
              {#each visibleDayOccurrences(dayOccurrences) as occurrence}
                <span
                  class:echo-highlighted={activeTaskId === occurrence.task.id}
                  class:echo-muted={activeTaskId !== null && activeTaskId !== occurrence.task.id}
                  class={`day-task ${momentTone(occurrence)}`}
                  data-task-id={occurrence.task.id}
                >
                  {#if isCompletedOccurrence(occurrence)}
                    <span class="completion-mark">✓</span>
                  {/if}
                  {occurrence.task.text}
                </span>
              {/each}
            </span>
            {#if day.capturedCount > 0 || day.dueCount > 0 || day.completedCount > 0}
              <span class="day-counts" aria-hidden="true">
                {#if day.capturedCount > 0}<span class="captured-count">{day.capturedCount} captured</span>{/if}
                {#if day.dueCount > 0}<span class="due-count">{day.dueCount} due</span>{/if}
                {#if day.completedCount > 0}<span class="completed-count">{day.completedCount} done</span>{/if}
              </span>
            {/if}
          </button>
        {/each}
      </div>
    </section>

    <aside class="agenda-panel" aria-label={`Tasks for ${formatSelectedDate(selectedDate)}`}>
      <header class="agenda-head">
        <span>{selectedIsToday ? "Today" : "Selected day"}</span>
        <h2>{formatSelectedDate(selectedDate)}</h2>
      </header>

      <section class="bucket" aria-label="Task activity">
        <div class="bucket-head">
          <h3>Activity</h3>
          <span>{selectedOccurrences.length}</span>
        </div>
        {#if selectedOccurrences.length === 0}
          <p class="empty">No task activity for this day.</p>
        {:else}
          <div class="task-list">
            {#each selectedOccurrences as occurrence}{@render taskRow(occurrence)}{/each}
          </div>
        {/if}
      </section>
    </aside>
  </div>
</section>

<style>
  .calendar-view {
    display: flex;
    flex-direction: column;
    gap: 18px;
    height: 100%;
    min-height: 0;
    padding: 24px 28px 36px;
    color: var(--color-text-primary);
    background: var(--color-app-bg);
    overflow: auto;
  }

  .calendar-head,
  .month-navigation,
  .bucket-head,
  .day-counts,
  .task-meta,
  .task-action {
    display: flex;
    align-items: center;
  }

  .calendar-head {
    align-items: flex-start;
    flex-direction: column;
    gap: 5px;
  }

  h1, h2, h3, p { margin: 0; }

  .eyebrow,
  .agenda-head > span,
  .bucket-head h3,
  .weekday-row {
    color: var(--color-accent-primary);
    font-size: 10px;
    font-weight: 800;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  h1 { min-width: 150px; font-size: 26px; line-height: 1.1; text-align: center; }
  .calendar-head p { color: var(--color-text-muted); font: 12px ui-monospace, SFMono-Regular, Consolas, monospace; }

  .month-navigation { gap: 5px; }
  .month-navigation button {
    display: inline-grid;
    min-width: 32px;
    min-height: 32px;
    place-items: center;
    border: 1px solid var(--color-border-default);
    border-radius: 7px;
    color: var(--color-text-primary);
    background: var(--color-surface-1);
    cursor: pointer;
  }
  .month-navigation button:hover { border-color: var(--color-border-strong); background: var(--color-surface-2); }
  .month-navigation .today-button { margin-left: 7px; padding: 0 12px; font: 700 11px ui-monospace, SFMono-Regular, Consolas, monospace; }

  .calendar-layout {
    display: grid;
    grid-template-columns: minmax(580px, 1fr) minmax(300px, 380px);
    gap: 18px;
    min-height: 620px;
  }

  .month-panel,
  .agenda-panel {
    min-width: 0;
    background: transparent;
  }

  .month-panel {
    display: grid;
    grid-template-rows: 34px 1fr;
    border: 1px solid var(--color-border-default);
    overflow: hidden;
  }
  .weekday-row, .month-grid { display: grid; grid-template-columns: repeat(7, minmax(0, 1fr)); }
  .weekday-row { align-items: center; border-bottom: 1px solid var(--color-border-default); color: var(--color-text-muted); text-align: center; }
  .month-grid { grid-template-rows: repeat(6, minmax(92px, 1fr)); }

  .calendar-day {
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 6px;
    min-width: 0;
    padding: 8px;
    border: 0;
    border-right: 1px solid var(--color-border-default);
    border-bottom: 1px solid var(--color-border-default);
    color: var(--color-text-primary);
    background: transparent;
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .calendar-day:nth-child(7n) { border-right: 0; }
  .calendar-day:nth-last-child(-n + 7) { border-bottom: 0; }
  .calendar-day:hover { background: var(--color-surface-2); }
  .calendar-day.outside { color: var(--color-text-muted); opacity: 0.58; }
  .calendar-day.selected { box-shadow: inset 0 0 0 1px var(--color-accent-primary); background: color-mix(in srgb, var(--color-accent-primary) 7%, transparent); }
  .calendar-day:focus-visible { z-index: 1; outline: 1px solid var(--color-accent-primary); outline-offset: -2px; }

  .day-number {
    display: inline-grid;
    width: 24px;
    height: 24px;
    place-items: center;
    border-radius: 999px;
    font: 700 11px ui-monospace, SFMono-Regular, Consolas, monospace;
  }
  .calendar-day.today .day-number { color: var(--color-app-bg); background: var(--color-accent-primary); }
  .day-items { display: grid; gap: 3px; min-width: 0; }
  .day-task { display: flex; gap: 4px; overflow: hidden; padding: 1px 0 1px 5px; border-left: 2px solid var(--color-border-strong); font-size: 10px; line-height: 1.35; text-overflow: ellipsis; white-space: nowrap; }
  .day-task.captured { border-color: var(--color-accent-primary); color: var(--color-text-primary); }
  .day-task.due { border-color: var(--color-status-warning); color: var(--color-status-warning); }
  .day-task.completed { border-color: var(--color-status-success); color: var(--color-status-success); }
  .completion-mark { flex: none; font: 800 9px ui-monospace, SFMono-Regular, Consolas, monospace; }
  .day-counts { flex-wrap: wrap; gap: 5px; margin-top: auto; font: 9px ui-monospace, SFMono-Regular, Consolas, monospace; }
  .captured-count { color: var(--color-accent-primary); }
  .due-count { color: var(--color-status-warning); }
  .completed-count { color: var(--color-status-success); }

  .agenda-panel { padding-left: 18px; border-left: 1px solid var(--color-border-default); overflow: auto; }
  .agenda-head { display: grid; gap: 4px; padding-bottom: 16px; border-bottom: 1px solid var(--color-border-default); }
  .agenda-head h2 { font-size: 17px; }
  .bucket { display: grid; gap: 8px; padding: 17px 0; border-bottom: 1px solid var(--color-border-default); }
  .bucket:last-child { border-bottom: 0; }
  .bucket-head { justify-content: space-between; gap: 10px; }
  .bucket-head h3 { color: var(--color-text-primary); }
  .bucket-head > span { min-width: 20px; color: var(--color-text-muted); font: 700 10px ui-monospace, SFMono-Regular, Consolas, monospace; text-align: right; }
  .empty { padding: 9px 0; color: var(--color-text-muted); font-size: 12px; }
  .task-list { display: grid; }
  .task-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; align-items: center; padding: 10px 0; }
  .task-row + .task-row { border-top: 1px solid var(--color-border-default); }
  .task-row.link-active { background: color-mix(in srgb, var(--color-accent-primary) 4%, transparent); }
  .task-row.done .task-text { color: var(--color-text-muted); text-decoration: line-through; }
  .task-copy { display: grid; gap: 4px; min-width: 0; padding: 0; border: 0; color: inherit; background: transparent; font: inherit; text-align: left; cursor: pointer; }
  .moment-badges { display: flex; flex-wrap: wrap; gap: 4px; }
  .moment-badge { color: var(--color-text-muted); font: 800 8px ui-monospace, SFMono-Regular, Consolas, monospace; letter-spacing: 0.06em; text-transform: uppercase; }
  .moment-badge.captured { color: var(--color-accent-primary); }
  .moment-badge.due { color: var(--color-status-warning); }
  .moment-badge.completed { color: var(--color-status-success); }
  .task-text { font-size: 12px; font-weight: 650; line-height: 1.35; }
  .lifecycle-rail { display: flex; align-items: center; min-width: 0; padding: 3px 0 2px; }
  .lifecycle-moment { display: flex; flex: none; align-items: center; gap: 4px; color: var(--color-text-muted); }
  .lifecycle-moment.captured { color: var(--color-accent-primary); }
  .lifecycle-moment.due { color: var(--color-status-warning); }
  .lifecycle-moment.completed { color: var(--color-status-success); }
  .lifecycle-moment.overdue { color: var(--color-status-error); }
  .lifecycle-node { display: grid; width: 13px; height: 13px; flex: none; place-items: center; border: 1px solid currentColor; border-radius: 999px; background: var(--color-app-bg); font: 800 8px ui-monospace, SFMono-Regular, Consolas, monospace; }
  .lifecycle-moment.past .lifecycle-node { background: color-mix(in srgb, currentColor 28%, var(--color-app-bg)); }
  .lifecycle-moment.selected .lifecycle-node { background: color-mix(in srgb, currentColor 45%, var(--color-app-bg)); box-shadow: 0 0 0 2px color-mix(in srgb, currentColor 20%, transparent); }
  .lifecycle-moment.future .lifecycle-node { opacity: 0.72; }
  .lifecycle-copy { display: grid; gap: 1px; color: inherit; font: 8px ui-monospace, SFMono-Regular, Consolas, monospace; line-height: 1.05; }
  .lifecycle-copy > span:first-child { font-weight: 800; letter-spacing: 0.04em; text-transform: uppercase; }
  .lifecycle-connector { height: 1px; min-width: 10px; max-width: 34px; flex: 1 1 24px; margin: 0 5px; background: var(--color-border-strong); }
  .task-meta { flex-wrap: wrap; gap: 5px; color: var(--color-text-muted); font: 9px ui-monospace, SFMono-Regular, Consolas, monospace; line-height: 1.3; }
  .owner { color: var(--color-accent-primary); font-weight: 800; }
  .task-relations { display: inline-flex; align-items: center; gap: 3px; color: var(--color-accent-primary); }
  .completion-unknown { color: var(--color-status-warning); }
  .task-action { gap: 4px; min-height: 27px; padding: 0 8px; border: 1px solid var(--color-border-default); border-radius: 6px; color: var(--color-text-muted); background: var(--color-surface-input); font: 750 10px ui-monospace, SFMono-Regular, Consolas, monospace; cursor: pointer; }
  .task-action:hover { border-color: var(--color-accent-primary); color: var(--color-text-primary); }
  .task-action.complete:hover { border-color: var(--color-status-success); color: var(--color-status-success); }
  .task-action:disabled { opacity: 0.5; cursor: default; }
  .task-copy:focus-visible, .task-action:focus-visible { outline: 1px solid var(--color-accent-primary); outline-offset: 2px; }
  .day-task { transition: opacity 120ms ease, background-color 120ms ease, border-left-width 120ms ease; }
  .day-task.echo-highlighted { border-left-width: 3px; background: color-mix(in srgb, var(--color-accent-primary) 9%, transparent); opacity: 1; }
  .day-task.echo-muted { opacity: 0.22; }

  @media (max-width: 1050px) {
    .calendar-layout {
      flex: none;
      grid-template-columns: 1fr;
      grid-template-rows: auto auto;
      gap: 28px;
      min-height: 0;
    }
    .month-panel { grid-template-rows: 34px auto; }
    .month-grid { grid-template-rows: repeat(6, minmax(68px, auto)); }
    .agenda-panel {
      max-height: none;
      padding: 18px 0 0;
      border-top: 1px solid var(--color-border-default);
      border-left: 0;
      overflow: visible;
    }
  }

  @media (max-width: 720px) {
    .calendar-view { padding: 18px 14px 28px; }
    .calendar-layout { min-height: 0; }
    .month-panel { overflow-x: auto; }
    .weekday-row, .month-grid { min-width: 620px; }
  }
</style>
