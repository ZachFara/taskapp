import type { Api } from './api';

export type ThemeMode = 'light' | 'dark' | 'system';
const KEY = 'cairn.theme';
const order: ThemeMode[] = ['light', 'dark', 'system'];

// The settings row (`theme`) is the source of truth. localStorage is only a startup cache, read before the first
// paint so the app never flashes the wrong theme while the backend answers.
function cached(): ThemeMode {
  try {
    const v = localStorage.getItem(KEY);
    return order.includes(v as ThemeMode) ? (v as ThemeMode) : 'system';
  } catch {
    return 'system';
  }
}

export const theme = $state<{ mode: ThemeMode }>({ mode: cached() });

// `system` leaves data-theme off so tokens.css follows prefers-color-scheme.
export function applyTheme() {
  const root = document.documentElement;
  if (theme.mode === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', theme.mode);
}
applyTheme();

let backend: Pick<Api, 'settings'> | null = null;

/** Hands the store its api and reconciles with the saved setting (the backend wins over the cache). */
export function initTheme(api: Pick<Api, 'settings'>) {
  backend = api;
  void api.settings
    .get('theme')
    .then((v) => {
      if (order.includes(v as ThemeMode) && v !== theme.mode) setTheme(v as ThemeMode, false);
    })
    .catch(() => {});
}

export function setTheme(mode: ThemeMode, persist = true) {
  theme.mode = mode;
  try {
    localStorage.setItem(KEY, mode);
  } catch {
    // Storage blocked: the cache is skipped, the saved setting still holds.
  }
  applyTheme();
  if (persist) void backend?.settings.set('theme', mode).catch(() => {});
}

export const cycleTheme = () => setTheme(order[(order.indexOf(theme.mode) + 1) % order.length]);
