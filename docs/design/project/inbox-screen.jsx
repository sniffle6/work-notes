/* global React, Icon, SAMPLE_NOTES, formatWhen, formatClock */

// ─── Atoms ────────────────────────────────────────────────────────────────

function StatusDot({ status, review, size = 8 }) {
  // Combine parse + review into one signal — most informative one wins.
  let color, label, pulse = false;
  if (status === "failed") { color = "var(--err)"; label = "Parse failed"; }
  else if (status === "parsing") { color = "var(--info)"; label = "Parsing"; pulse = true; }
  else if (status === "queued") { color = "var(--text-subtle)"; label = "Queued"; }
  else if (review === "needs_review") { color = "var(--warn)"; label = "Needs review"; }
  else if (review === "reviewed") { color = "var(--ok)"; label = "Reviewed"; }
  else { color = "var(--text-subtle)"; label = "Captured"; }
  return (
    <span className="status-dot" title={label}
          style={{ background: color, width: size, height: size,
                   boxShadow: pulse ? `0 0 0 3px ${color.replace(")", " / 0.2)").replace("oklch", "oklch")}` : "none",
                   animation: pulse ? "wn-pulse 1.6s ease-in-out infinite" : "none" }} />
  );
}

function TagChip({ tag, subtle = false }) {
  const symbol = tag.kind === "person" ? "@" : tag.kind === "project" ? "#" : tag.kind === "urgency" ? "!" : "·";
  return (
    <span className={`tag-chip ${subtle ? "subtle" : ""} kind-${tag.kind}`}>
      <span className="tag-sym">{symbol}</span>{tag.name}
    </span>
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

// ─── Sidebar ──────────────────────────────────────────────────────────────

function Sidebar({ active = "inbox", inboxCount, needsReview }) {
  const nav = [
    { id: "inbox", label: "Inbox", icon: Icon.Inbox, count: inboxCount },
    { id: "today", label: "Today", icon: Icon.Today, count: 3 },
    { id: "tags", label: "Tags", icon: Icon.Tag },
    { id: "people", label: "People", icon: Icon.People },
    { id: "archive", label: "Archive", icon: Icon.Archive },
  ];
  const tags = [
    { name: "Q3-dashboard", kind: "project" },
    { name: "integrations", kind: "project" },
    { name: "Maria", kind: "person" },
    { name: "roadmap", kind: "topic" },
  ];
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
        <span className="capture-btn-kbd"><Kbd>Ctrl</Kbd><Kbd>⇧</Kbd><Kbd>Space</Kbd></span>
      </button>

      <nav className="sb-nav" aria-label="Workspace">
        {nav.map((n) => {
          const Ic = n.icon;
          return (
            <button key={n.id} className={`sb-nav-item ${active === n.id ? "active" : ""}`}>
              <Ic size={15} />
              <span className="sb-nav-label">{n.label}</span>
              {n.count ? <span className="sb-nav-count">{n.count}</span> : null}
            </button>
          );
        })}
      </nav>

      <div className="sb-section-label">Tags</div>
      <div className="sb-tags">
        {tags.map((t) => <TagChip key={t.name} tag={t} subtle />)}
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

// ─── Inbox list ──────────────────────────────────────────────────────────

function InboxList({ notes, selectedId, onSelect, filter, setFilter, query, setQuery }) {
  const filters = [
    { id: "all", label: "All", count: notes.length },
    { id: "review", label: "Needs review", count: notes.filter(n => n.reviewStatus === "needs_review").length },
    { id: "actions", label: "Has actions", count: notes.filter(n => n.actions.some(a => a.status === "suggested")).length },
    { id: "failed", label: "Failed", count: notes.filter(n => n.parseStatus === "failed").length },
  ];

  const filtered = notes.filter((n) => {
    if (filter === "review") return n.reviewStatus === "needs_review";
    if (filter === "actions") return n.actions.some(a => a.status === "suggested");
    if (filter === "failed") return n.parseStatus === "failed";
    return true;
  }).filter((n) => {
    if (!query) return true;
    const q = query.toLowerCase();
    return n.title.toLowerCase().includes(q) || n.raw.toLowerCase().includes(q);
  });

  return (
    <section className="inbox-list">
      <header className="il-head">
        <div className="il-head-row">
          <h1 className="il-title">Inbox</h1>
          <div className="il-tools">
            <IconBtn icon={Icon.Refresh} label="Refresh" />
            <IconBtn icon={Icon.More} label="More" />
          </div>
        </div>
        <div className="il-search">
          <Icon.Search size={14} />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search notes, people, tags"
          />
          <Kbd>/</Kbd>
        </div>
        <div className="il-filters">
          {filters.map((f) => (
            <button key={f.id}
              className={`il-filter ${filter === f.id ? "active" : ""}`}
              onClick={() => setFilter(f.id)}>
              {f.label}
              {f.count > 0 && <span className="il-filter-count">{f.count}</span>}
            </button>
          ))}
        </div>
      </header>

      <div className="il-scroll">
        {filtered.length === 0 && (
          <div className="il-empty">
            <div className="il-empty-mark">·</div>
            <div className="il-empty-title">No notes match</div>
            <div className="il-empty-sub">Try clearing the filter or search.</div>
          </div>
        )}
        {filtered.map((n) => (
          <button key={n.id}
            className={`il-card ${selectedId === n.id ? "selected" : ""}`}
            onClick={() => onSelect(n.id)}>
            <div className="il-card-top">
              <StatusDot status={n.parseStatus} review={n.reviewStatus} />
              <span className="il-card-source">{n.captureSource}</span>
              <span className="il-card-when">{formatWhen(n.when)}</span>
            </div>
            <div className="il-card-title">
              {n.pinned && <Icon.Pin size={11} />}
              <span>{n.title}</span>
            </div>
            <div className="il-card-preview">{n.summary ?? n.raw}</div>
            {(n.tags.length > 0 || n.actions.length > 0) && (
              <div className="il-card-meta">
                {n.tags.slice(0, 3).map((t) => <TagChip key={t.name} tag={t} subtle />)}
                {n.tags.length > 3 && <span className="il-card-more">+{n.tags.length - 3}</span>}
                {n.actions.filter(a => a.status === "suggested").length > 0 && (
                  <span className="il-card-actions">
                    <Icon.Check size={11} /> {n.actions.filter(a => a.status === "suggested").length} action{n.actions.filter(a => a.status === "suggested").length !== 1 ? "s" : ""}
                  </span>
                )}
              </div>
            )}
          </button>
        ))}
      </div>
    </section>
  );
}

// ─── Note detail ─────────────────────────────────────────────────────────

function NoteDetail({ note, showRaw, setShowRaw, reparseOpen, setReparseOpen }) {
  if (!note) {
    return (
      <section className="note-detail empty">
        <div className="nd-empty">
          <div className="nd-empty-mark"><Icon.Inbox size={28} /></div>
          <div className="nd-empty-title">Select a note</div>
          <div className="nd-empty-sub">Or press <Kbd>Ctrl</Kbd>+<Kbd>⇧</Kbd>+<Kbd>Space</Kbd> to capture a new one.</div>
        </div>
      </section>
    );
  }

  return (
    <section className="note-detail">
      <header className="nd-head">
        <div className="nd-head-left">
          <div className="nd-head-meta">
            <StatusDot status={note.parseStatus} review={note.reviewStatus} />
            <span>{note.captureSource}</span>
            <span className="nd-dot-sep">·</span>
            <span>{formatWhen(note.when)} at {formatClock(note.when)}</span>
          </div>
        </div>
        <div className="nd-head-right">
          <button className="nd-pill-btn">
            <Icon.Check size={13} /> Mark reviewed
          </button>
          <IconBtn icon={Icon.Refresh} label="Reparse" />
          <IconBtn icon={Icon.Archive2} label="Archive" />
          <IconBtn icon={Icon.More} label="More" />
        </div>
      </header>

      <div className="nd-scroll">
        <h1 className="nd-title">{note.title}</h1>

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
              <div className="nd-banner-sub">Codex is cleaning, summarizing, tagging. Raw note is saved.</div>
            </div>
          </div>
        )}

        {note.summary && (
          <div className="nd-summary">
            <div className="nd-eyebrow"><Icon.Sparkle size={11} /> Summary</div>
            <p>{note.summary}</p>
          </div>
        )}

        <div className="nd-body-head">
          <div className="nd-tabs">
            <button className={`nd-tab ${!showRaw ? "active" : ""}`} onClick={() => setShowRaw(false)}>
              Cleaned
            </button>
            <button className={`nd-tab ${showRaw ? "active" : ""}`} onClick={() => setShowRaw(true)}>
              Raw
            </button>
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
            <textarea placeholder="What did the parser get wrong? e.g. &quot;Anita is the owner, not me&quot;"></textarea>
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
            <div className="nd-section-head"><Icon.Tag size={12} /> Tags</div>
            <div className="nd-tags">
              {note.tags.map((t) => <TagChip key={t.name} tag={t} />)}
              <button className="nd-tag-add">+ add</button>
            </div>
          </div>
        )}

        {note.actions.length > 0 && (
          <div className="nd-section">
            <div className="nd-section-head"><Icon.Check size={12} /> Suggested actions</div>
            <ol className="nd-actions">
              {note.actions.map((a, i) => (
                <li key={a.id} className="nd-action">
                  <span className="nd-action-num">{i + 1}</span>
                  <div className="nd-action-body">
                    <div className="nd-action-text">{a.text}</div>
                    <div className="nd-action-meta">
                      {a.owner && <span>@{a.owner}</span>}
                      {a.due && <><span>·</span><span>due {a.due}</span></>}
                    </div>
                  </div>
                  <div className="nd-action-btns">
                    <button className="nd-action-accept"><Icon.Check size={12} /> Accept</button>
                    <button className="nd-action-dismiss" aria-label="Dismiss"><Icon.X size={12} /></button>
                  </div>
                </li>
              ))}
            </ol>
          </div>
        )}
      </div>
    </section>
  );
}

