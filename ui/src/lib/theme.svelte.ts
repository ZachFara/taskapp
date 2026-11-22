export type ThemeMode = 'light' | 'dark' | 'system';
const KEY = 'cairn.theme';
const order: ThemeMode[] = ['light', 'dark', 'system'];

// localStorage and not a backend setting: the theme must be applied before the first paint, before any api call returns.
function read(): ThemeMode {
  try {
    const v = localStorage.getItem(KEY);
    return order.includes(v as ThemeMode) ? (v as ThemeMode) : 'system';
  } catch {
    return 'system';
  }
}

export const theme = $state<{ mode: ThemeMode }>({ mode: read() });

// `system` leaves data-theme off so tokens.css follows prefers-color-scheme.
export function applyTheme() {
  const root = document.documentElement;
  if (theme.mode === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', theme.mode);
}

export function setTheme(mode: ThemeMode) {
  theme.mode = mode;
  try {
    localStorage.setItem(KEY, mode);
  } catch {
    // Storage blocked: the choice lasts until reload.
  }
  applyTheme();
}

export const cycleTheme = () => setTheme(order[(order.indexOf(theme.mode) + 1) % order.length]);
