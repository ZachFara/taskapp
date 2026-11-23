import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import type { EventInput } from '../../lib/bindings';
import EventPage from './EventPage.svelte';
import EventsPage from './EventsPage.svelte';
import IssueEvents from './IssueEvents.svelte';

const TODAY = new Date('2026-11-22T12:00:00Z');
const input = (title: string, starts_at: string, extra: Partial<EventInput> = {}): EventInput =>
  ({ title, starts_at, ends_at: null, all_day: false, place: null, notes: '', icon: null, color: null, archived: false, ...extra });

let live: ReturnType<typeof mount> | null = null;
let api: Api;
beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(TODAY);
  api = createMockApi();
  setApi(api);
});
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.useRealTimers();
});

const settle = async () => {
  await new Promise((r) => setTimeout(r, 20));
  flushSync();
};
async function show(component: Parameters<typeof mount>[0], props: Record<string, unknown> = {}) {
  live = mount(component, { target: document.body, props });
  await settle();
}
const button = (text: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === text)!;
const field = (label: string) => document.querySelector<HTMLInputElement>(`[aria-label="${label}"]`)!;
async function type(label: string, value: string) {
  const el = field(label);
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}

describe('Events page', () => {
  it('lists upcoming events by day and keeps the past behind a toggle', async () => {
    await api.events.create(input('Launch', '2026-12-01T09:30:00', { place: 'HQ' }));
    await api.events.create(input('Standup', '2026-12-01T10:00:00'));
    await api.events.create(input('Retro', '2026-11-01T00:00:00', { all_day: true }));
    await show(EventsPage);
    const days = [...document.querySelectorAll('[data-day]')].map((d) => d.getAttribute('data-day'));
    expect(days).toEqual(['2026-12-01']);
    expect(document.querySelectorAll('[data-day="2026-12-01"] [data-event]').length).toBe(2);
    expect(document.body.textContent).toContain('09:30');
    expect(document.querySelector('[data-day="2026-11-01"]')).toBeNull();
    button('Past (1)').click();
    await settle();
    expect(document.querySelector('[data-day="2026-11-01"]')?.textContent).toContain('All day');
  });

  it('creates an event from the dialog and opens it', async () => {
    await show(EventsPage);
    button('New event').click();
    await settle();
    await type('Event title', 'Offsite');
    await type('Date', '2026-12-10');
    button('Create event').click();
    await settle();
    expect((await api.events.list(null, null)).map((e) => [e.title, e.starts_at, e.all_day])).toEqual([['Offsite', '2026-12-10T00:00:00', true]]);
    expect(location.pathname).toBe('/event/1');
  });
});

describe('Event page', () => {
  it('shows linked issues and unlinks one', async () => {
    const issue = await api.issues.create({ title: 'Prepare slides' });
    const e = await api.events.create(input('Demo', '2026-12-02T14:00:00', { ends_at: '2026-12-02T15:00:00' }));
    await api.events.link(e.id, issue.id);
    await show(EventPage, { params: { id: String(e.id) } });
    expect(document.body.textContent).toContain('14:00 to 15:00');
    expect(document.body.textContent).toContain('Prepare slides');
    document.querySelector<HTMLButtonElement>(`[aria-label="Unlink ${issue.key}"]`)!.click();
    await settle();
    expect((await api.events.get(e.id)).issue_ids).toEqual([]);
    expect(document.body.textContent).not.toContain('Prepare slides');
  });

  it('says so for an event that does not exist', async () => {
    await show(EventPage, { params: { id: '99' } });
    expect(document.body.textContent).toContain('Event not found');
  });
});

describe('Events section of an issue', () => {
  it('lists the linked events and removes one', async () => {
    const issue = await api.issues.create({ title: 'Plan trip' });
    const e = await api.events.create(input('Flight', '2026-12-05T08:00:00'));
    await api.events.link(e.id, issue.id);
    const act = vi.fn((p: Promise<unknown>) => p);
    await show(IssueEvents, { issue, act, rev: 0 });
    expect(document.querySelector('[aria-label="Events"]')?.textContent).toContain('Flight');
    document.querySelector<HTMLButtonElement>('[aria-label="Remove from Flight"]')!.click();
    await settle();
    expect((await api.events.get(e.id)).issue_ids).toEqual([]);
    expect(act).toHaveBeenCalledOnce();
  });
});
