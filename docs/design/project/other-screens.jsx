/* global React, Icon, Kbd, TagChip, StatusDot, formatWhen */

// ─── Quick Capture popup window ─────────────────────────────────
// Lives in its own Tauri window — bottom-right of screen.
// Shows the "shell" as a windowed floating panel.

function QuickCapture() {
  const [text, setText] = React.useState(
    "stopped by maria's desk, q3 dashboard broken since migration, no clear owner since dan left. needs decision by fri"
  );
  return (
    <div className="qc-stage">
      <div className="qc-window">
        <div className="qc-titlebar">
          <div className="qc-titlebar-left">
            <span className="qc-dot" />
            <span className="qc-title-text">Quick capture</span>
          </div>
          <div className="qc-titlebar-right">
            <span className="qc-tray-hint">
              <Icon.Tray size={11} /> Tray
            </span>
            <button className="qc-close" aria-label="Close"><Icon.X size={13} /></button>
          </div>
        </div>
        <div className="qc-body">
          <textarea
            value={text}
            onChange={(e) => setText(e.target.value)}
            placeholder="Name, ask, deadline, next step…"
            autoFocus
          />
          <div className="qc-hints">
            <div className="qc-hint-row">
              <Icon.Sparkle size={11} />
              <span>Detected: <strong>@maria</strong>, <strong>#q3-dashboard</strong>, <strong>due Fri</strong></span>
            </div>
          </div>
        </div>
        <div className="qc-foot">
          <div className="qc-foot-left">
            <span><Kbd>Esc</Kbd> close</span>
            <span><Kbd>⇧</Kbd>+<Kbd>↵</Kbd> newline</span>
          </div>
          <button className="qc-save">
            Save <span className="qc-save-kbd"><Kbd>↵</Kbd></span>
          </button>
        </div>
      </div>
    </div>
  );
}

