import { api } from './api';
import type { ShutdownState } from './bindings';

/** The shutdown checklist: whether it shows, what is picked for tomorrow, and the loose end being typed. The panel and its commands share it. */
export const shutdown = $state<{ open: boolean; state: ShutdownState | null; picked: number[]; loose: string }>({ open: false, state: null, picked: [], loose: '' });

async function load() {
  shutdown.state = await api.shutdown.state();
  shutdown.picked = shutdown.state.tomorrow.map((i) => i.id);
}

export async function openShutdown() {
  shutdown.open = true;
  await load();
}

export async function saveTomorrow() {
  shutdown.state = await api.shutdown.setTomorrow(shutdown.picked);
}

export async function captureLooseEnd() {
  const text = shutdown.loose.trim();
  if (!text) return;
  await api.issues.createFromQuickadd(text);
  shutdown.loose = '';
  await load();
}

export async function finishShutdown() {
  shutdown.state = await api.shutdown.complete();
}
