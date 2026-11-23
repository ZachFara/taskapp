import { api } from './api';
import { commandById, type Ctx } from './commands/registry';
import { navigate } from '../app/routes';
import type { Kind, Target } from './rowActions';
import type { RowAction } from '../ui/RowActions.svelte';

/** The one open row dialog (rename or delete) and the entity whose icon picker is open. */
export const rowUi = $state<{ dialog: { mode: 'rename' | 'delete'; target: Target } | null; picking: string | null }>({ dialog: null, picking: null });

export const askRename = (target: Target) => (rowUi.dialog = { mode: 'rename', target });
export const askDelete = (target: Target) => (rowUi.dialog = { mode: 'delete', target });

export const pickKey = (t: Target) => `${t.kind}:${t.id}`;

/**
 * Runs a registry command for a row. `target` is the row's entity, or an issue id for issue commands. Commands that
 * have a page open it, as their key does.
 */
export function runRow(id: string, target: Target | number) {
  const c = commandById(id);
  if (!c) return;
  const ctx: Ctx = {
    route: location.pathname,
    selection: [],
    focusedIssueId: typeof target === 'number' ? target : null,
    openIssueId: null,
    api,
    navigate,
  };
  if (c.run) void c.run(ctx, 1, target);
  else dispatchEvent(new CustomEvent('command:palette.open', { detail: { page: c.id } }));
}

const VERB: Record<string, string> = { rename: 'Rename', duplicate: 'Duplicate', pin: 'Pin', change: 'Change icon of', delete: 'Delete' };
const ACTIONS: Record<Kind, string[]> = {
  view: ['view.rename', 'view.duplicate', 'view.pin', 'icon.change', 'view.delete'],
  project: ['project.rename', 'project.pin', 'icon.change', 'project.delete'],
  cycle: ['cycle.rename', 'cycle.pin', 'icon.change', 'cycle.delete'],
  label: ['label.rename', 'icon.change', 'label.delete'],
  template: ['template.rename', 'icon.change', 'template.delete'],
};

/** The actions a row of this entity offers, each one a registry command run with the row as its target. */
export function rowActs(t: Target, pinned = false): RowAction[] {
  return ACTIONS[t.kind].map((id) => {
    const verb = id.split('.')[1];
    const label = verb === 'pin' ? `${pinned ? 'Unpin' : 'Pin'} ${t.name}` : `${VERB[verb]} ${t.name}`;
    return { id, label, icon: commandById(id)?.icon ?? 'more', danger: verb === 'delete', run: () => runRow(id, t) };
  });
}
