import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { listCommands, type Ctx } from './index';

// Every "New ..." button and page shortcut has a command; each goes to the page that owns the form, then fires
// `command:<event>` for that page to open it.
const table: [id: string, route: string, event: string][] = [
  ['cycle.new', '/cycles', 'command:cycle.new'],
  ['project.new', '/projects', 'command:project.new'],
  ['view.new', '/views', 'command:view.new'],
  ['label.new', '/settings/labels', 'command:label.new'],
  ['status.new', '/settings/statuses', 'command:status.new'],
  ['template.new', '/settings/templates', 'command:template.new'],
  ['import.csv', '/settings/import', 'command:import.csv'],
];

let navigate: ReturnType<typeof vi.fn>;
const ctx = (route: string): Ctx => ({ route, selection: [], focusedIssueId: null, openIssueId: null, api: {} as Ctx['api'], navigate });
const cmd = (id: string) => listCommands(ctx('/'), { all: true }).find((c) => c.id === id)!;

beforeEach(() => {
  navigate = vi.fn();
  vi.useFakeTimers();
});
afterEach(() => vi.useRealTimers());

describe('create and open commands', () => {
  it.each(table)('%s opens its page and then its form', async (id, route, event) => {
    const heard = vi.fn();
    addEventListener(event, heard);
    const c = cmd(id);
    expect(c.label, id).toBeTruthy();
    expect(c.noShortcutReason ?? c.shortcut, id).toBeTruthy();
    await c.run!(ctx('/'));
    expect(navigate).toHaveBeenCalledWith(route);
    expect(heard).not.toHaveBeenCalled();
    vi.runAllTimers();
    expect(heard).toHaveBeenCalledTimes(1);
    removeEventListener(event, heard);
  });

  it('on the page already, no navigation and the form opens at once', async () => {
    const heard = vi.fn();
    addEventListener('command:cycle.new', heard);
    await cmd('cycle.new').run!(ctx('/cycles'));
    expect(navigate).not.toHaveBeenCalled();
    expect(heard).toHaveBeenCalledTimes(1);
    removeEventListener('command:cycle.new', heard);
  });

  it('go to Pinned and reload exist', async () => {
    await cmd('nav.pinned').run!(ctx('/'));
    expect(navigate).toHaveBeenCalledWith('/pinned');
    const reload = vi.fn();
    vi.stubGlobal('location', { ...location, reload });
    await cmd('app.reload').run!(ctx('/'));
    expect(reload).toHaveBeenCalled();
    vi.unstubAllGlobals();
  });
});
