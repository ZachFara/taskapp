import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import type { StaleIssue } from '../../lib/bindings';
import StaleCard from './StaleCard.svelte';

let app: Record<string, unknown> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});
const text = () => document.body.textContent?.replace(/\s+/g, ' ') ?? '';
const row = (id: number, days: number, status_name: string): StaleIssue =>
  ({ issue: { id, key: `ABC-${id}`, title: `Idea ${id}` }, status_name, nudge_days: 7, since: '2026-01-01T00:00:00Z', days }) as unknown as StaleIssue;
const show = (stale: StaleIssue[]) => {
  app = mount(StaleCard, { target: document.body, props: { metrics: null as never, measure: 'Count', stale } });
  flushSync();
};

describe('StaleCard', () => {
  it('says so when nothing is stale, and how to make issues stale', () => {
    show([]);
    expect(text()).toContain('Nothing is stale');
    expect(text()).toContain('Settings, Statuses');
  });

  it('counts by status and lists the oldest first, each linked to its issue', () => {
    show([row(1, 90, 'Inbox'), row(2, 40, 'Someday'), row(3, 30, 'Inbox')]);
    expect(text()).toContain('3 issues waiting past their nudge');
    const keys = [...document.querySelectorAll<HTMLAnchorElement>('ol.oldest a.key')].map((a) => a.getAttribute('href'));
    expect(keys).toEqual(['/issue/ABC-1', '/issue/ABC-2', '/issue/ABC-3']);
    expect(text()).toContain('90 d in Inbox');
    expect(document.querySelector('ul.statuses')?.textContent).toContain('Inbox2');
  });
});
