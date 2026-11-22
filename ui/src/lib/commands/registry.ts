import type { Api } from '../api';

export type Ctx = {
  route: string;
  selection: number[];
  focusedIssueId: number | null;
  openIssueId: number | null;
  api: Api;
  navigate: (path: string) => void;
};
export type CommandPage = {
  title: string;
  items: { id: string; label: string; icon?: string; keywords?: string[]; run: () => void | Promise<void> }[];
};
export type Command = {
  id: string;
  label: string;
  icon: string;
  group: string;
  shortcut?: string | string[];
  noShortcutReason?: string;
  /** Fires from inside a text field too; only meaningful for `mod+` combos. */
  global?: boolean;
  /** Names the key scope for the duplicate-binding check; commands with a `when` and no scope never clash. */
  scope?: string;
  when?: (ctx: Ctx) => boolean;
  run?: (ctx: Ctx) => void | Promise<void>;
  page?: (ctx: Ctx) => CommandPage;
  keywords?: string[];
};

const commands = new Map<string, Command>();

/** Registers a command; call it from one file under `defs/`. */
export function defineCommand(c: Command) {
  if (!c.run && !c.page) throw new Error(`command ${c.id} needs run or page`);
  commands.set(c.id, c);
  return c;
}

/** Commands whose `when` passes (all of them with `all`), contextual ones first. */
export function listCommands(ctx: Ctx, opts: { all?: boolean } = {}) {
  const list = [...commands.values()].filter((c) => opts.all || !c.when || c.when(ctx));
  return [...list.filter((c) => c.when), ...list.filter((c) => !c.when)];
}

export const labelFor = (id: string) => commands.get(id)?.label;
export const shortcutFor = (id: string) => [commands.get(id)?.shortcut ?? []].flat()[0];

/** Commands that have a key, grouped for the cheat sheet. */
export function groups() {
  const by = new Map<string, Command[]>();
  for (const c of commands.values()) if (c.shortcut) by.set(c.group, [...(by.get(c.group) ?? []), c]);
  return [...by].map(([group, list]) => ({ group, commands: list }));
}

/** Keys bound twice in the same scope (a command without `when` or `scope` is in the global scope). */
export function duplicateBindings(list: Pick<Command, 'id' | 'shortcut' | 'scope' | 'when'>[]) {
  const seen = new Set<string>();
  const dup = new Set<string>();
  for (const c of list)
    for (const s of [c.shortcut ?? []].flat()) {
      const k = `${c.scope ?? (c.when ? c.id : '')}|${s}`;
      if (seen.has(k)) dup.add(s);
      seen.add(k);
    }
  return [...dup];
}
