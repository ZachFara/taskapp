import { api } from './api';
import { isOn } from './features.svelte';

// Ids of issues past their status's nudge age, for the row marker. Reloaded when a list opens and after each write.
export const stale = $state({ ids: new Set<number>() });
// Ids of waiting issues whose follow-up day has come, for the "follow up" row marker. Reloaded with the stale ids.
export const followUps = $state({ ids: new Set<number>() });

export async function refreshStale() {
  stale.ids = isOn('stale') ? new Set((await api.stale.list()).map((s) => s.issue.id)) : new Set();
  if (!isOn('waiting')) return void (followUps.ids = new Set());
  const [tz, waiting] = await Promise.all([api.workspace.status().then((w) => w.timezone, () => ''), api.waiting.list()]);
  const today = tz ? new Intl.DateTimeFormat('en-CA', { timeZone: tz }).format(new Date()) : new Date().toISOString().slice(0, 10);
  followUps.ids = new Set(waiting.filter((i) => i.follow_up !== null && i.follow_up <= today).map((i) => i.id));
}
