/* global React, Icon, useTweaks, TweaksPanel, TweakSection, TweakRadio */

// ─── Live theme switch via Tweaks panel ──────────────────────────────────

const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
  "theme": "memphis"
}/*EDITMODE-END*/;

function ThemeTweaks() {
  const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
  // Apply to body — cascades into every artboard that hasn't pinned its own theme.
  React.useEffect(() => {
    document.body.setAttribute("data-theme", t.theme);
  }, [t.theme]);

  return (
    <TweaksPanel title="Theme">
      <TweakSection label="Appearance">
        <TweakRadio label="Theme" value={t.theme}
                    options={["dark", "memphis"]}
                    onChange={(v) => setTweak("theme", v)} />
      </TweakSection>
      <TweakSection label="Notes">
        <div style={{ fontSize: 11.5, lineHeight: 1.5, color: "var(--text-muted)", padding: "6px 4px" }}>
          Most artboards follow this setting. A few are pinned to a specific theme for the comparison shots.
        </div>
      </TweakSection>
    </TweaksPanel>
  );
}

// ─── Theme picker UI — used inside Settings → Appearance ────────────────

function ThemeSwatch({ id, name, accent, palette, selected, onClick }) {
  return (
    <button className={`tp-card ${selected ? "selected" : ""}`} data-theme={id} onClick={onClick} type="button">
      <div className="tp-preview">
        <div className="tp-preview-sb" />
        <div className="tp-preview-content">
          <div className="tp-preview-row">
            <div className="tp-preview-dot" style={{ background: palette[0], borderColor: palette[3] }} />
            <div className="tp-preview-line" />
          </div>
          <div className="tp-preview-row">
            <div className="tp-preview-dot" style={{ background: palette[1], borderColor: palette[3] }} />
            <div className="tp-preview-line" />
          </div>
          <div className="tp-preview-row">
            <div className="tp-preview-dot" style={{ background: palette[2], borderColor: palette[3] }} />
            <div className="tp-preview-line short" />
          </div>
        </div>
      </div>
      <div className="tp-meta">
        <div className="tp-name">{name}</div>
        <div className="tp-palette">
          {palette.map((c, i) => <span key={i} className="tp-swatch" style={{ background: c, borderColor: palette[3] }} />)}
        </div>
      </div>
      {selected && <div className="tp-check"><Icon.Check size={11} /></div>}
    </button>
  );
}

function ThemePicker({ value = "memphis", onChange }) {
  return (
    <div className="tp-grid">
      <ThemeSwatch id="dark" name="Graphite (dark)" selected={value === "dark"}
                   onClick={() => onChange && onChange("dark")}
                   palette={["#2f6f7a", "#4f9f6f", "#d69a2d", "#11151c"]} />
      <ThemeSwatch id="memphis" name="Memphis ’86" selected={value === "memphis"}
                   onClick={() => onChange && onChange("memphis")}
                   palette={["#ec4899", "#49c5e3", "#fbbf24", "#121315"]} />
    </div>
  );
}

window.ThemeTweaks = ThemeTweaks;
window.ThemePicker = ThemePicker;

// ─── Settings v3 — adds Appearance section ──────────────────────────────

function SettingsSheetV3({ theme = "memphis" }) {
  const [pick, setPick] = React.useState(theme);
  return (
    <div className="app">
      <div className="settings-backdrop" />
      <div className="settings-sheet">
        <header className="ss-head">
          <div>
            <h1>Settings</h1>
            <p>Tune capture, parsing, and how Work Notes looks on your machine.</p>
          </div>
          <button className="ss-close"><Icon.X size={14} /></button>
        </header>
        <div className="ss-body">
          <aside className="ss-nav">
            {[
              { id: "general", label: "General", icon: Icon.Settings },
              { id: "appearance", label: "Appearance", icon: Icon.Eye, active: true },
              { id: "capture", label: "Capture & hotkey", icon: Icon.Plus },
              { id: "parser", label: "Parser", icon: Icon.Sparkle },
              { id: "tags", label: "Tags & people", icon: Icon.Tag },
              { id: "data", label: "Data & backup", icon: Icon.Archive2 },
              { id: "about", label: "About", icon: Icon.Inbox },
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
                <h2>Appearance</h2>
                <p>Themes are full token swaps — every surface, every status color, every shadow follows the choice. Bring your own at <span className="mono">themes/*.css</span>.</p>
              </div>

              <div className="ss-field span2" style={{ marginBottom: 16 }}>
                <label>Theme</label>
                <ThemePicker value={pick} onChange={setPick} />
              </div>

              <div className="ss-toggle-row">
                <div>
                  <label>Match system color scheme</label>
                  <p className="ss-help">Override the chosen theme with light/dark based on your OS setting.</p>
                </div>
                <button className="toggle" aria-pressed="false"><span className="toggle-knob" /></button>
              </div>
              <div className="ss-toggle-row">
                <div>
                  <label>Reduce motion</label>
                  <p className="ss-help">Disable theme transitions and parsing-pulse animation.</p>
                </div>
                <button className="toggle" aria-pressed="false"><span className="toggle-knob" /></button>
              </div>
            </section>

            <section className="ss-section">
              <div className="ss-section-head">
                <h2>Capture & hotkey</h2>
                <p>The hotkey opens a small capture window from the tray.</p>
              </div>
              <div className="ss-field-row">
                <div className="ss-field">
                  <label>Global hotkey</label>
                  <div className="ss-hotkey">
                    <Kbd>Ctrl</Kbd><Kbd>⇧</Kbd><Kbd>Space</Kbd>
                    <button className="ss-rebind">Rebind</button>
                  </div>
                </div>
                <div className="ss-field">
                  <label>Capture window position</label>
                  <select defaultValue="bottom-right">
                    <option value="bottom-right">Bottom right</option>
                  </select>
                </div>
              </div>
            </section>
          </div>
        </div>
      </div>
    </div>
  );
}

window.SettingsSheetV3 = SettingsSheetV3;
