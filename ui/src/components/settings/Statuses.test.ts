import { describe, expect, it, vi } from 'vitest';
import Statuses from './Statuses.svelte';
import { clickLabel, field, mountSettings, press, setField } from './testing';

async function add(target: Element, category: string, name: string) {
  await clickLabel(`Add to ${category}`);
  await setField(target, 'New status name', name);
  await press(target, 'Add status');
}

describe('Statuses', () => {
  it('shows the starting status: the chosen one, else the first', async () => {
    const trigger = (t: Element) => t.querySelector('[aria-label="New issues start in"]')!;
    const first = await mountSettings(Statuses);
    await vi.waitFor(() => expect(trigger(first.target).textContent).toContain('Backlog'));
    document.body.innerHTML = '';
    const chosen = await mountSettings(Statuses, { settings: { get: async (k: string) => (k === 'default_status' ? 3 : null) } });
    const name = (await (chosen.api as unknown as { statuses: { list: () => Promise<{ id: number; name: string }[]> } }).statuses.list()).find((s) => s.id === 3)!.name;
    await vi.waitFor(() => expect(trigger(chosen.target).textContent).toContain(name));
  });

  it('shows every category as a band, in order', async () => {
    const { target } = await mountSettings(Statuses);
    const bands = [...target.querySelectorAll('.band .name')].map((n) => n.textContent);
    expect(bands).toEqual(['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled']);
  });

  it('each status row shows the glyph of its category beside the icon picker', async () => {
    const { target } = await mountSettings(Statuses);
    const cats = [...target.querySelectorAll('.es-row')].map((r) => r.querySelector('svg.glyph[data-category]')?.getAttribute('data-category'));
    expect(cats).toEqual(['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled']);
  });

  it('adds a status to the chosen category once', async () => {
    const { target, calls } = await mountSettings(Statuses);
    await add(target, 'Started', 'Review');
    expect(calls.filter((c) => c === 'statuses.create')).toHaveLength(1);
    expect(field(target, 'Name of Review').value).toBe('Review');
  });

  it('renames inline with one call', async () => {
    const { target, calls } = await mountSettings(Statuses);
    await add(target, 'Started', 'Review');
    await setField(target, 'Name of Review', 'QA');
    expect(calls.filter((c) => c === 'statuses.update')).toHaveLength(1);
    expect(field(target, 'Name of QA')).toBeTruthy();
  });

  it('moves a status up with one reorder call and no updates', async () => {
    const { target, calls, api } = await mountSettings(Statuses);
    await add(target, 'Started', 'Aaa');
    await add(target, 'Started', 'Bbb');
    await clickLabel('Move Bbb up');
    expect(calls.filter((c) => c === 'statuses.reorder')).toHaveLength(1);
    expect(calls.filter((c) => c === 'statuses.update')).toHaveLength(0);
    const names = (await api.statuses.list()).map((s) => s.name);
    expect(names.indexOf('Bbb')).toBeLessThan(names.indexOf('Aaa'));
  });

  it('delete needs a target status before it can confirm', async () => {
    const { target, calls, api } = await mountSettings(Statuses);
    await add(target, 'Started', 'Review');
    await clickLabel('Delete Review');
    const confirm = () => [...document.body.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Delete status')!;
    expect(confirm().disabled).toBe(true);
    const other = (await api.statuses.list()).find((s) => s.name !== 'Review')!;
    await clickLabel(`Move issues to ${other.name}`);
    expect(confirm().disabled).toBe(false);
    confirm().click();
    await new Promise((r) => setTimeout(r, 0));
    expect(calls.filter((c) => c === 'statuses.delete')).toHaveLength(1);
    expect((await api.statuses.list()).some((s) => s.name === 'Review')).toBe(false);
  });

  it('changing the icon through the picker updates once', async () => {
    const { target, calls } = await mountSettings(Statuses);
    await add(target, 'Started', 'Review');
    await clickLabel('Icon and colour of Review');
    await clickLabel('flag');
    expect(calls.filter((c) => c === 'statuses.update')).toHaveLength(1);
  });

  it('issue counts agree with their number: 1 issue, not 1 issues', async () => {
    const { target } = await mountSettings(Statuses);
    const meta = [...target.querySelectorAll('.es-meta')].map((m) => m.textContent?.trim() ?? '');
    expect(meta.length).toBeGreaterThan(0);
    expect(meta.filter((t) => /^1 issues?$/.test(t))).toEqual(meta.filter((t) => t.startsWith('1 ')));
    expect(meta.join(' ')).not.toMatch(/\b1 issues\b/);
  });
});
