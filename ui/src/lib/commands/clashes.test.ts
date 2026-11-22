import { afterEach, describe, expect, it } from 'vitest';
import { listState } from '../listState.svelte';
import { activeCommands, clashes, keysOf, type Command, type Ctx, type Layout } from './index';

// Walks every registered command in every state the app can be in and fails when two commands that are live at the
// same moment take the same key, or when one key is the start of another's sequence ("g" and "g g").
const base: Ctx = { layout: null, route: '/', selection: [], focusedIssueId: null, openIssueId: null, api: {} as Ctx['api'], navigate: () => {} };
const layouts: Layout[] = ['list', 'table', 'board', 'calendar', 'timeline'];

const states: [string, Ctx, { editing?: boolean; timeline?: boolean }][] = [
  ...['/', '/my', '/inbox', '/pinned', '/projects', '/cycles', '/views', '/settings/general', '/project/website'].map(
    (route): [string, Ctx, object] => [route, { ...base, route }, {}],
  ),
  ['issue page', { ...base, route: '/issue/ABC-1', openIssueId: 1, focusedIssueId: 1, selection: [1] }, {}],
  ...layouts.flatMap((layout): [string, Ctx, { editing?: boolean; timeline?: boolean }][] => {
    const view = { ...base, route: '/view/1', layout, focusedIssueId: 1, selection: [1] };
    const dom = { timeline: layout === 'timeline' };
    return [
      [`${layout} view`, view, dom],
      [`${layout} view with an issue open`, { ...view, openIssueId: 1 }, dom],
      [`${layout} view with several selected`, { ...view, selection: [1, 2] }, dom],
      ...(layout === 'list' ? [[`list while renaming`, view, { editing: true }] as [string, Ctx, { editing: boolean }]] : []),
    ];
  }),
];

afterEach(() => {
  listState.editing = null;
  listState.ids = [];
  document.body.innerHTML = '';
});

describe('key clashes', () => {
  it.each(states)('no two live commands share a key: %s', (_name, ctx, dom) => {
    listState.ids = [1, 2];
    listState.editing = dom.editing ? 1 : null;
    document.body.innerHTML = dom.timeline ? '<div data-timeline></div>' : '';
    const live = activeCommands(ctx);
    expect(live.length).toBeGreaterThan(0);
    expect(clashes(live, ctx)).toEqual([]);
  });

  it('every shortcut is parseable and unique per command', () => {
    for (const [, ctx] of states) for (const c of activeCommands(ctx)) expect(new Set(keysOf(c)).size, c.id).toBe(keysOf(c).length);
  });

  const mk = (id: string, shortcut: string | string[], scope?: Command['scope'], rank?: never) =>
    ({ id, label: id, icon: 'x', group: 'T', shortcut, scope, rank }) as Command;

  it('the detector reports the same key at the same level, a sequence prefix, and nothing across levels', () => {
    expect(clashes([mk('a', 'x'), mk('b', 'x')])).toEqual([{ a: 'a', b: 'b', keys: 'x' }]);
    expect(clashes([mk('a', 'g'), mk('b', 'g g')])).toEqual([{ a: 'a', b: 'b', keys: 'g | g g' }]);
    expect(clashes([mk('a', 'g i'), mk('b', 'g g')])).toEqual([]);
    // the more specific level wins an identical key, so that is a deliberate shadow, not a clash
    expect(clashes([mk('a', 'x', 'detail'), mk('b', 'x')])).toEqual([]);
    expect(clashes([mk('a', 'x', 'board'), mk('b', 'x', 'detail')])).toEqual([]);
    // but a short key must not swallow the first key of a longer sequence at another level
    expect(clashes([mk('a', 'g', 'detail'), mk('b', 'g g')])).toEqual([{ a: 'a', b: 'b', keys: 'g | g g' }]);
  });
});
