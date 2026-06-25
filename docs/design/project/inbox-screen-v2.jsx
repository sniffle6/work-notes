/* global React, Icon, SAMPLE_NOTES, formatWhen, formatClock, formatDue, dueTone, NOW */

// ─── Atoms ────────────────────────────────────────────────────────────────

function StatusDot({ status, review, size = 7 }) {
  let color, label, pulse = false;
  if (status === "failed") { color = "var(--err)"; label = "Parse failed"; }
  else if (status === "parsing") { color = "var(--info)"; label = "Parsing"; pulse = true; }
  else if (status === "queued") { color = "var(--text-subtle)"; label = "Queued"; }
  else if (review === "needs_review") { color = "var(--warn)"; label = "Needs review"; }
  else if (review === "reviewed") { color = "transparent"; label = "Reviewed"; }
  else { color = "var(--text-subtle)"; label = "Captured"; }
  if (color === "transparent") {
    return <span className="status-dot reviewed" title={label}
                 style={{ width: size, height: size, borderColor: "var(--ok)" }} />;
  }
  return (
    <span className="status-dot" title={label}
          style={{ background: color, width: size, height: size,
                   animation: pulse ? "wn-pulse 1.6s ease-in-out infinite" : "none" }} />
  );
}

function TagChip({ name, on, onClick }) {
  return (
    <button className={`tag2 ${on ? "on" : ""}`} onClick={onClick} type="button">
      {name}
    </button>
  );
}

function Kbd({ children }) {
  return <kbd className="kbd">{children}</kbd>;
}

function IconBtn({ icon: I, label, onClick, tone = "ghost", size = 28 }) {
  return (
    <button className={`icon-btn ${tone}`} aria-label={label} title={label} onClick={onClick}
            style={{ width: size, height: size }}>
      <I size={15} />
    </button>
  );
}

// ─── Sidebar (v2: same but with kbd hint slot) ─────────────────────────

function Sidebar({ active = "inbox", inboxCount }) {
  const nav = [
    { id: "inbox", label: "Inbox", icon: Icon.Inbox, count: inboxCount, kbd: "1" },
    { id: "today", label: "Today", icon: Icon.Today, count: 3, kbd: "2" },
    { id: "actions", label: "Actions", icon: Icon.Check, count: 6, kbd: "3" },
    { id: "tags", label: "Tags", icon: Icon.Tag, kbd: "4" },
    { id: "people", label: "People", icon: Icon.People, kbd: "5" },
    { id: "archive", label: "Archive", icon: Icon.Archive, kbd: "6" },
  ];
  const tags = ["Q3-dashboard", "integrations", "Maria", "Jin", "roadmap"];

  return (
    <aside className="sidebar">
      <div className="sb-brand">
        <div className="sb-mark">WN</div>
        <div className="sb-brand-meta">
          <div className="sb-workspace">Local workspace</div>
          <div className="sb-app">Work Notes</div>
        </div>
      </div>

      <button className="capture-btn">
        <span className="capture-btn-icon"><Icon.Plus size={14} /></span>
        <span className="capture-btn-label">New note</span>
        <span className="capture-btn-kbd"><Kbd>⌃</Kbd><Kbd>⇧</Kbd><Kbd>␣</Kbd></span>
      </button>

      <nav className="sb-nav" aria-label="Workspace">
        {nav.map((n) => {
          const Ic = n.icon;
          return (
            <button key={n.id} className={`sb-nav-item ${active === n.id ? "active" : ""}`}>
              <Ic size={15} />
              <span className="sb-nav-label">{n.label}</span>
              {n.count ? <span className="sb-nav-count">{n.count}</span> :
                <span className="sb-nav-kbd"><Kbd>{n.kbd}</Kbd></span>}
            </button>
          );
        })}
      </nav>

      <div className="sb-section-label">Tags</div>
      <div className="sb-tags">
        {tags.map((t) => <TagChip key={t} name={t} />)}
        <button className="sb-tag-more">+ 12 more</button>
      </div>

      <div className="sb-spacer" />

      <div className="sb-footer">
        <div className="sb-parser">
          <span className="parser-dot" />
          <div className="parser-meta">
            <div className="parser-line">Parser ready</div>
            <div className="parser-sub">codex.cmd · 1 in queue</div>
          </div>
          <button className="parser-cog" aria-label="Settings"><Icon.Settings size={14} /></button>
        </div>
      </div>
    </aside>
  );
}

// ─── Inbox list (v2: mode toggle, calmer cards) ─────────────────────────

