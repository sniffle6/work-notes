/* global React, Icon, Sidebar, Kbd, IconBtn, StatusDot, SAMPLE_NOTES, formatWhen, formatDue, dueTone, NOW */

// ─── Today view ─────────────────────────────────────────────────────────

function TodayView() {
  const today = new Date(NOW).toLocaleDateString(undefined, { weekday: "long", month: "long", day: "numeric" });
  // Notes captured today
  const recent = SAMPLE_NOTES.filter((n) => (NOW - n.when) < 12 * 60 * 60 * 1000);
  // Actions due today (or overdue)
  const dueActions = [];
  SAMPLE_NOTES.forEach((n) => {
    n.actions.forEach((a) => {
      if (a.status !== "suggested" || !a.due) return;
      const diff = Math.round((a.due - NOW) / (24 * 60 * 60 * 1000));
      if (diff <= 0) dueActions.push({ ...a, note: n });
    });
  });

  return (
    <div className="app">
      <Sidebar active="today" inboxCount={SAMPLE_NOTES.length} />
      <section className="today-pane">
        <header className="today-head">
          <div className="today-eyebrow">Today</div>
          <h1>{today}</h1>
          <p>3 actions due · 2 captured · 1 needs review</p>
        </header>

        <section className="today-section">
          <div className="today-section-head">
            <h2>Due today</h2>
            <span className="today-section-count">{dueActions.length}</span>
          </div>
          <ul className="today-actions">
            {dueActions.map((a) => {
              const tone = dueTone(a.due);
              return (
                <li key={a.id} className="today-action">
                  <button className="al-check" aria-label="Accept"><Icon.Check size={11} /></button>
                  <div className="al-body">
                    <div className="al-text">{a.text}</div>
                    <div className="al-meta">
                      {a.owner && <span className="al-owner">@{a.owner}</span>}
                      <span className={`al-due tone-${tone}`}>{formatDue(a.due)}</span>
                      <span className="al-sep">·</span>
                      <span className="al-from">from “{a.note.title.length > 42 ? a.note.title.slice(0, 42) + "…" : a.note.title}”</span>
                    </div>
                  </div>
                </li>
              );
            })}
          </ul>
        </section>

        <section className="today-section">
          <div className="today-section-head">
            <h2>Captured today</h2>
            <span className="today-section-count">{recent.length}</span>
          </div>
          <ul className="today-notes">
            {recent.map((n) => (
              <li key={n.id} className="today-note">
                <StatusDot status={n.parseStatus} review={n.reviewStatus} />
                <div className="today-note-body">
                  <div className="today-note-title">{n.titleIsGenerated && n.summary ? n.summary : n.title}</div>
                  <div className="today-note-meta">
                    <span>{formatWhen(n.when)}</span>
                    <span className="al-sep">·</span>
                    <span>{n.tags.slice(0, 2).map(t => "#" + t.name).join(" ")}</span>
                  </div>
                </div>
                {n.actions.filter(a => a.status === "suggested").length > 0 && (
                  <span className="today-note-actions">
                    <Icon.Check size={11} /> {n.actions.filter(a => a.status === "suggested").length}
                  </span>
                )}
              </li>
            ))}
          </ul>
        </section>

        <section className="today-section">
          <div className="today-section-head">
            <h2>This week</h2>
          </div>
          <div className="today-week">
            {["Mon", "Tue", "Wed", "Thu", "Fri"].map((d, i) => {
              const dotCount = [1, 2, 3, 2, 1][i];
              const isToday = d === "Wed";
              return (
                <div key={d} className={`today-day ${isToday ? "is-today" : ""}`}>
                  <div className="today-day-label">{d}</div>
                  <div className="today-day-dots">
                    {Array.from({ length: dotCount }).map((_, j) => <span key={j} className="today-day-dot" />)}
                  </div>
                  <div className="today-day-count">{dotCount}</div>
                </div>
              );
            })}
          </div>
        </section>
      </section>
    </div>
  );
}

// ─── People view ────────────────────────────────────────────────────────

