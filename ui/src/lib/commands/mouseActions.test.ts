import { describe, expect, it, vi } from 'vitest';
import { listCommands, type Ctx } from './index';

// Every id the screens listen for: each must be a command, offered where the action exists, and fire its event.
const ctx = (route: string, over: Partial<Ctx> = {}): Ctx => ({
  route, layout: 'list', selection: [], focusedIssueId: 1, openIssueId: null, api: {} as Ctx['api'], navigate: () => {}, ...over,
});
const IDS: [string, string][] = [
  ['view.group.open', '/view/1'], ['view.sort.open', '/view/1'], ['view.display.open', '/view/1'], ['view.filter.add', '/view/1'],
  ['view.rename', '/view/1'], ['view.duplicate', '/view/1'], ['view.delete', '/view/1'], ['view.pin', '/view/1'],
  ['group.collapse', '/view/1'], ['group.collapse.all', '/view/1'], ['group.add', '/view/1'],
  ['issue.description.edit', '/issue/ABC-1'], ['issue.comment.focus', '/issue/ABC-1'], ['issue.subissue.add', '/issue/ABC-1'],
  ['issue.relation.add', '/issue/ABC-1'], ['issue.event.add', '/issue/ABC-1'],
  ['event.edit', '/event/1'], ['event.delete', '/event/1'], ['event.link', '/event/1'], ['event.new', '/events'],
  ['event.open', '/events'], ['event.edit', '/events'], ['event.delete', '/events'], ['event.link', '/events'],
  ['project.rename', '/project/web'], ['project.description.edit', '/project/web'], ['milestone.new', '/project/web'],
  ['decision.answer', '/decisions'], ['decision.withdraw', '/decisions'],
  ['icon.change', '/view/1'], ['icon.change', '/project/web'], ['cycle.move_unfinished', '/cycles'], ['settings.save', '/settings/general'],
];

describe('a command for every mouse action', () => {
  it.each(IDS)('%s is registered, labelled, and dispatches command:%s', (id, route) => {
    const c = listCommands(ctx(route), { all: true }).find((x) => x.id === id);
    expect(c, `no command ${id}`).toBeDefined();
    expect(c!.label && c!.icon && (c!.shortcut ?? c!.noShortcutReason)).toBeTruthy();
    expect(!c!.when || c!.when(ctx(route)), `${id} hidden on ${route}`).toBe(true);
    const seen = vi.fn();
    addEventListener(`command:${id}`, seen);
    void c!.run?.(ctx(route));
    removeEventListener(`command:${id}`, seen);
    expect(seen).toHaveBeenCalledOnce();
  });
});
