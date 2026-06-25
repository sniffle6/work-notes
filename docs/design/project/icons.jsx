/* global React */
// Tiny line icon set — 16px viewBox, stroke=1.6, currentColor.
// Hand-tuned for crispness at 14–18px.

const sw = 1.6;
const ico = (paths) => ({ size = 16, ...rest } = {}) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor"
       strokeWidth={sw} strokeLinecap="round" strokeLinejoin="round" {...rest}>
    {paths}
  </svg>
);

const Icon = {
  Inbox: ico(<>
    <path d="M2 9.5 L4 3.5 H12 L14 9.5" />
    <path d="M2 9.5 H5.5 L6.5 11 H9.5 L10.5 9.5 H14 V12.5 H2 Z" />
  </>),
  Today: ico(<>
    <rect x="2.5" y="3.5" width="11" height="10" rx="1.5" />
    <path d="M2.5 6.5 H13.5" />
    <path d="M5.5 2 V5" />
    <path d="M10.5 2 V5" />
  </>),
  Tag: ico(<>
    <path d="M2.5 7.5 V3 H7 L13.5 9.5 L9.5 13.5 L3 7 Z" />
    <circle cx="5" cy="5.5" r="0.6" fill="currentColor" />
  </>),
  People: ico(<>
    <circle cx="6" cy="6" r="2.5" />
    <path d="M2 13 C2.5 10.5 4 9.5 6 9.5 C8 9.5 9.5 10.5 10 13" />
    <path d="M10 4.5 C11.5 4.7 12.5 5.7 12.5 7 C12.5 8.3 11.5 9.3 10 9.5" />
    <path d="M11.5 13 C11.7 11.5 12.7 10.5 14 10.3" />
  </>),
  Archive: ico(<>
    <rect x="2" y="3" width="12" height="3" rx="0.6" />
    <path d="M3 6 V13 H13 V6" />
    <path d="M6.5 9 H9.5" />
  </>),
  Search: ico(<>
    <circle cx="7" cy="7" r="4" />
    <path d="M10 10 L13.5 13.5" />
  </>),
  Plus: ico(<>
    <path d="M8 3 V13" />
    <path d="M3 8 H13" />
  </>),
  Dot: ico(<>
    <circle cx="8" cy="8" r="2.5" fill="currentColor" stroke="none" />
  </>),
  More: ico(<>
    <circle cx="3.5" cy="8" r="0.9" fill="currentColor" stroke="none" />
    <circle cx="8" cy="8" r="0.9" fill="currentColor" stroke="none" />
    <circle cx="12.5" cy="8" r="0.9" fill="currentColor" stroke="none" />
  </>),
  Trash: ico(<>
    <path d="M3 4.5 H13" />
    <path d="M6 4.5 V3 H10 V4.5" />
    <path d="M4 4.5 L4.7 13 H11.3 L12 4.5" />
    <path d="M6.5 7 V11" />
    <path d="M9.5 7 V11" />
  </>),
  Archive2: ico(<>
    <path d="M2 4.5 L4 3 H12 L14 4.5" />
    <rect x="3" y="4.5" width="10" height="8.5" rx="1" />
    <path d="M6.5 8 H9.5" />
  </>),
  Refresh: ico(<>
    <path d="M13 5 C12 3.5 10.1 2.5 8 2.5 C4.7 2.5 2 5.2 2 8.5" />
    <path d="M13 2.5 V5 H10.5" />
    <path d="M3 12 C4 13.5 5.9 14.5 8 14.5 C11.3 14.5 14 11.8 14 8.5" />
    <path d="M3 14.5 V12 H5.5" />
  </>),
  Sparkle: ico(<>
    <path d="M8 2.5 L9 6 L12.5 7 L9 8 L8 11.5 L7 8 L3.5 7 L7 6 Z" />
    <path d="M13 11 L13.5 12.5 L15 13 L13.5 13.5 L13 15" strokeWidth="1.2" />
  </>),
  Settings: ico(<>
    <circle cx="8" cy="8" r="2" />
    <path d="M8 1.5 V3 M8 13 V14.5 M14.5 8 H13 M3 8 H1.5 M12.6 3.4 L11.5 4.5 M4.5 11.5 L3.4 12.6 M12.6 12.6 L11.5 11.5 M4.5 4.5 L3.4 3.4" />
  </>),
  Check: ico(<>
    <path d="M3 8.5 L6.5 12 L13 4.5" />
  </>),
  X: ico(<>
    <path d="M4 4 L12 12" />
    <path d="M12 4 L4 12" />
  </>),
  Eye: ico(<>
    <path d="M1.5 8 C3 5 5.3 3.5 8 3.5 C10.7 3.5 13 5 14.5 8 C13 11 10.7 12.5 8 12.5 C5.3 12.5 3 11 1.5 8 Z" />
    <circle cx="8" cy="8" r="1.8" />
  </>),
  Alert: ico(<>
    <path d="M8 2.5 L14 13 H2 Z" />
    <path d="M8 6.5 V9.5" />
    <circle cx="8" cy="11.5" r="0.6" fill="currentColor" stroke="none" />
  </>),
  Loader: ico(<>
    <circle cx="8" cy="8" r="5" strokeOpacity="0.25" />
    <path d="M13 8 A5 5 0 0 0 8 3" />
  </>),
  Pin: ico(<>
    <path d="M8 2 L11 5 L10 6 L11 9 L8 9 L8 13.5" />
    <path d="M5 6 L8 9" />
  </>),
  Cmd: ico(<>
    <path d="M5 3.5 A1.5 1.5 0 1 1 5 6.5 H11 A1.5 1.5 0 1 1 11 9.5 H5 A1.5 1.5 0 1 1 5 12.5 V3.5 Z" />
  </>),
  Tray: ico(<>
    <rect x="2" y="3" width="12" height="10" rx="1.5" />
    <path d="M5 13 V14" />
    <path d="M11 13 V14" />
    <circle cx="5" cy="6" r="0.5" fill="currentColor" stroke="none" />
    <circle cx="7.5" cy="6" r="0.5" fill="currentColor" stroke="none" />
  </>),
};

window.Icon = Icon;
