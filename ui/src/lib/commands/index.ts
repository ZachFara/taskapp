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
 * Duplicate key in one scope fails `registry.test.ts`.
 *
 * Read: `listCommands(ctx)`, `shortcutFor(id)`, `labelFor(id)`, `groups()`, `rank(query, items, recent)` (match.ts).
 * Root: `attachKeys(() => ctx)` once. Selection store: `selection.svelte.ts`.
 */
// Every file under defs/ registers one command when imported.
import.meta.glob('./defs/**/*.ts', { eager: true });
export * from './registry';
import { createKeys } from './keys';
import { listCommands, type Command, type Ctx } from './registry';

/** Binds every command's shortcut; call once at the app root with a getter for the live context. */
export function attachKeys(ctx: () => Ctx) {
  return createKeys(() =>
    listCommands(ctx(), { all: true }).flatMap((c: Command) =>
      [c.shortcut ?? []].flat().map((seq) => ({
        seq,
        global: c.global,
        run: () => {
          const x = ctx();
          if (!c.when || c.when(x)) void c.run?.(x);
        },
      })),
    ),
  );
}
