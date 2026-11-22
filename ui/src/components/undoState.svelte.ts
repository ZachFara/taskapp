import type { ErrorPayload } from '../lib/bindings';
import { type Api, api } from '../lib/api';
import { refreshAll } from '../lib/stores.svelte';

export type UndoMsg = {
  label: string;
  tone: 'info' | 'success' | 'error';
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

// The backend labels a step with the command that made it. Known commands get a sentence; a label that is
// already prose ("Created ABC-25") passes through; any other command name is sentence-cased.
const STEPS: Record<string, string> = {
  issues_create: 'Created issue', issues_update: 'Updated issue', issues_bulk_update: 'Updated issues', issues_archive: 'Archived issue',
  issues_delete: 'Deleted issue', properties_set_value: 'Set property value', comments_add: 'Added comment', pins_pin: 'Pinned', pins_unpin: 'Unpinned',
  views_save: 'Changed view', views_delete: 'Deleted view', views_create_from_template: 'Created view from template',
  views_save_as_template: 'Saved view as template', views_reset_to_template: 'Reset view to template',
  view_templates_create: 'Created view template', view_templates_update: 'Changed view template', view_templates_delete: 'Deleted view template',
};
export function stepLabel(raw: string): string {
  if (!/^[a-z]+(_[a-z]+)+$/.test(raw)) return raw;
  const words = raw.replace(/_/g, ' ');
  return STEPS[raw] ?? words[0].toUpperCase() + words.slice(1);
}

// Named by the app after any write; the step label comes from the backend, not from the write.
// A saved view's layout, grouping, sorting and filters change as the user looks around; that is not a step to announce
// (`u` still undoes it).
const QUIET = new Set(['views_save']);

export async function onWrote() {
  const { undo } = await api.history.undoStatus();
  if (undo && !QUIET.has(undo)) show({ label: stepLabel(undo), tone: 'success', action: { run: 'undo', text: 'Undo' } });
}

const message = (e: unknown, fallback: string) => (e as Partial<ErrorPayload>)?.message ?? fallback;

export async function runUndo(a: Api) {
  try {
    await a.history.undo();
    await refreshAll();
    show({ label: 'Undone', tone: 'success', action: { run: 'redo', text: 'Redo' } });
  } catch (e) {
    show({ label: message(e, 'Undo failed'), tone: 'error' });
  }
}

export async function runRedo(a: Api) {
  try {
    await a.history.redo();
    await refreshAll();
    show({ label: 'Redone', tone: 'success', action: { run: 'undo', text: 'Undo' } });
  } catch (e) {
    show({ label: message(e, 'Redo failed'), tone: 'error' });
  }
}
