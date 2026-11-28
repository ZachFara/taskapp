import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { listCommands, type Ctx } from '../lib/commands';
import PlanPage from './PlanPage.svelte';

const settle = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve();
  await tick();
};
let app: ReturnType<typeof mount> | undefined;
afterEach(() => app && unmount(app));
const click = async (el: Element | null) => (el!.dispatchEvent(new MouseEvent('click', { bubbles: true })), await new Promise((r) => setTimeout(r, 5)), await settle());

describe('the Today plan page', () => {
  it('plans a picked issue as a linked event, shows planned vs done, and removes the block', async () => {
    const api = createMockApi();
    setApi(api);
    const made = await api.issues.create({ title: 'Write the report' } as never);
    const issue = await api.issues.update(made.id, { depth: 'deep' });
    app = mount(PlanPage, { target: document.body });
    await settle();
    expect(document.body.textContent).toContain('Deep work');
    await click(document.querySelector(`[data-issue="${issue.id}"]`));
    await click([...document.querySelectorAll('button')].find((b) => b.getAttribute('aria-label') === 'Time-block an issue')!);
    expect(document.querySelector<HTMLInputElement>('input[aria-label="Length in minutes"]')!.value).toBe('60');
    await click([...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Add block')!);

    const [event] = (await api.events.list(null, null)).filter((e) => e.issue_ids.includes(issue.id));
    expect(event.title).toContain('Write the report');
    expect(event.starts_at.slice(11, 16)).toBe('09:00');
    expect(event.ends_at!.slice(11, 16)).toBe('10:00');
    expect(document.querySelector(`[data-event="${event.id}"]`)).not.toBeNull();
    const day = event.starts_at.slice(0, 10);
    expect(document.querySelector(`[data-summary="${day}"]`)!.textContent).toContain('Planned 1h in 1 block; done 0h (0 of 1)');

    const done = (await api.statuses.list()).find((s) => s.category === 'Completed')!;
    await api.issues.update(issue.id, { status_id: done.id });
    dispatchEvent(new CustomEvent('rows:changed'));
    await settle();
    expect(document.querySelector(`[data-summary="${day}"]`)!.textContent).toContain('done 1h (1 of 1)');

    dispatchEvent(new CustomEvent('command:plan.remove'));
    await settle();
    expect(await api.events.list(null, null)).not.toContainEqual(expect.objectContaining({ id: event.id }));
  });

  it('plan commands follow the time blocking switch', async () => {
    const { features, isOn } = await import('../lib/features.svelte');
    const ctx = { route: '/plan', layout: 'list', selection: [], focusedIssueId: null, openIssueId: null, api: {} as Ctx['api'], navigate: () => {} } as Ctx;
    const ids = () => listCommands(ctx).map((c) => c.id);
    expect(ids()).toEqual(expect.arrayContaining(['nav.plan', 'plan.add', 'plan.edit', 'plan.remove']));
    features.time_blocking = false;
    expect(isOn('time_blocking')).toBe(false);
    expect(ids().filter((i) => i.startsWith('plan.') || i === 'nav.plan')).toEqual([]);
    features.time_blocking = true;
  });
});
