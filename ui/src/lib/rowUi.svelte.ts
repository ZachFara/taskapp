import { api } from './api';
import { commandById, type Ctx } from './commands/registry';
import { navigate } from '../app/routes';
import { SECTION, type Kind, type Move, type Target } from './rowActions';
import { choices } from './folderTree';
import { folders } from './stores.svelte';
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
export function runRow(id: string, target: unknown) {
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
  // A refused action tells the sidebar, which shows the reason.
  const failed = (e: unknown) => dispatchEvent(new CustomEvent('rows:failed', { detail: (e as { message?: string }).message ?? String(e) }));
  if (c.run) void Promise.resolve(c.run(ctx, 1, target)).catch(failed);
  else dispatchEvent(new CustomEvent('command:palette.open', { detail: { page: c.id } }));
}

const VERB: Record<string, string> = { rename: 'Rename', duplicate: 'Duplicate', pin: 'Pin', change: 'Change icon of', delete: 'Delete', unpin: 'Unpin', newInside: 'New folder inside', copyLink: 'Copy link to' };
const ACTIONS: Record<Kind, string[]> = {
  view: ['view.rename', 'view.duplicate', 'view.pin', 'view.copyLink', 'icon.change', 'view.delete'],
  project: ['project.rename', 'project.pin', 'project.copyLink', 'icon.change', 'project.delete'],
  cycle: ['cycle.rename', 'cycle.pin', 'cycle.copyLink', 'icon.change', 'cycle.delete'],
  label: ['label.rename', 'icon.change', 'label.delete'],
  template: ['template.rename', 'icon.change', 'template.delete'],
  folder: ['folder.newInside', 'folder.rename', 'folder.pin', 'folder.delete'],
  pin: ['pin.unpin'],
};

/** The places a row can go: the top level, every folder of its section (indented by depth), or a new folder. */
function moveMenu(t: Target): RowAction {
  const section = t.kind === 'folder' ? folders.find((f) => f.id === t.id)?.section : SECTION[t.kind];
  const go = (folder: Move['folder']) => () => runRow('folder.move', { ...t, folder } satisfies Move);
  const sub = [
    { id: 'top', label: 'Top level', icon: 'layers', run: go(null) },
    ...(section ? choices(folders, section, t.kind === 'folder' ? t.id : undefined) : []).map((c) => ({ id: `f${c.id}`, label: c.name, icon: 'folder', indent: c.depth, run: go(c.id) })),
    { id: 'new', label: 'New folder…', icon: 'plus', run: go('new') },
  ];
  return { id: 'folder.move', label: 'Move to folder', icon: 'folder', run: () => {}, sub };
}

/** The actions a row of this entity offers, each one a registry command run with the row as its target. */
export function rowActs(t: Target, pinned = false): RowAction[] {
  const acts = ACTIONS[t.kind].map((id) => {
    const verb = id.split('.')[1];
    const label = verb === 'pin' ? `${pinned ? 'Unpin' : 'Pin'} ${t.name}` : `${VERB[verb]} ${t.name}`;
    return { id, label, icon: commandById(id)?.icon ?? 'more', danger: verb === 'delete', run: () => runRow(id, t) };
  });
  // Before the last action, which is the destructive one.
  if ((t.kind === 'folder' || SECTION[t.kind])) acts.splice(acts.length - 1, 0, moveMenu(t));
  return acts;
}
