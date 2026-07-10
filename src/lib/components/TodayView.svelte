<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import ChevronLeft from "@lucide/svelte/icons/chevron-left";
  import ChevronRight from "@lucide/svelte/icons/chevron-right";
  import Check from "@lucide/svelte/icons/check";
  import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
  import { dueTone, formatActionDue } from "$lib/action-groups";
  import {
    buildCalendarMonth,
    buildCalendarTasks,
    formatMonthHeading,
    formatSelectedDate,
    localDateKey,
    tasksForCalendarDate,
    unplacedDoneTasks,
    unscheduledOpenTasks,
    type CalendarDay,
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
  const selectedTasks = $derived(tasksForCalendarDate(tasks, selectedDate, selectedIsToday));
  const unscheduled = $derived(unscheduledOpenTasks(tasks));
  const unplacedDone = $derived(unplacedDoneTasks(tasks));
  const openCount = $derived(tasks.filter((task) => task.status !== "done").length);
  const monthDoneCount = $derived(
    calendarDays
      .filter((day) => day.isCurrentMonth)
      .reduce((total, day) => total + day.doneCount, 0),
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
    selectedDate = new Date(day.date.getFullYear(), day.date.getMonth(), day.date.getDate());
    if (!day.isCurrentMonth) {
      visibleMonth = new Date(day.date.getFullYear(), day.date.getMonth(), 1);
    }
  }

  function dayLabel(day: CalendarDay): string {
    const date = formatSelectedDate(day.date);
    const open = `${day.openCount} open`;
    const done = `${day.doneCount} done`;
    return `Select ${date}, ${open}, ${done}`;
  }

  function completedTime(task: CalendarTask): string | null {
    if (!task.completedAt) return null;
    const date = new Date(task.completedAt);
    if (Number.isNaN(date.getTime())) return null;
    return date.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  }
</script>

{#snippet taskRow(task: CalendarTask)}
  <article class:done={task.status === "done"} class="task-row">
    <button
      class="task-copy"
      type="button"
      aria-label={`Open note: ${task.noteTitle}`}
      onclick={() => dispatch("openNote", task.noteId)}
    >
      <span class="task-text">{task.text}</span>
      <span class="task-meta">
        {#if task.owner}<span class="owner">@{task.owner}</span>{/if}
        {#if task.status === "done"}
          {#if completedTime(task)}<span>Completed {completedTime(task)}</span>{/if}
        {:else}
          {@const due = formatActionDue(task.dueDate, now)}
          {#if due}<span class={`tone-${dueTone(task.dueDate, now)}`}>{due}</span>{/if}
          <span>{task.status === "suggested" ? "Needs review" : "Open"}</span>
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
          {@const dayTasks = tasksForCalendarDate(tasks, day.date)}
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
              {#each dayTasks.open.slice(0, 2) as task}
                <span class="day-task open">{task.text}</span>
              {/each}
              {#each dayTasks.done.slice(0, Math.max(0, 2 - dayTasks.open.length)) as task}
                <span class="day-task done">{task.text}</span>
              {/each}
            </span>
            {#if day.openCount > 0 || day.doneCount > 0}
              <span class="day-counts" aria-hidden="true">
                {#if day.openCount > 0}<span class="open-count">{day.openCount} open</span>{/if}
                {#if day.doneCount > 0}<span class="done-count">{day.doneCount} done</span>{/if}
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

      <section class="bucket" aria-label="Open tasks">
        <div class="bucket-head">
          <h3>{selectedIsToday ? "Open & overdue" : "Open"}</h3>
          <span>{selectedTasks.open.length}</span>
        </div>
        {#if selectedTasks.open.length === 0}
          <p class="empty">No open tasks for this day.</p>
        {:else}
          <div class="task-list">
            {#each selectedTasks.open as task}{@render taskRow(task)}{/each}
          </div>
        {/if}
      </section>

      <section class="bucket done-bucket" aria-label="Done tasks">
        <div class="bucket-head">
          <h3>Done</h3>
          <span>{selectedTasks.done.length}</span>
        </div>
        {#if selectedTasks.done.length === 0}
          <p class="empty">Nothing completed on this day.</p>
        {:else}
          <div class="task-list">
            {#each selectedTasks.done as task}{@render taskRow(task)}{/each}
          </div>
        {/if}
      </section>

      {#if selectedIsToday && unscheduled.length > 0}
        <section class="bucket" aria-label="Unscheduled open tasks">
          <div class="bucket-head">
            <h3>Unscheduled</h3>
            <span>{unscheduled.length}</span>
          </div>
          <div class="task-list">
            {#each unscheduled as task}{@render taskRow(task)}{/each}
          </div>
        </section>
      {/if}

      {#if selectedIsToday && unplacedDone.length > 0}
        <section class="bucket done-bucket" aria-label="Done tasks without completion dates">
          <div class="bucket-head">
            <h3>Done · date unavailable</h3>
            <span>{unplacedDone.length}</span>
          </div>
          <div class="task-list">
            {#each unplacedDone as task}{@render taskRow(task)}{/each}
          </div>
        </section>
      {/if}
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
    border: 1px solid var(--color-border-default);
    border-radius: 10px;
    background: var(--color-surface-1);
    overflow: hidden;
  }

  .month-panel { display: grid; grid-template-rows: 34px 1fr; }
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
  .day-task { overflow: hidden; padding: 2px 5px; border-radius: 4px; font-size: 10px; line-height: 1.35; text-overflow: ellipsis; white-space: nowrap; }
  .day-task.open { color: var(--color-text-primary); background: var(--color-surface-2); }
  .day-task.done { color: var(--color-status-success); background: color-mix(in srgb, var(--color-status-success) 10%, transparent); text-decoration: line-through; }
  .day-counts { flex-wrap: wrap; gap: 5px; margin-top: auto; font: 9px ui-monospace, SFMono-Regular, Consolas, monospace; }
  .open-count { color: var(--color-accent-primary); }
  .done-count { color: var(--color-status-success); }

  .agenda-panel { padding: 18px; overflow: auto; }
  .agenda-head { display: grid; gap: 4px; padding-bottom: 16px; border-bottom: 1px solid var(--color-border-default); }
  .agenda-head h2 { font-size: 17px; }
  .bucket { display: grid; gap: 8px; padding: 17px 0; border-bottom: 1px solid var(--color-border-default); }
  .bucket:last-child { border-bottom: 0; }
  .bucket-head { justify-content: space-between; gap: 10px; }
  .bucket-head h3 { color: var(--color-text-primary); }
  .bucket-head > span { min-width: 20px; padding: 1px 6px; border-radius: 999px; color: var(--color-text-muted); background: var(--color-surface-2); font: 700 10px ui-monospace, SFMono-Regular, Consolas, monospace; text-align: center; }
  .done-bucket .bucket-head h3 { color: var(--color-status-success); }
  .empty { padding: 9px 0; color: var(--color-text-muted); font-size: 12px; }
  .task-list { display: grid; gap: 3px; }
  .task-row { display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 8px; align-items: center; padding: 8px; border-radius: 7px; }
  .task-row:hover { background: var(--color-surface-2); }
  .task-row.done .task-text { color: var(--color-text-muted); text-decoration: line-through; }
  .task-copy { display: grid; gap: 4px; min-width: 0; padding: 0; border: 0; color: inherit; background: transparent; font: inherit; text-align: left; cursor: pointer; }
  .task-text { font-size: 12px; font-weight: 650; line-height: 1.35; }
  .task-meta { flex-wrap: wrap; gap: 5px; color: var(--color-text-muted); font: 9px ui-monospace, SFMono-Regular, Consolas, monospace; line-height: 1.3; }
  .owner { color: var(--color-accent-primary); font-weight: 800; }
  .tone-error { color: var(--color-status-error); }
  .tone-warning { color: var(--color-status-warning); }
  .tone-muted { color: var(--color-text-muted); }
  .task-action { gap: 4px; min-height: 27px; padding: 0 8px; border: 1px solid var(--color-border-default); border-radius: 6px; color: var(--color-text-muted); background: var(--color-surface-input); font: 750 10px ui-monospace, SFMono-Regular, Consolas, monospace; cursor: pointer; }
  .task-action:hover { border-color: var(--color-accent-primary); color: var(--color-text-primary); }
  .task-action.complete:hover { border-color: var(--color-status-success); color: var(--color-status-success); }
  .task-action:disabled { opacity: 0.5; cursor: default; }
  .task-copy:focus-visible, .task-action:focus-visible { outline: 1px solid var(--color-accent-primary); outline-offset: 2px; }

  @media (max-width: 1050px) {
    .calendar-layout { grid-template-columns: 1fr; }
    .agenda-panel { max-height: none; }
  }

  @media (max-width: 720px) {
    .calendar-view { padding: 18px 14px 28px; }
    .calendar-layout { min-height: 0; }
    .month-panel { overflow-x: auto; }
    .weekday-row, .month-grid { min-width: 620px; }
  }
</style>
