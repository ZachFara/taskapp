import { api } from './api';
import { isOn } from './features.svelte';

// Ids of issues past their status's nudge age, for the row marker. Reloaded when a list opens and after each write.
export const stale = $state({ ids: new Set<number>() });

export async function refreshStale() {
  stale.ids = isOn('stale') ? new Set((await api.stale.list()).map((s) => s.issue.id)) : new Set();
}