// ─── Screen ──────────────────────────────────────────────────────────────

function InboxScreen({ initialFilter = "all", initialSelectedId = "n1", initialShowRaw = false, initialReparseOpen = false }) {
  const [selectedId, setSelectedId] = React.useState(initialSelectedId);
  const [filter, setFilter] = React.useState(initialFilter);
  const [query, setQuery] = React.useState("");
  const [showRaw, setShowRaw] = React.useState(initialShowRaw);
  const [reparseOpen, setReparseOpen] = React.useState(initialReparseOpen);
  const notes = SAMPLE_NOTES;
  const selected = notes.find((n) => n.id === selectedId);
  const needsReview = notes.filter(n => n.reviewStatus === "needs_review").length;

  return (
    <div className="app">
      <Sidebar active="inbox" inboxCount={notes.length} needsReview={needsReview} />
      <InboxList
        notes={notes}
        selectedId={selectedId}
        onSelect={setSelectedId}
        filter={filter} setFilter={setFilter}
        query={query} setQuery={setQuery}
      />
      <NoteDetail
        note={selected}
        showRaw={showRaw} setShowRaw={setShowRaw}
        reparseOpen={reparseOpen} setReparseOpen={setReparseOpen}
      />
    </div>
  );
}

window.InboxScreen = InboxScreen;
window.Sidebar = Sidebar;
window.InboxList = InboxList;
window.NoteDetail = NoteDetail;
window.StatusDot = StatusDot;
window.TagChip = TagChip;
window.Kbd = Kbd;
window.IconBtn = IconBtn;
