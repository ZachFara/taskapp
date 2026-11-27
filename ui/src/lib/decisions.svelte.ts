import { api } from './api';
import type { Decision } from './bindings';

/** Every decision, shared by the Decisions page and the sidebar count. */
export const decisions = $state<{ all: Decision[]; loaded: boolean; error: string }>({ all: [], loaded: false, error: '' });

export const openDecisions = () => decisions.all.filter((d) => d.status === 'open');

export async function refreshDecisions() {
  try {
    decisions.all = await api.decisions.list(null);
    decisions.error = '';
  } catch (e) {
    decisions.error = (e as { message?: string }).message ?? String(e);
  }
  decisions.loaded = true;
}

/** Reloads now, when the window gains focus and every minute; returns the stop function. */
export function watchDecisions() {
  void refreshDecisions();
  const timer = setInterval(() => void refreshDecisions(), 60_000);
  addEventListener('focus', refreshDecisions);
  addEventListener('history:wrote', refreshDecisions);
  return () => (clearInterval(timer), removeEventListener('focus', refreshDecisions), removeEventListener('history:wrote', refreshDecisions));
}