// ─── Settings sheet ─────────────────────────────────────────────
function SettingsSheet() {
  return (
    <div className="app">
      <div className="settings-backdrop" />
      <div className="settings-sheet">
        <header className="ss-head">
          <div>
            <h1>Settings</h1>
            <p>Tune capture, parsing, and where Work Notes lives on your machine.</p>
          </div>
          <button className="ss-close"><Icon.X size={14} /></button>
        </header>
        <div className="ss-body">
          <aside className="ss-nav">
            {[
              { id: "general", label: "General", icon: Icon.Settings },
              { id: "capture", label: "Capture & hotkey", icon: Icon.Plus, active: true },
              { id: "parser", label: "Parser", icon: Icon.Sparkle },
              { id: "tags", label: "Tags & people", icon: Icon.Tag },
              { id: "data", label: "Data & backup", icon: Icon.Archive2 },
              { id: "about", label: "About", icon: Icon.Eye },
            ].map((s) => {
              const I = s.icon;
              return (
                <button key={s.id} className={`ss-nav-item ${s.active ? "active" : ""}`}>
                  <I size={14} />
                  <span>{s.label}</span>
                </button>
              );
            })}
          </aside>
          <div className="ss-content">
            <section className="ss-section">
              <div className="ss-section-head">
                <h2>Capture & hotkey</h2>
                <p>The hotkey opens a small capture window from the tray. Press it, type, hit Enter, get back to work.</p>
              </div>

              <div className="ss-field-row">
                <div className="ss-field">
                  <label>Global hotkey</label>
                  <div className="ss-hotkey">
                    <Kbd>Ctrl</Kbd><Kbd>⇧</Kbd><Kbd>Space</Kbd>
                    <button className="ss-rebind">Rebind</button>
                  </div>
                  <p className="ss-help">Works while any other window is focused.</p>
                </div>
                <div className="ss-field">
                  <label>Capture window position</label>
                  <select defaultValue="bottom-right">
                    <option value="bottom-right">Bottom right</option>
                    <option value="bottom-center">Bottom center</option>
                    <option value="follow-cursor">Follow cursor</option>
                  </select>
                </div>
              </div>

              <div className="ss-toggle-row">
                <div>
                  <label>Launch at startup</label>
                  <p className="ss-help">Start Work Notes when Windows boots and keep it in the tray.</p>
                </div>
                <Toggle on />
              </div>
              <div className="ss-toggle-row">
                <div>
                  <label>Minimize to tray on close</label>
                  <p className="ss-help">Closing the window keeps capture alive in the background.</p>
                </div>
                <Toggle on />
              </div>
              <div className="ss-toggle-row">
                <div>
                  <label>Detect names, projects, and dates as you type</label>
                  <p className="ss-help">Shown inline in the capture window before save.</p>
                </div>
                <Toggle on />
              </div>
            </section>

            <section className="ss-section">
              <div className="ss-section-head">
                <h2>Parser</h2>
                <p>Cleaning, summarizing, and tagging run locally via your Codex subscription. No API keys, no data leaves your machine.</p>
              </div>
              <div className="ss-field-row">
                <div className="ss-field span2">
                  <label>Codex command</label>
                  <div className="ss-path">
                    <Icon.Cmd size={13} />
                    <code>C:\Users\you\AppData\Local\codex\codex.cmd</code>
                    <button className="ss-rebind">Browse</button>
                  </div>
                  <p className="ss-help status-ok">
                    <span className="parser-dot" /> Codex command found · responded in 1.2s
                  </p>
                </div>
              </div>
              <div className="ss-field-row">
                <div className="ss-field">
                  <label>Parser timeout</label>
                  <div className="ss-num">
                    <input defaultValue="45" />
                    <span>seconds</span>
                  </div>
                </div>
                <div className="ss-field">
                  <label>Max retries</label>
                  <div className="ss-num">
                    <input defaultValue="2" />
                    <span>attempts</span>
                  </div>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}

function Toggle({ on = false }) {
  return (
    <button className={`toggle ${on ? "on" : ""}`} aria-pressed={on}>
      <span className="toggle-knob" />
    </button>
  );
}

// ─── Empty state (first run) ────────────────────────────────────
function EmptyStateScreen() {
  return (
    <div className="app">
      {React.createElement(window.Sidebar, { active: "inbox", inboxCount: 0, needsReview: 0 })}
      <section className="inbox-list">
        <header className="il-head">
          <div className="il-head-row">
            <h1 className="il-title">Inbox</h1>
          </div>
          <div className="il-search">
            <Icon.Search size={14} />
            <input placeholder="Search notes, people, tags" disabled />
          </div>
        </header>
        <div className="il-empty-first">
          <div className="il-empty-illu">
            <Icon.Inbox size={28} />
          </div>
          <h2>No notes yet</h2>
          <p>Press the hotkey to capture your first one.</p>
          <div className="il-empty-kbd">
            <Kbd>Ctrl</Kbd><Kbd>⇧</Kbd><Kbd>Space</Kbd>
          </div>
        </div>
      </section>
      <section className="note-detail">
        <div className="onboarding">
          <div className="onb-eyebrow">Welcome to Work Notes</div>
          <h1>Capture in 2 seconds. Sort it out later.</h1>
          <p>The fast path: a global hotkey opens a tiny window. Type, press Enter, get back to your day. Codex cleans it up in the background.</p>

          <div className="onb-steps">
            <div className="onb-step">
              <div className="onb-step-num">1</div>
              <div>
                <h3>Capture</h3>
                <p>Hit <Kbd>Ctrl</Kbd>+<Kbd>⇧</Kbd>+<Kbd>Space</Kbd> from anywhere. The capture window appears bottom-right and focuses the input.</p>
              </div>
            </div>
            <div className="onb-step">
              <div className="onb-step-num">2</div>
              <div>
                <h3>Save instantly</h3>
                <p>Press <Kbd>↵</Kbd> to save. Your raw text is the source of truth — it's never overwritten.</p>
              </div>
            </div>
            <div className="onb-step">
              <div className="onb-step-num">3</div>
              <div>
                <h3>Codex cleans up</h3>
                <p>A background pass extracts a title, summary, tags, and suggested action items. You stay in flow.</p>
              </div>
            </div>
          </div>

          <div className="onb-actions">
            <button className="onb-primary">Capture your first note</button>
            <button className="onb-secondary">Import from Markdown</button>
          </div>
        </div>
      </section>
    </div>
  );
}

window.QuickCapture = QuickCapture;
window.SettingsSheet = SettingsSheet;
window.EmptyStateScreen = EmptyStateScreen;