function PeopleView({ initialSelected = "Maria" }) {
  const [selected, setSelected] = React.useState(initialSelected);
  const PEOPLE = ["Maria", "Jin", "Kavi", "Anita", "Sam", "Priya", "Devon"];
  // Build person → notes / actions
  const stats = {};
  PEOPLE.forEach((p) => (stats[p] = { notes: [], actions: 0, last: null }));
  SAMPLE_NOTES.forEach((n) => {
    n.tags.forEach((t) => {
      if (PEOPLE.includes(t.name)) {
        stats[t.name].notes.push(n);
        if (!stats[t.name].last || n.when > stats[t.name].last) stats[t.name].last = n.when;
      }
    });
    n.actions.forEach((a) => {
      if (a.status === "suggested" && a.owner && PEOPLE.includes(a.owner.charAt(0).toUpperCase() + a.owner.slice(1))) {
        stats[a.owner.charAt(0).toUpperCase() + a.owner.slice(1)].actions += 1;
      }
    });
  });

  const sel = stats[selected];
  const youOwe = SAMPLE_NOTES.flatMap((n) => n.actions.map((a) => ({ ...a, note: n })))
    .filter((a) => a.status === "suggested" && a.owner === "me"
                   && a.note.tags.some((t) => t.name === selected));
  const theyOwe = SAMPLE_NOTES.flatMap((n) => n.actions.map((a) => ({ ...a, note: n })))
    .filter((a) => a.status === "suggested" && a.owner && a.owner.toLowerCase() === selected.toLowerCase());

  const avatarHue = (name) => (name.charCodeAt(0) * 37) % 360;

  return (
    <div className="app">
      <Sidebar active="people" inboxCount={SAMPLE_NOTES.length} />
      <section className="people-list">
        <header className="il-head">
          <div className="il-head-row">
            <h1 className="il-title">People</h1>
            <IconBtn icon={Icon.More} label="More" />
          </div>
          <div className="il-search">
            <Icon.Search size={14} />
            <input placeholder="Find a person" />
          </div>
        </header>
        <div className="il-scroll">
          {PEOPLE.map((p) => (
            <button key={p} className={`person-row ${selected === p ? "selected" : ""}`} onClick={() => setSelected(p)}>
              <div className="person-avatar" style={{ background: `oklch(0.4 0.08 ${avatarHue(p)})`, color: `oklch(0.94 0.04 ${avatarHue(p)})` }}>
                {p.charAt(0)}
              </div>
              <div className="person-body">
                <div className="person-name">{p}</div>
                <div className="person-meta">
                  {stats[p].notes.length} note{stats[p].notes.length !== 1 ? "s" : ""}
                  {stats[p].last && <> · last {formatWhen(stats[p].last)}</>}
                </div>
              </div>
              {stats[p].actions > 0 && <span className="person-pending">{stats[p].actions}</span>}
            </button>
          ))}
        </div>
      </section>

      <section className="person-detail">
        <header className="nd-head">
          <div className="nd-head-meta">
            <div className="person-avatar lg" style={{ background: `oklch(0.4 0.08 ${avatarHue(selected)})`, color: `oklch(0.94 0.04 ${avatarHue(selected)})` }}>
              {selected.charAt(0)}
            </div>
            <div>
              <div className="person-detail-name">{selected}</div>
              <div className="person-detail-sub">{sel.notes.length} notes · last interaction {sel.last && formatWhen(sel.last)}</div>
            </div>
          </div>
          <div className="nd-head-right">
            <button className="nd-pill-btn"><Icon.Plus size={12} /> New note about {selected}</button>
          </div>
        </header>

        <div className="nd-scroll">
          <div className="person-cols">
            <div className="person-col">
              <div className="nd-section-head">You owe {selected} <span className="nd-section-count">{youOwe.length}</span></div>
              {youOwe.length === 0 && <div className="person-empty">Nothing on your plate.</div>}
              <ul className="nd-actions">
                {youOwe.map((a) => (
                  <li key={a.id} className="nd-action">
                    <button className="nd-action-check"><Icon.Check size={11} /></button>
                    <div className="nd-action-body">
                      <div className="nd-action-text">{a.text}</div>
                      <div className="nd-action-meta">
                        {a.due && <span className={`al-due tone-${dueTone(a.due)}`}>{formatDue(a.due)}</span>}
                      </div>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
            <div className="person-col">
              <div className="nd-section-head">{selected} owes you <span className="nd-section-count">{theyOwe.length}</span></div>
              {theyOwe.length === 0 && <div className="person-empty">Nothing outstanding.</div>}
              <ul className="nd-actions">
                {theyOwe.map((a) => (
                  <li key={a.id} className="nd-action">
                    <span className="nd-action-bullet" />
                    <div className="nd-action-body">
                      <div className="nd-action-text">{a.text}</div>
                      <div className="nd-action-meta">
                        {a.due && <span className={`al-due tone-${dueTone(a.due)}`}>{formatDue(a.due)}</span>}
                      </div>
                    </div>
                  </li>
                ))}
              </ul>
            </div>
          </div>

          <div className="nd-section" style={{ marginTop: 28 }}>
            <div className="nd-section-head">Recent notes <span className="nd-section-count">{sel.notes.length}</span></div>
            <ul className="person-notes">
              {sel.notes.map((n) => (
                <li key={n.id} className="person-note">
                  <StatusDot status={n.parseStatus} review={n.reviewStatus} />
                  <div className="person-note-body">
                    <div className="person-note-title">{n.titleIsGenerated && n.summary ? n.summary : n.title}</div>
                    <div className="person-note-meta">{formatWhen(n.when)}</div>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        </div>
      </section>
    </div>
  );
}

// ─── Keyboard cheatsheet overlay ────────────────────────────────────────

function KeyboardOverlay() {
  const groups = [
    {
      title: "Capture",
      rows: [
        { label: "Open quick capture (global)", keys: ["Ctrl", "⇧", "Space"] },
        { label: "Save note", keys: ["↵"] },
        { label: "New line", keys: ["⇧", "↵"] },
        { label: "Cancel capture", keys: ["Esc"] },
      ],
    },
    {
      title: "Navigate",
      rows: [
        { label: "Inbox", keys: ["1"] },
        { label: "Today", keys: ["2"] },
        { label: "Actions", keys: ["3"] },
        { label: "Move down / up note list", keys: ["j", "k"] },
        { label: "Search", keys: ["/"] },
        { label: "Toggle Notes / Actions", keys: ["⇥"] },
      ],
    },
    {
      title: "Note",
      rows: [
        { label: "Mark reviewed", keys: ["R"] },
        { label: "Archive", keys: ["E"] },
        { label: "Delete", keys: ["⌫", "⌫"] },
        { label: "Reparse with feedback", keys: ["⇧", "R"] },
        { label: "Toggle raw / cleaned", keys: ["V"] },
      ],
    },
    {
      title: "Actions",
      rows: [
        { label: "Accept action", keys: ["A"] },
        { label: "Dismiss action", keys: ["D"] },
        { label: "Move action to top", keys: ["⌘", "↑"] },
      ],
    },
  ];

  return (
    <div className="app">
      <div className="kb-backdrop" />
      <div className="kb-sheet">
        <header className="kb-head">
          <div>
            <h1>Keyboard shortcuts</h1>
            <p>Press <Kbd>?</Kbd> anywhere to bring this up. Press <Kbd>Esc</Kbd> to dismiss.</p>
          </div>
          <button className="ss-close"><Icon.X size={14} /></button>
        </header>
        <div className="kb-grid">
          {groups.map((g) => (
            <section key={g.title} className="kb-group">
              <div className="kb-group-title">{g.title}</div>
              <ul>
                {g.rows.map((r) => (
                  <li key={r.label}>
                    <span>{r.label}</span>
                    <span className="kb-keys">
                      {r.keys.map((k, i) => <Kbd key={i}>{k}</Kbd>)}
                    </span>
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
        <footer className="kb-foot">
          <span className="kb-tip">
            <Icon.Sparkle size={11} />
            All shortcuts are rebindable in <span className="mono">Settings → Keyboard</span>.
          </span>
        </footer>
      </div>
    </div>
  );
}

window.TodayView = TodayView;
window.PeopleView = PeopleView;
window.KeyboardOverlay = KeyboardOverlay;
