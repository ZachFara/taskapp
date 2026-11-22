import { expect, it } from 'vitest';
import { listCommands, type Ctx } from './index';

// One icon per concept: the same thing looks the same in the sidebar, the command menu and its settings page.
const all = listCommands({ route: '/', selection: [], focusedIssueId: null, openIssueId: null } as unknown as Ctx, { all: true });
const icon = (id: string) => all.find((c) => c.id === id)!.icon;

it.each([
  [['nav.projects', 'project.new', 'settings.projects'], 'project'],
  [['nav.views', 'view.new'], 'layers'],
  [['nav.cycles', 'cycle.new', 'settings.cycles'], 'cycle'],
  [['nav.settings', 'settings.general'], 'settings'],
  [['label.new', 'settings.labels'], 'tag'],
  [['template.new', 'settings.templates'], 'copy'],
])('%j all use %s', (ids, want) => ids.forEach((id) => expect(icon(id), id).toBe(want)));

it('different concepts do not share an icon', () => {
  const pairs = [['settings.properties', 'nav.views'], ['settings.templates', 'settings.labels'], ['settings.properties', 'nav.projects'], ['nav.design', 'nav.views']];
  for (const [a, b] of pairs) if (all.some((c) => c.id === a)) expect(icon(a), `${a} vs ${b}`).not.toBe(icon(b));
});
