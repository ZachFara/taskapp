import { describe, expect, it } from 'vitest';
import { defineCommand, duplicateBindings, groups, labelFor, listCommands, shortcutFor, type Ctx } from './index';

const ctx = (over: Partial<Ctx> = {}): Ctx => ({
  route: '/',
  selection: [],
  focusedIssueId: null,
  openIssueId: null,
  api: {} as Ctx['api'],
  navigate: () => {},
  ...over,
});

describe('registry', () => {
  it('every registered command has label, icon and a shortcut or a reason', () => {
    const all = listCommands(ctx({ selection: [1], openIssueId: 1 }), { all: true });
    expect(all.length).toBeGreaterThan(0);
    for (const c of all) {
      expect(c.label, c.id).toBeTruthy();
      expect(c.icon, c.id).toBeTruthy();
      expect(c.shortcut ?? c.noShortcutReason, c.id).toBeTruthy();
    }
  });

  it('ships the example commands with their keys', () => {
    expect(shortcutFor('issue.create')).toBe('c');
    expect(shortcutFor('palette.open')).toBe('mod+k');
    expect(shortcutFor('help.shortcuts')).toBe('?');
    expect(shortcutFor('nav.search')).toBe('/');
    expect(labelFor('issue.create')).toBeTruthy();
    expect(groups().flatMap((g) => g.commands.map((c) => c.id))).toContain('help.shortcuts');
  });

  it('no two commands share a key in the same scope', () => {
    expect(duplicateBindings(listCommands(ctx(), { all: true }))).toEqual([]);
  });

  it('duplicateBindings reports a clash and tolerates different scopes', () => {
    const mk = (id: string, shortcut: string, scope?: string) => ({ id, shortcut, scope }) as never;
    expect(duplicateBindings([mk('a', 'x'), mk('b', 'x')])).toEqual(['x']);
    expect(duplicateBindings([mk('a', 'x', 'list'), mk('b', 'x', 'board')])).toEqual([]);
  });

  it('when hides a command and contextual commands come first', () => {
    defineCommand({ id: 't.plain', label: 'Plain', icon: 'x', group: 'T', noShortcutReason: 'test', run: () => {} });
    defineCommand({
      id: 't.ctx', label: 'Needs selection', icon: 'x', group: 'T', noShortcutReason: 'test', run: () => {},
      when: (c) => c.selection.length > 0,
    });
    expect(listCommands(ctx()).map((c) => c.id)).not.toContain('t.ctx');
    const ids = listCommands(ctx({ selection: [1] })).map((c) => c.id);
    expect(ids.indexOf('t.ctx')).toBeLessThan(ids.indexOf('t.plain'));
    expect(ids[0]).toBe('t.ctx');
  });

  it('a command can open a nested page', () => {
    const c = defineCommand({
      id: 't.page', label: 'Set thing', icon: 'x', group: 'T', noShortcutReason: 'test',
      page: () => ({ title: 'Thing', items: [{ id: 'a', label: 'A', run: () => {} }] }),
    });
    expect(c.page!(ctx()).items[0].label).toBe('A');
  });
});
