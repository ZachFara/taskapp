import { api, type Api } from './api';

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

export const SKINS = [
  { id: 'survey', label: 'Survey map' },
  { id: 'linear', label: 'Linear' },
  { id: 'jira', label: 'Jira' },
  { id: 'cupertino', label: 'Cupertino' },
  { id: 'obsidian', label: 'Obsidian' },
  { id: 'scholastic', label: 'Scholastic' },
] as const;
export type Skin = (typeof SKINS)[number]['id'];
const SKIN_KEY = 'cairn.skin';
const isSkin = (v: unknown): v is Skin => SKINS.some((s) => s.id === v);

function cachedSkin(): Skin {
  try {
    const v = localStorage.getItem(SKIN_KEY);
    return isSkin(v) ? v : 'survey';
  } catch {
    return 'survey';
  }
}

export const theme = $state<{ mode: ThemeMode; skin: Skin }>({ mode: cached(), skin: cachedSkin() });

// `system` leaves data-theme off so tokens.css follows prefers-color-scheme.
export function applyTheme() {
  const root = document.documentElement;
  if (theme.mode === 'system') root.removeAttribute('data-theme');
  else root.setAttribute('data-theme', theme.mode);
  // The first skin is tokens.css itself, so it carries no attribute.
  if (theme.skin === 'survey') root.removeAttribute('data-skin');
  else root.setAttribute('data-skin', theme.skin);
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
  void api.settings
    .get('skin')
    .then((v) => {
      if (isSkin(v) && v !== theme.skin) setSkin(v, false);
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

export function setSkin(skin: Skin, persist = true) {
  theme.skin = skin;
  try {
    localStorage.setItem(SKIN_KEY, skin);
  } catch {
    // Storage blocked: the cache is skipped, the saved setting still holds.
  }
  applyTheme();
  if (persist) void backend?.settings.set('skin', skin).catch(() => {});
}

export const cycleTheme = () => setTheme(order[(order.indexOf(theme.mode) + 1) % order.length]);

/** The first day of the week from the `week_start` setting; Sunday when it is unset or unreadable. */
export async function getWeekStart(): Promise<'sunday' | 'monday'> {
  try {
    return (await api.settings.get('week_start')) === 'monday' ? 'monday' : 'sunday';
  } catch {
    return 'sunday';
  }
}
