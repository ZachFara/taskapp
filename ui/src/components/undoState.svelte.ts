import type { ErrorPayload } from '../lib/bindings';
import { type Api, api } from '../lib/api';
import { refreshAll } from '../lib/stores.svelte';

export type UndoMsg = {
  label: string;
  tone: 'info' | 'error';
  action?: { run: 'undo' | 'redo'; text: string };
};

// Shared by the toast host and the undo/redo commands, so both see one message at a time.
export const undoToast = $state<{ value: UndoMsg | null }>({ value: null });

let timer: ReturnType<typeof setTimeout> | undefined;
function show(msg: UndoMsg | null) {
  clearTimeout(timer);
  undoToast.value = msg;
  if (msg) timer = setTimeout(() => (undoToast.value = null), 6_000);
}

export const dismiss = () => show(null);

// Named by the app after any write; the step label comes from the backend, not from the write.
export async function onWrote() {
  const { undo } = await api.history.undoStatus();
  if (undo) show({ label: undo, tone: 'info', action: { run: 'undo', text: 'Undo (cmd+Z)' } });
}

const message = (e: unknown, fallback: string) => (e as Partial<ErrorPayload>)?.message ?? fallback;

export async function runUndo(a: Api) {
  try {
    await a.history.undo();
    await refreshAll();
    show({ label: 'Undone', tone: 'info', action: { run: 'redo', text: 'Redo' } });
  } catch (e) {
    show({ label: message(e, 'Undo failed'), tone: 'error' });
  }
}

export async function runRedo(a: Api) {
  try {
    await a.history.redo();
    await refreshAll();
    show({ label: 'Redone', tone: 'info', action: { run: 'undo', text: 'Undo (cmd+Z)' } });
  } catch (e) {
    show({ label: message(e, 'Redo failed'), tone: 'error' });
  }
}
