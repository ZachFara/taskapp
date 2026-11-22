import type { Api } from '../api';

/** What the open view shows; the view publishes it (`selection.layout`) so keys can be scoped to it. */
export type Layout = 'list' | 'table' | 'board' | 'calendar' | 'timeline';
/**
 * Where a key is live. `global` everywhere; a layout only while that layout is shown; `detail` while an issue is open
 * (page or panel); `list-edit` while a list row is being renamed. The more specific scope wins an identical key.
 */
export type Scope = 'global' | Layout | 'detail' | 'list-edit';

export type Ctx = {
  route: string;
  /** Undefined when the screen has not said (layout scopes are then live and equal to `global` in rank); null when no view is shown. */
  layout?: Layout | null;
  selection: number[];
  focusedIssueId: number | null;
  openIssueId: number | null;
  api: Api;
  navigate: (path: string) => void;
};
/**
 * A second palette step. `multi`: several items may be checked (each shows `checked`) before the page is confirmed,
 * and `run` then receives how many were chosen.
 */
export type PageItem = {
  id: string;
  label: string;
  icon?: string;
  keywords?: string[];
  checked?: boolean | (() => boolean);
  run: (count?: number) => void | Promise<void>;
};
export type CommandPage = {
  title: string;
  multi?: boolean;
  /** Free text typed into the palette becomes one preview row that runs when chosen; null when it is no value. */
  parse?: (text: string) => { label: string; run: () => void | Promise<void> } | null;
  /** Listed now, or a Promise while the choices load (the palette shows skeleton rows until it settles). */
  items: PageItem[] | Promise<PageItem[]>;
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
  /** Where the key is live (default `global`); see `Scope`. `when` still decides case by case. */
  scope?: Scope;
  when?: (ctx: Ctx) => boolean;
  /** `count` is the number typed before the key (`5 j`), 1 when none. */
  run?: (ctx: Ctx, count?: number) => void | Promise<void>;
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
  const list = [...commands.values()].filter((c) => opts.all || (inScope(c, ctx) && (!c.when || c.when(ctx))));
  return [...list.filter((c) => c.when), ...list.filter((c) => !c.when)];
}

export const labelFor = (id: string) => commands.get(id)?.label;
export const shortcutFor = (id: string) => keysOf(commands.get(id) ?? { id, shortcut: [] })[0];

/** Commands that have a key, grouped for the cheat sheet. */
export function groups() {
  const by = new Map<string, Command[]>();
  for (const c of commands.values()) if (keysOf(c).length) by.set(c.group, [...(by.get(c.group) ?? []), c]);
  return [...by].map(([group, list]) => ({ group, commands: list }));
}

// Table commands predate scopes and live in another owner's files; they are table-scoped by their id until they say so.
export const scopeOf = (c: Pick<Command, 'id' | 'scope'>): Scope => c.scope ?? (c.id.startsWith('table.') ? 'table' : 'global');
const RANK: Record<Scope, number> = { global: 0, list: 1, table: 1, board: 1, calendar: 1, timeline: 1, detail: 2, 'list-edit': 3 };
/** Higher wins when two live commands take the same key. A layout scope does not outrank `global` until the layout is known. */
export const rankOf = (c: Pick<Command, 'id' | 'scope'>, ctx?: Ctx) => {
  const s = scopeOf(c);
  return RANK[s] === 1 && ctx?.layout === undefined && ctx ? 0 : RANK[s];
};

/** Whether a command's scope is live in `ctx`. A layout scope is live everywhere while the layout is unknown (the command's own `when` still applies). */
export function inScope(c: Pick<Command, 'id' | 'scope'>, ctx: Ctx) {
  const s = scopeOf(c);
  if (s === 'global') return true;
  if (s === 'detail') return !!ctx.route?.startsWith('/issue/') || (ctx.openIssueId ?? null) !== null;
  const layout = s === 'list-edit' ? 'list' : s;
  return ctx.layout === undefined || ctx.layout === layout;
}

/** Keys taken off a command by a ruling that outranks the file that defines it. */
const unbound = new Map<string, Set<string>>();
export const unbind = (id: string, ...keys: string[]) => void unbound.set(id, new Set([...(unbound.get(id) ?? []), ...keys]));
/** A command's keys as bound (`shortcut` minus anything `unbind` took away). */
export const keysOf = (c: Pick<Command, 'id' | 'shortcut'>) => [c.shortcut ?? []].flat().filter((k) => !unbound.get(c.id)?.has(k));

/** What a key press can reach in `ctx`: in scope and `when` passing. */
export const activeCommands = (ctx: Ctx) => listCommands(ctx, { all: true }).filter((c) => inScope(c, ctx) && (!c.when || c.when(ctx)));

const split = (k: string) => k.split(/\s+/);
const startsWith = (long: string[], short: string[]) => short.every((x, i) => long[i] === x);

/**
 * Pairs of commands that cannot both work. The same key at the same rank is a clash (the more specific scope wins
 * otherwise); a key that is the start of another's sequence is one at any rank, since it would fire before the
 * sequence could finish. Pass the commands live at one moment.
 */
export function clashes(list: Pick<Command, 'id' | 'shortcut' | 'scope'>[], ctx?: Ctx) {
  const out: { a: string; b: string; keys: string }[] = [];
  list.forEach((a, i) =>
    list.slice(i + 1).forEach((b) => {
      for (const x of keysOf(a))
        for (const y of keysOf(b)) {
          const [p, q] = [split(x), split(y)];
          const same = p.length === q.length && startsWith(p, q);
          const prefix = p.length !== q.length && (p.length < q.length ? startsWith(q, p) : startsWith(p, q));
          if ((same && rankOf(a, ctx) === rankOf(b, ctx)) || prefix) out.push({ a: a.id, b: b.id, keys: same ? x : `${x} | ${y}` });
        }
    }),
  );
  return out;
}

/** Keys bound twice at one level, over every command regardless of when it is live (a stricter, static view). */
export function duplicateBindings(list: Pick<Command, 'id' | 'shortcut' | 'scope'>[]) {
  const seen = new Set<string>();
  const dup = new Set<string>();
  for (const c of list)
    for (const s of keysOf(c)) {
      const k = `${scopeOf(c)}|${s}`;
      if (seen.has(k)) dup.add(s);
      seen.add(k);
    }
  return [...dup];
}
