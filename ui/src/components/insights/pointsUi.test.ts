import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import type { Issue } from '../../lib/bindings';
import { listCommands, shortcutFor, type Command, type Ctx } from '../../lib/commands';
import IssueDetail from '../IssueDetail.svelte';
import QuickAdd from '../QuickAdd.svelte';
import General from '../settings/General.svelte';
import { mountSettings, pick, press, toggle } from '../settings/testing';
import PointsRow from './PointsRow.svelte';

const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};

const settle = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
let api: ReturnType<typeof createMockApi>;
let app: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  api = createMockApi();
  setApi(api as unknown as Api);
});
afterEach(() => {
  if (app) void unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const row = () => document.querySelector<HTMLElement>('[aria-label="Points"]');
const issue = (points: number | null) => ({ id: 1, key: 'ABC-1', points }) as unknown as Issue;
async function mountRow(points: number | null = null) {
  const save = vi.fn();
  app = mount(PointsRow, { target: document.body, props: { issue: issue(points), save, openRow: 'points' } });
  await settle();
  return save;
}
const option = (text: string) => [...document.querySelectorAll<HTMLElement>('[aria-label="Points"] [role=option]')].find((o) => o.textContent?.trim().endsWith(text))!;

describe('Points row', () => {
  it('shows the value, or None', async () => {
    await mountRow(8);
    expect(row()!.textContent).toContain('8');
    void unmount(app!);
    document.body.innerHTML = '';
    await mountRow(null);
    expect(row()!.textContent).toContain('None');
  });
  it('picks from the scale', async () => {
    const save = await mountRow();
    expect([...document.querySelectorAll('[aria-label="Points"] [role=option]')].map((o) => o.textContent?.trim())).toEqual(['No points', '0', '1', '2', '3', '5', '8', '13', '21']);
    option('5').click();
    expect(save).toHaveBeenCalledWith({ points: 5 });
  });
  it('uses the chosen scale', async () => {
    await api.settings.set('point_scale', 'tshirt');
    await mountRow();
    expect([...document.querySelectorAll('[aria-label="Points"] [role=option]')].map((o) => o.textContent?.trim())).toEqual(['No points', 'XS', 'S', 'M', 'L', 'XL']);
  });
  it('clears with No points', async () => {
    const save = await mountRow(3);
    option('No points').click();
    expect(save).toHaveBeenCalledWith({ points: null });
  });
  it('takes a free number, to one decimal, and refuses a negative one', async () => {
    const save = await mountRow();
    const input = document.querySelector<HTMLInputElement>('[aria-label="Points value"]')!;
    const submit = () => document.querySelector('form')!.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    input.value = '3.46';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    submit();
    expect(save).toHaveBeenLastCalledWith({ points: 3.5 });
    save.mockClear();
    input.value = '-2';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    submit();
    expect(save).not.toHaveBeenCalled();
  });
  it.each(['time', 'none'])('is hidden when estimation is %s', async (mode) => {
    await api.settings.set('estimation', mode);
    await mountRow();
    expect(row()).toBeNull();
  });
  it('is shown when estimation is points', async () => {
    await api.settings.set('estimation', 'points');
    await mountRow();
    expect(row()).not.toBeNull();
  });
  it('sits in the issue detail properties', async () => {
    const i = await api.issues.create({ title: 'Detail subject' });
    app = mount(IssueDetail, { target: document.body, props: { params: { key: i.key } } });
    await settle();
    expect(row()).not.toBeNull();
  });
});

describe('QuickAdd points chip', () => {
  const withPoints = (a: ReturnType<typeof createMockApi>) => {
    const parse = a.issues.parseQuickadd;
    (a.issues as unknown as { parseQuickadd: unknown }).parseQuickadd = async (t: string) => ({ ...(await parse(t)), points: /3pt/.test(t) ? 3 : null });
  };
  async function type(text: string) {
    app = mount(QuickAdd, { target: document.body, props: { onclose: () => {} } });
    await settle();
    const input = document.querySelector<HTMLInputElement>('input[data-quickadd]')!;
    input.value = text;
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    return [...document.querySelectorAll('[data-chip]')].map((c) => c.textContent?.trim());
  }
  it('shows 3 pts for 3pt', async () => {
    withPoints(api);
    expect(await type('Fix login 3pt')).toContain('3 pts');
  });
  it('shows no chip when estimation is time', async () => {
    withPoints(api);
    await api.settings.set('estimation', 'time');
    expect(await type('Fix login 3pt')).not.toContain('3 pts');
  });
});

describe('General estimation settings', () => {
  it('saves estimation, scale and the two chart switches', async () => {
    const { target, api: a } = await mountSettings(General);
    await pick(target, 'Estimation', 'Points');
    await pick(target, 'Point scale', 'T-shirt (XS to XL)');
    await toggle(target, 'Sidebar sparklines');
    await press(target, 'Save');
    expect(await a.settings.get('estimation')).toBe('points');
    expect(await a.settings.get('point_scale')).toBe('tshirt');
    expect(await a.settings.get('sidebar_sparklines')).toBe(true);
    expect(await a.settings.get('insights_charts')).toBe(true);
  });
  it('shows the stored choices', async () => {
    const { target } = await mountSettings(General, { settings: { get: async (k: unknown) => (k === 'estimation' ? 'time' : null) } });
    expect(target.querySelector('[aria-label="Estimation"]')!.textContent).toContain('Time');
  });
});

describe('commands', () => {
  const cmd = (id: string) => listCommands({} as Ctx, { all: true }).find((c) => c.id === id) as Command;
  const ctx = (route = '/', selection: number[] = []) => ({ route, selection, focusedIssueId: null, openIssueId: null, api: api as unknown as Ctx['api'], navigate: () => {} }) as Ctx;
  it('registers view.insights on views and projects with a key', () => {
    expect(cmd('view.insights')).toBeTruthy();
    expect(shortcutFor('view.insights')).toBe('shift+i');
    expect(cmd('view.insights').when!(ctx('/view/3'))).toBe(true);
    expect(cmd('view.insights').when!(ctx('/project/web'))).toBe(true);
    expect(cmd('view.insights').when!(ctx('/inbox'))).toBe(false);
  });
  it('view.insights asks the page to toggle the panel', () => {
    const got = vi.fn();
    addEventListener('command:insights.toggle', got);
    cmd('view.insights').run!(ctx('/view/3'));
    removeEventListener('command:insights.toggle', got);
    expect(got).toHaveBeenCalledTimes(1);
  });
  it('issue.points lists the scale and applies one bulk patch', async () => {
    const bulk = vi.fn(async () => null);
    (api.issues as unknown as { bulkUpdate: unknown }).bulkUpdate = bulk;
    const page = cmd('issue.points').page!(ctx('/', [1, 2]));
    const items = await page.items;
    expect(items.map((i) => i.label)).toEqual(['0', '1', '2', '3', '5', '8', '13', '21', 'No points']);
    await items[4].run!();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { points: 5 } }));
    await page.parse!('2.5')!.run();
    expect(bulk).toHaveBeenLastCalledWith([1, 2], expect.objectContaining({ patch: { points: 2.5 } }));
    expect(page.parse!('lots')).toBeNull();
  });
});
