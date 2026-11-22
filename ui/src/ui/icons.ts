// One 16px set, drawn as inline SVG: 1.5 stroke, round caps and joins, no fill unless the markup says so.
// Ids are the `icon:<id>` values the backend stores.
const dot = (x: number, y: number) => `M${x} ${y}h.01`;

export const ICONS: Record<string, string> = {
  // brand
  cairn: `<rect x="1.8" y="10.2" width="12.4" height="4" rx="2" fill="currentColor" stroke="none"/><rect x="3.6" y="5.9" width="8.8" height="3.5" rx="1.75" fill="currentColor" stroke="none"/><rect x="5.6" y="2" width="4.8" height="3.1" rx="1.55" fill="currentColor" stroke="none"/>`,
  // navigation
  inbox: `<path d="M2 9.5h3.2l.8 1.8h4l.8-1.8H14M2 9.5L3.6 4h8.8L14 9.5V13H2z"/>`,
  user: `<circle cx="8" cy="5.5" r="2.5"/><path d="M3 13.5c0-2.8 2.3-4.5 5-4.5s5 1.7 5 4.5"/>`,
  'user-circle': `<circle cx="8" cy="8" r="6"/><circle cx="8" cy="6.5" r="2"/><path d="M4.2 12.6c.8-1.6 2.1-2.4 3.8-2.4s3 .8 3.8 2.4"/>`,
  pin: `<path d="M10 2.5l3.5 3.5-1.6.7-2.3 2.3.3 2.2-.9.9-5-5 .9-.9 2.2.3 2.3-2.3zM5.2 10.8l-2.7 2.7"/>`,
  home: `<path d="M2.5 7.5L8 2.8l5.5 4.7M4 6.8V13h8V6.8M6.8 13V9.5h2.4V13"/>`,
  settings: `<circle cx="8" cy="8" r="2.3"/><path d="M8 1.8v1.9M8 12.3v1.9M1.8 8h1.9M12.3 8h1.9M3.6 3.6l1.3 1.3M11.1 11.1l1.3 1.3M12.4 3.6l-1.3 1.3M4.9 11.1l-1.3 1.3"/>`,
  search: `<circle cx="7" cy="7" r="4.3"/><path d="M10.2 10.2l3.3 3.3"/>`,
  command: `<path d="M5.5 5.5h5v5h-5zM5.5 5.5V4A1.5 1.5 0 1 0 4 5.5zM10.5 5.5V4A1.5 1.5 0 1 1 12 5.5zM5.5 10.5V12A1.5 1.5 0 1 1 4 10.5zM10.5 10.5V12a1.5 1.5 0 1 0 1.5-1.5z"/>`,
  bell: `<path d="M4 11V7a4 4 0 0 1 8 0v4l1 1.3H3zM6.8 14h2.4"/>`,
  help: `<circle cx="8" cy="8" r="6"/><path d="M6.2 6.4a1.9 1.9 0 1 1 2.7 1.7c-.6.3-.9.7-.9 1.3"/><path d="${dot(8, 11.5)}" stroke-width="1.9"/>`,
  // layouts
  list: `<path d="M5.5 4h8M5.5 8h8M5.5 12h8"/><path d="${dot(2.5, 4)}${dot(2.5, 8)}${dot(2.5, 12)}" stroke-width="2"/>`,
  board: `<rect x="2.5" y="3" width="3.2" height="10" rx="1"/><rect x="6.4" y="3" width="3.2" height="6.5" rx="1"/><rect x="10.3" y="3" width="3.2" height="8.2" rx="1"/>`,
  table: `<rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M2 6.5h12M6.5 6.5V13"/>`,
  timeline: `<rect x="2.5" y="3" width="6" height="2.4" rx="1.2"/><rect x="6" y="6.8" width="7.5" height="2.4" rx="1.2"/><rect x="4" y="10.6" width="5.5" height="2.4" rx="1.2"/>`,
  calendar: `<rect x="2.5" y="3.5" width="11" height="10" rx="1.5"/><path d="M2.5 6.8h11M5.5 2v3M10.5 2v3"/>`,
  layers: `<path d="M8 2l6 3.3-6 3.3-6-3.3zM2 8.2l6 3.3 6-3.3M2 11l6 3.3 6-3.3"/>`,
  panel: `<rect x="2" y="3" width="12" height="10" rx="1.5"/><path d="M10 3v10"/>`,
  // entities
  project: `<path d="M1.8 13.2l4.4-8 2.7 4.6 1.7-2.4 3.6 5.8z"/>`,
  cycle: `<path d="M13 8a5 5 0 1 1-1.6-3.7M13 2.5v2.8h-2.8"/>`,
  folder: `<path d="M2 4.5a1 1 0 0 1 1-1h3l1.5 1.8H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"/>`,
  'folder-open': `<path d="M2 12.5v-8a1 1 0 0 1 1-1h3l1.4 1.6H12a1 1 0 0 1 1 1V7M2 12.5l1.6-4.9a1 1 0 0 1 .95-.7H14a.8.8 0 0 1 .78 1l-1.4 4.6a1 1 0 0 1-.95.7H2z"/>`,
  tag: `<path d="M2.5 2.5h5l6 6-5 5-6-6z"/><path d="${dot(5.3, 5.3)}" stroke-width="1.8"/>`,
  flag: `<path d="M3.5 14V2.5M3.5 3h8l-1.6 2.6 1.6 2.6h-8"/>`,
  star: `<path d="M8 1.8l1.9 4 4.3.5-3.2 2.9.9 4.3L8 11.4l-3.9 2.1.9-4.3L1.8 6.3l4.3-.5z"/>`,
  bookmark: `<path d="M4 2.5h8v11l-4-3-4 3z"/>`,
  target: `<circle cx="8" cy="8" r="6"/><circle cx="8" cy="8" r="3"/><path d="${dot(8, 8)}" stroke-width="1.6"/>`,
  bolt: `<path d="M9 1.8L3.8 9h3.7l-.8 5.2L12.2 7H8.5z"/>`,
  circle: `<circle cx="8" cy="8" r="5.5"/>`,
  branch: `<circle cx="4.5" cy="3.5" r="1.6"/><circle cx="4.5" cy="12.5" r="1.6"/><circle cx="11.5" cy="6" r="1.6"/><path d="M4.5 5.1v5.8M11.5 7.6c0 2-1.8 2.8-5 3"/>`,
  'sub-issue': `<path d="M4 2.5v6.5a2 2 0 0 0 2 2h6M9.5 8.5l2.7 2.5-2.7 2.5"/>`,
  clock: `<circle cx="8" cy="8" r="6"/><path d="M8 4.8V8l2.2 1.4"/>`,
  link: `<path d="M6.8 9.2a2.6 2.6 0 0 0 3.7 0l2-2a2.6 2.6 0 0 0-3.7-3.7l-.6.6M9.2 6.8a2.6 2.6 0 0 0-3.7 0l-2 2a2.6 2.6 0 0 0 3.7 3.7l.6-.6"/>`,
  // actions
  plus: `<path d="M8 3v10M3 8h10"/>`,
  close: `<path d="M4 4l8 8M12 4l-8 8"/>`,
  check: `<path d="M3.5 8.5l3 3 6-7"/>`,
  more: `<path d="${dot(3.5, 8)}${dot(8, 8)}${dot(12.5, 8)}" stroke-width="2.2"/>`,
  filter: `<path d="M2.5 4h11M4.8 8h6.4M6.8 12h2.4"/>`,
  sort: `<path d="M5 2.5v11M2.8 11.2L5 13.5l2.2-2.3M11 13.5v-11M8.8 4.8L11 2.5l2.2 2.3"/>`,
  group: `<rect x="2.5" y="2.5" width="11" height="3.2" rx="1"/><path d="M5 8.5h8.5M5 12h8.5"/>`,
  edit: `<path d="M10.5 2.8l2.7 2.7-7.6 7.6-3.1.4.4-3.1zM9 4.3L11.7 7"/>`,
  trash: `<path d="M2.8 4.5h10.4M6 4.5V3h4v1.5M4 4.5l.6 8.5h6.8l.6-8.5M6.8 7v3.5M9.2 7v3.5"/>`,
  archive: `<rect x="2" y="3" width="12" height="3" rx=".8"/><path d="M3 6v6.5h10V6M6.5 9h3"/>`,
  copy: `<rect x="5.5" y="5.5" width="8" height="8" rx="1.3"/><path d="M10.5 3.5V3a.8.8 0 0 0-.8-.8H3.3a.8.8 0 0 0-.8.8v6.4a.8.8 0 0 0 .8.8h.7"/>`,
  undo: `<path d="M3.5 6.5h6a3.5 3.5 0 0 1 0 7H6M6 3.5l-2.8 3 2.8 3"/>`,
  download: `<path d="M8 2.5v8M4.5 7.5L8 11l3.5-3.5M3 13.5h10"/>`,
  eye: `<path d="M1.8 8s2.2-4.2 6.2-4.2S14.2 8 14.2 8s-2.2 4.2-6.2 4.2S1.8 8 1.8 8z"/><circle cx="8" cy="8" r="2"/>`,
  sun: `<circle cx="8" cy="8" r="2.8"/><path d="M8 1.8v1.4M8 12.8v1.4M1.8 8h1.4M12.8 8h1.4M3.6 3.6l1 1M11.4 11.4l1 1M12.4 3.6l-1 1M4.6 11.4l-1 1"/>`,
  moon: `<path d="M12.8 9.6A5.5 5.5 0 1 1 6.4 3.2a4.4 4.4 0 0 0 6.4 6.4z"/>`,
  'arrow-right': `<path d="M3 8h10M9 4l4 4-4 4"/>`,
  'arrow-up': `<path d="M8 13V3M4 7l4-4 4 4"/>`,
  'arrow-down': `<path d="M8 3v10M4 9l4 4 4-4"/>`,
  'chevron-down': `<path d="M4 6l4 4 4-4"/>`,
  'chevron-up': `<path d="M4 10l4-4 4 4"/>`,
  'chevron-right': `<path d="M6 4l4 4-4 4"/>`,
  'chevron-left': `<path d="M10 4L6 8l4 4"/>`,
};

export const ICON_IDS = Object.keys(ICONS);