function InboxList({ notes, selectedId, onSelect, filter, setFilter, query, setQuery, mode = "notes", setMode }) {
  const filters = [
    { id: "all", label: "All", count: notes.length },
    { id: "review", label: "Needs review", count: notes.filter(n => n.reviewStatus === "needs_review").length },
    { id: "failed", label: "Failed", count: notes.filter(n => n.parseStatus === "failed").length },
  ];
  const filtered = notes.filter((n) => {
    if (filter === "review") return n.reviewStatus === "needs_review";
    if (filter === "failed") return n.parseStatus === "failed";
    return true;
  }).filter((n) => !query || n.title.toLowerCase().includes(query.toLowerCase()) || n.raw.toLowerCase().includes(query.toLowerCase()));

  return (
    <section className="inbox-list">
      <header className="il-head">
        <div className="il-head-row">
          <div className="il-mode">
            <button className={`il-mode-btn ${mode === "notes" ? "active" : ""}`} onClick={() => setMode && setMode("notes")}>
              Notes
            </button>
            <button className={`il-mode-btn ${mode === "actions" ? "active" : ""}`} onClick={() => setMode && setMode("actions")}>
              Actions
              <span className="il-mode-count">6</span>
            </button>
          </div>
          <IconBtn icon={Icon.More} label="More" />
        </div>
        <div className="il-search">
          <Icon.Search size={14} />
          <input value={query} onChange={(e) => setQuery(e.target.value)} placeholder="Search notes, people, tags" />
          <Kbd>/</Kbd>
        </div>
        <div className="il-filters">
          {filters.map((f) => (
            <button key={f.id} className={`il-filter ${filter === f.id ? "active" : ""}`} onClick={() => setFilter(f.id)}>
              {f.label}
              {f.count > 0 && <span className="il-filter-count">{f.count}</span>}
            </button>
          ))}
        </div>
      </header>

      <div className="il-scroll">
        {filtered.map((n) => {
          const head = n.titleIsGenerated && n.summary ? n.summary : n.title;
          const sub = n.titleIsGenerated && n.summary ? n.cleaned : (n.cleaned ?? n.raw);
          const pendingActions = n.actions.filter(a => a.status === "suggested").length;
          return (
            <button key={n.id}
              className={`il-card ${selectedId === n.id ? "selected" : ""}`}
              onClick={() => onSelect(n.id)}>
              <div className="il-card-top">
                <StatusDot status={n.parseStatus} review={n.reviewStatus} />
                {n.pinned && <Icon.Pin size={11} />}
                <span className="il-card-when">{formatWhen(n.when)}</span>
              </div>
              <div className="il-card-title">{head}</div>
              {sub && sub !== head && <div className="il-card-preview">{sub}</div>}
              {(n.tags.length > 0 || pendingActions > 0) && (
                <div className="il-card-meta">
                  {n.tags.slice(0, 3).map((t) => <span key={t.name} className="tag2 mini">{t.name}</span>)}
                  {n.tags.length > 3 && <span className="il-card-more">+{n.tags.length - 3}</span>}
                  {pendingActions > 0 && (
                    <span className="il-card-actions">
                      <Icon.Check size={11} /> {pendingActions}
                    </span>
                  )}
                </div>
              )}
            </button>
          );
        })}
      </div>
    </section>
  );
}

// ─── Actions list (v2: actions-first mode) ──────────────────────────────

