import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import SimilarIssues from './SimilarIssues.svelte';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
// The owner's Duplicate status on top of the fixtures.
const seed = () => ({
  ...fixtures,
  statuses: [...fixtures.statuses, { ...fixtures.statuses[0], id: 90, name: 'Duplicate', position: 90, category: 'Canceled' as const }],
});
const act = async (p: Promise<unknown>) => {
  await p;
};

describe('SimilarIssues', () => {
  it('lists up to three other similar issues, and clicking one opens it', async () => {
    const api = createMockApi(seed());
    const base = await api.issues.create({ title: 'Login page is slow' });
    for (const title of ['Login page slow on mobile', 'Slow login page load', 'Login page slow again', 'Login page slow to load']) {
      await api.issues.create({ title });
    }
    setApi(api);
    const opened: string[] = [];
    live = mount(SimilarIssues, { target: document.body, props: { issue: base, act, rev: 0, open: (key: string) => opened.push(key) } });
    await settle();
    const rows = document.querySelectorAll('[data-similar]');
    expect(rows.length).toBe(3);
    expect(document.body.textContent).not.toContain(base.key + ' ' + base.title);
    (rows[0].querySelector('button.go') as HTMLElement).click();
    expect(opened.length).toBe(1);
  });

  it('Mark duplicate sets status Duplicate and adds a duplicate relation, in one call', async () => {
    const api = createMockApi(seed());
    const base = await api.issues.create({ title: 'Export fails on empty list' });
    const other = await api.issues.create({ title: 'Export fails for an empty list' });
    setApi(api);
    live = mount(SimilarIssues, { target: document.body, props: { issue: base, act, rev: 0, open: () => {} } });
    await settle();
    (document.querySelector('[data-similar] button.dupe') as HTMLElement).click();
    await settle();
    expect((await api.issues.get(base.id)).status_id).toBe(90);
    const rels = await api.relations.forIssue(base.id);
    expect(rels.some((r) => r.kind === 'duplicate' && r.from === base.id && r.to === other.id)).toBe(true);
  });
});
