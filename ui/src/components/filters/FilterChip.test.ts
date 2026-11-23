import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import FilterChip from './FilterChip.svelte';
import { makeLookup } from './lookup';

let live: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
});

describe('FilterChip', () => {
  it('names every choice in its list, not only the checkbox', async () => {
    const lookup = makeLookup({ statuses: [], projects: [], defs: [], cycles: [], labels: [], relationKinds: [{ id: 'mentors', name: 'Mentors', forward: 'mentors', reverse: 'is mentored by', symmetric: false, built_in: false }] });
    live = mount(FilterChip, { target: document.body, props: { chip: { field: { Fixed: 'RelationKind' }, op: 'In', value: [] }, lookup, open: true, onchange: () => {}, onremove: () => {} } });
    await new Promise((r) => setTimeout(r, 20));
    flushSync();
    expect(document.querySelector('.opts')?.textContent).toContain('Mentors');
  });
});
