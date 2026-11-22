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