function ActionsList({ notes, selectedId, onSelect, mode, setMode }) {
  // Flatten all suggested actions across notes.
  const all = [];
  notes.forEach((n) => {
    n.actions.forEach((a) => {
      if (a.status === "suggested") all.push({ ...a, note: n });
    });
  });
  // Group: overdue / today / this week / later
  const groups = { Overdue: [], Today: [], "This week": [], Later: [], "No date": [] };
  all.forEach((a) => {
    if (!a.due) groups["No date"].push(a);
    else {
      const diffDay = Math.round((a.due - NOW) / (24 * 60 * 60 * 1000));
      if (diffDay < 0) groups.Overdue.push(a);
      else if (diffDay === 0) groups.Today.push(a);
      else if (diffDay <= 6) groups["This week"].push(a);
      else groups.Later.push(a);
    }
  });

  return (
    <section className="inbox-list">
      <header className="il-head">
        <div className="il-head-row">
          <div className="il-mode">
            <button className={`il-mode-btn ${mode === "notes" ? "active" : ""}`} onClick={() => setMode("notes")}>
              Notes
            </button>
            <button className={`il-mode-btn ${mode === "actions" ? "active" : ""}`} onClick={() => setMode("actions")}>
              Actions
              <span className="il-mode-count">{all.length}</span>
            </button>
          </div>
          <IconBtn icon={Icon.More} label="More" />
        </div>
        <div className="il-search">
          <Icon.Search size={14} />
          <input placeholder="Search actions, owners" />
          <Kbd>/</Kbd>
        </div>
      </header>

      <div className="il-scroll actions-scroll">
        {Object.entries(groups).filter(([, arr]) => arr.length > 0).map(([bucket, arr]) => (
          <div key={bucket} className="al-group">
            <div className="al-group-head">
              <span>{bucket}</span>
              <span className="al-group-count">{arr.length}</span>
            </div>
            {arr.map((a) => {
              const tone = dueTone(a.due);
              const due = formatDue(a.due);
              return (
                <div key={a.id} className={`al-row ${selectedId === a.note.id ? "selected" : ""}`}
                     onClick={() => onSelect(a.note.id)}>
                  <button className="al-check" aria-label="Accept" onClick={(e) => e.stopPropagation()}>
                    <Icon.Check size={11} />
                  </button>
                  <div className="al-body">
                    <div className="al-text">{a.text}</div>
                    <div className="al-meta">
                      {a.owner && <span className="al-owner">@{a.owner}</span>}
                      {due && <span className={`al-due tone-${tone}`}>{due}</span>}
                      <span className="al-sep">·</span>
                      <span className="al-from">from “{a.note.title.length > 38 ? a.note.title.slice(0, 38) + "…" : a.note.title}”</span>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </section>
  );
}

// ─── Note detail (v2: lead with summary, lighter actions, prominent review) ──

function NoteDetail({ note, showRaw, setShowRaw, reparseOpen, setReparseOpen }) {
  if (!note) {
    return (
      <section className="note-detail empty">
        <div className="nd-empty">
          <div className="nd-empty-mark"><Icon.Inbox size={28} /></div>
          <div className="nd-empty-title">Select a note</div>
          <div className="nd-empty-sub">Or press <Kbd>⌃</Kbd>+<Kbd>⇧</Kbd>+<Kbd>␣</Kbd> to capture a new one.</div>
        </div>
      </section>
    );
  }

  // If parser generated a "summary-style" title, use it as the headline and skip the summary callout.
  const headline = note.titleIsGenerated && note.summary ? note.summary : note.title;
  const showSummaryBlock = !note.titleIsGenerated && note.summary;
  const pending = note.actions.filter(a => a.status === "suggested");
  const done = note.actions.filter(a => a.status === "done" || a.status === "accepted");

  return (
    <section className="note-detail">
      <header className="nd-head">
        <div className="nd-head-meta">
          <StatusDot status={note.parseStatus} review={note.reviewStatus} />
          <span>{formatWhen(note.when)}</span>
          <span className="nd-dot-sep">·</span>
          <span>{formatClock(note.when)}</span>
          {note.reviewStatus === "reviewed" && <>
            <span className="nd-dot-sep">·</span>
            <span className="nd-reviewed-tag"><Icon.Check size={11} /> reviewed</span>
          </>}
        </div>
        <div className="nd-head-right">
          {note.reviewStatus !== "reviewed" && (
            <button className="nd-review-btn">
              <Icon.Check size={13} />
              Mark reviewed
              <Kbd>R</Kbd>
            </button>
          )}
          <IconBtn icon={Icon.Refresh} label="Reparse" />
          <IconBtn icon={Icon.Archive2} label="Archive" />
          <IconBtn icon={Icon.More} label="More" />
        </div>
      </header>

      <div className="nd-scroll">
        <h1 className="nd-title">{headline}</h1>

        {note.parseStatus === "failed" && (
          <div className="nd-banner err">
            <Icon.Alert size={14} />
            <div className="nd-banner-body">
              <div className="nd-banner-title">Parser failed</div>
              <div className="nd-banner-sub">{note.parseError}</div>
            </div>
            <button className="nd-banner-btn">Retry</button>
          </div>
        )}
        {note.parseStatus === "parsing" && (
          <div className="nd-banner info">
            <Icon.Loader size={14} className="spin" />
            <div className="nd-banner-body">
              <div className="nd-banner-title">Parsing in background</div>
              <div className="nd-banner-sub">Codex is cleaning and tagging. Raw note is saved.</div>
            </div>
          </div>
        )}

        {showSummaryBlock && (
          <div className="nd-summary">
            <div className="nd-eyebrow"><Icon.Sparkle size={11} /> Summary</div>
            <p>{note.summary}</p>
          </div>
        )}

        <div className="nd-body-head">
          <div className="nd-tabs">
            <button className={`nd-tab ${!showRaw ? "active" : ""}`} onClick={() => setShowRaw(false)}>Cleaned</button>
            <button className={`nd-tab ${showRaw ? "active" : ""}`} onClick={() => setShowRaw(true)}>Raw</button>
          </div>
          <button className="nd-feedback-link" onClick={() => setReparseOpen(!reparseOpen)}>
            <Icon.Sparkle size={11} />
            Reparse with feedback {reparseOpen ? "▾" : "▸"}
          </button>
        </div>
        <div className="nd-body">
          {showRaw
            ? <pre className="nd-raw">{note.raw}</pre>
            : <p className="nd-cleaned">{note.cleaned ?? <span className="nd-pending">Waiting for parser…</span>}</p>}
        </div>
        {reparseOpen && (
          <div className="nd-reparse">
            <textarea placeholder="What did the parser get wrong? e.g. &quot;Anita is the owner, not me&quot;" />
            <div className="nd-reparse-row">
              <span className="nd-reparse-hint">
                <Icon.Sparkle size={11} /> Sent to <span className="mono">codex exec</span> with the raw note.
              </span>
              <button className="nd-reparse-btn">Reparse</button>
            </div>
          </div>
        )}

        {note.tags.length > 0 && (
          <div className="nd-section">
            <div className="nd-section-head">Tags</div>
            <div className="nd-tags">
              {note.tags.map((t) => <span key={t.name} className="tag2">{t.name}</span>)}
              <button className="nd-tag-add">+ add</button>
            </div>
          </div>
        )}

        {pending.length > 0 && (
          <div className="nd-section">
            <div className="nd-section-head">
              Actions
              <span className="nd-section-count">{pending.length} suggested</span>
            </div>
            <ul className="nd-actions">
              {pending.map((a) => {
                const tone = dueTone(a.due);
                const due = formatDue(a.due);
                return (
                  <li key={a.id} className="nd-action">
                    <button className="nd-action-check" aria-label="Accept"><Icon.Check size={11} /></button>
                    <div className="nd-action-body">
                      <div className="nd-action-text">{a.text}</div>
                      <div className="nd-action-meta">
                        {a.owner && <span className="al-owner">@{a.owner}</span>}
                        {due && <span className={`al-due tone-${tone}`}>{due}</span>}
                      </div>
                    </div>
                    <button className="nd-action-dismiss" aria-label="Dismiss"><Icon.X size={11} /></button>
                  </li>
                );
              })}
            </ul>
          </div>
        )}
        {done.length > 0 && (
          <div className="nd-section">
            <div className="nd-section-head muted">Done</div>
            <ul className="nd-actions">
              {done.map((a) => (
                <li key={a.id} className="nd-action done">
                  <span className="nd-action-check checked"><Icon.Check size={11} /></span>
                  <div className="nd-action-body">
                    <div className="nd-action-text">{a.text}</div>
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </section>
  );
}

// ─── Screens ────────────────────────────────────────────────────────────

function InboxScreen({ initialFilter = "all", initialSelectedId = "n1", initialShowRaw = false, initialReparseOpen = false, initialMode = "notes" }) {
  const [selectedId, setSelectedId] = React.useState(initialSelectedId);
  const [filter, setFilter] = React.useState(initialFilter);
  const [query, setQuery] = React.useState("");
  const [showRaw, setShowRaw] = React.useState(initialShowRaw);
  const [reparseOpen, setReparseOpen] = React.useState(initialReparseOpen);
  const [mode, setMode] = React.useState(initialMode);
  const notes = SAMPLE_NOTES;
  const selected = notes.find((n) => n.id === selectedId);
  return (
    <div className="app">
      <Sidebar active={mode === "actions" ? "actions" : "inbox"} inboxCount={notes.length} />
      {mode === "notes" ? (
        <InboxList notes={notes} selectedId={selectedId} onSelect={setSelectedId}
                   filter={filter} setFilter={setFilter}
                   query={query} setQuery={setQuery}
                   mode={mode} setMode={setMode} />
      ) : (
        <ActionsList notes={notes} selectedId={selectedId} onSelect={setSelectedId}
                     mode={mode} setMode={setMode} />
      )}
      <NoteDetail note={selected} showRaw={showRaw} setShowRaw={setShowRaw}
                  reparseOpen={reparseOpen} setReparseOpen={setReparseOpen} />
    </div>
  );
}

window.InboxScreen = InboxScreen;
window.Sidebar = Sidebar;
window.StatusDot = StatusDot;
window.TagChip = TagChip;
window.Kbd = Kbd;
window.IconBtn = IconBtn;
