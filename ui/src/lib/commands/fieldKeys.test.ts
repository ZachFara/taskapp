import { describe, expect, it, vi } from 'vitest';
import { listCommands, shortcutFor, type Command, type Ctx } from './index';

// The Display panel listens for these events; the commands only decide when a key fires them.
describe('field order commands', () => {
  const find = (id: string) => listCommands({} as Ctx, { all: true }).find((c) => c.id === id) as Command;
  it.each([
    ['view.field.up', 'Move field up', 'alt+arrowup'],
    ['view.field.down', 'Move field down', 'alt+arrowdown'],
  ])('%s is registered and dispatches its event', (id, label, key) => {
    const c = find(id);
    expect([c.label, c.group, shortcutFor(id)]).toEqual([label, 'View', key]);
    const got = vi.fn();
    addEventListener(`command:${id}`, got);
    void c.run!({} as Ctx, 1);
    removeEventListener(`command:${id}`, got);
    expect(got).toHaveBeenCalledOnce();
  });
});
