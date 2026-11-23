import { expect, it } from 'vitest';
import { keycaps } from './glyphs';
import { keysOf, listCommands, type Ctx } from './index';

const all = listCommands({ route: '/', selection: [], focusedIssueId: null, openIssueId: null } as unknown as Ctx, { all: true });

it('no registered shortcut renders a key as a raw lowercase word', () => {
  const raw = all.flatMap((c) => keysOf(c).flatMap((k) => keycaps(k, 'MacIntel')).map((cap) => [c.id, cap])).filter(([, cap]) => /^[a-z]{2,}$/.test(cap));
  expect(raw).toEqual([]);
});
