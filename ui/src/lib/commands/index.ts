/**
 * One registry feeds the command menu, keyboard, tooltips and cheat sheet.
 *
 * Add a command: create ONE file `defs/<id>.ts` that calls `defineCommand({ id, label, icon, group, shortcut, run })`.
 * It is picked up by `import.meta.glob`; never edit a shared list. A command has `run` or `page` (a nested list).
 * A command without `shortcut` must give `noShortcutReason`. `when(ctx)` hides it; `scope` names the key scope.
 *
 * Key syntax: `c`, `?`, `G`, `shift+j`, `mod+k` (cmd or ctrl), `alt+x`, `escape`, `enter`; sequences are space
 * separated (`g i`, 1s timeout, Esc cancels); a number typed first is a count (`5 j`, handler gets it).
 * Plain keys never fire in input, textarea, select or contenteditable; `escape` and `global: true` `mod+` combos do.
 * The matched key is `preventDefault`ed so it is never typed into the field it opens. IME composition is ignored.
 * Scopes: `scope` says where a key is live (`global`, a layout, `detail`, `list-edit`). When two live commands take the
 * same key the more specific scope wins (list-edit > detail > a layout > global), so `detail` keys beat the bulk
 * `issue.*` ones and a layout's keys beat global ones. `Ctx.layout` comes from the view (`selection.setLayout`);
 * until it is set, layout scopes are live and rank like global. `clashes.test.ts` walks every command in every state
 * and fails when two live commands of equal rank share a key or one key starts another's sequence.
 * A key on a command with only a `page` opens the command menu at that page (`command:palette.open`, `detail.page`).
 * A page's `items` may be a Promise while choices load; `parse(text)` turns typed text into one preview row.
 *
 * Read: `listCommands(ctx)`, `shortcutFor(id)`, `labelFor(id)`, `groups()`, `rank(query, items, recent)` (match.ts).
 * Root: `attachKeys(() => ctx)` once. Selection store: `selection.svelte.ts`.
 */
// Every file under defs/ registers one command when imported.
import.meta.glob('./defs/**/*.ts', { eager: true });
export * from './registry';
import { createKeys } from './keys';
import { inScope, keysOf, listCommands, rankOf, type Command, type Ctx } from './registry';

/** A key on a command that only has a page opens the command menu at that page. */
const open = (c: Command, ctx: Ctx, count: number) =>
  c.run ? c.run(ctx, count) : dispatchEvent(new CustomEvent('command:palette.open', { detail: { page: c.id } }));

/** Binds every command's shortcut; call once at the app root with a getter for the live context. */
export function attachKeys(ctx: () => Ctx) {
  return createKeys(() =>
    listCommands(ctx(), { all: true }).flatMap((c: Command) =>
      keysOf(c).map((seq) => ({
        seq,
        global: c.global,
        rank: () => rankOf(c, ctx()),
        // Out of scope or `when` failing: the key falls through to whatever the next scope binds.
        active: () => {
          try {
            return inScope(c, ctx()) && (!c.when || c.when(ctx()));
          } catch {
            return false; // a `when` that throws must not take every other key down
          }
        },
        run: (count: number) => {
          // An open dialog owns the plain keys; only `global` combos such as mod+k reach through it.
          if (!c.global && document.querySelector('[role=dialog]')) return;
          void open(c, ctx(), count);
        },
      })),
    ),
  );
}
