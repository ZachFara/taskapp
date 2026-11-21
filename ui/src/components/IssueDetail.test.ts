import { afterEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import { createMockApi, fixtures } from '../lib/api.mock';
import { setApi, type Api } from '../lib/api';
import IssueDetail from './IssueDetail.svelte';

let app: object | undefined;
let api: Api;

const settle = () => new Promise((r) => setTimeout(r, 20));
async function open(key: string, wrap?: (a: Api) => Api) {
  api = createMockApi();
  setApi(wrap ? wrap(api) : api);
  app = mount(IssueDetail, { target: document.body, props: { params: { key } } });
  await settle();
}
const q = <T extends Element>(sel: string) => document.querySelector(sel) as T;
async function type(sel: string, value: string, event = 'change') {
  const el = q<HTMLInputElement>(sel);
  el.value = value;
  el.dispatchEvent(new Event(event, { bubbles: true }));
  await settle();
}
async function check(sel: string, on: boolean) {
  const el = q<HTMLInputElement>(sel);
  el.checked = on;
  el.dispatchEvent(new Event('change', { bubbles: true }));
  await settle();
}
const click = async (el: Element) => {
  (el as HTMLElement).click();
  await settle();
};
const button = (text: string) => [...document.querySelectorAll('button')].find((b) => b.textContent!.trim() === text || (text.endsWith(' ') && b.textContent!.startsWith(text)))!;
const field = (name: string) =>
  [...document.querySelectorAll('label')].find((l) => l.textContent!.startsWith(name))!.querySelector('input, select') as HTMLInputElement;
const get = (key: string) => api.issues.getByKey(key);

afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

describe('IssueDetail', () => {
  it('edits_title_and_saves', async () => {
    await open('ZF-1');
    await type('[aria-label=Title]', 'Renamed');
    expect((await get('ZF-1')).title).toBe('Renamed');
  });

  it('due_time_toggle_creates_datetime', async () => {
    await open('ZF-1');
    const due = field('Due');
    due.value = '2026-12-05';
    due.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect((await get('ZF-1')).due).toEqual({ Date: '2026-12-05' });
    await check('[aria-label="Due time"]', true);
    expect((await get('ZF-1')).due?.DateTime?.utc).toBe(new Date('2026-12-05T09:00').toISOString());
  });

  it('clearing_due_sends_null', async () => {
    await open('ZF-4'); // fixture with a due date
    const spy = vi.spyOn(api.issues, 'update');
    const due = field('Due');
    due.value = '';
    due.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect(spy).toHaveBeenCalledWith(4, { due: null });
    expect((await get('ZF-4')).due).toBeNull();
  });

  it('properties_filtered_by_project_scope', async () => {
    const seed = structuredClone(fixtures);
    seed.propertyDefs.push({ key: 'home_only', name: 'Home only', kind: 'Text', options: [], project_id: 1, position: 9 });
    api = createMockApi(seed);
    setApi(api);
    // ZF-2 is in project 1 (home), ZF-1 in project 2 (work).
    app = mount(IssueDetail, { target: document.body, props: { params: { key: 'ZF-2' } } });
    await settle();
    expect(q('[aria-label=Properties]').textContent).toContain('Home only');
    unmount(app);
    document.body.innerHTML = '';
    app = mount(IssueDetail, { target: document.body, props: { params: { key: 'ZF-1' } } });
    await settle();
    expect(q('[aria-label=Properties]').textContent).not.toContain('Home only');
    expect(q('[aria-label=Properties]').textContent).toContain('Area');
  });

  it('add_blocking_relation', async () => {
    await open('ZF-1');
    await type('[aria-label="Find issue"]', 'Fixture task 2', 'input');
    await click(button('ZF-2 '));
    expect(await api.relations.forIssue(1)).toEqual([{ from: 1, to: 2, kind: 'Blocks' }]);
    expect(q('[aria-label=Relations]').textContent).toContain('blocks ZF-2');
  });

  it('cycle_error_shown', async () => {
    const seed = { ...structuredClone(fixtures), cycles: [{ id: 1, number: 1, name: 'C1', start_date: '2026-12-01', end_date: '2026-12-14' }] };
    api = createMockApi(seed);
    setApi({ ...api, issues: { ...api.issues, update: async () => { throw { kind: 'Conflict', message: 'cycle in parent chain' }; } } });
    app = mount(IssueDetail, { target: document.body, props: { params: { key: 'ZF-1' } } });
    await settle();
    const cycle = field('Cycle');
    cycle.value = '1';
    cycle.dispatchEvent(new Event('change', { bubbles: true }));
    await settle();
    expect(q('[role=alert]').textContent).toContain('cycle in parent chain');
  });

  it('comment_posts', async () => {
    await open('ZF-1');
    await type('[aria-label="New comment"]', 'hello **there**', 'input');
    await click(button('Comment')!);
    expect(q('[aria-label=Comments]').innerHTML).toContain('<strong>there</strong>');
    expect(await api.comments.list(1)).toHaveLength(1);
  });

  it('history_tab_lists_changes', async () => {
    await open('ZF-1');
    await type('[aria-label=Title]', 'Changed');
    await click(button('History'));
    expect(q('[aria-label=History]').textContent).toContain('title');
    expect(q('[aria-label=History]').textContent).toContain('"Changed"');
  });

  it('script_in_description_is_sanitized', async () => {
    const seed = structuredClone(fixtures);
    seed.issues[0].description = '<script>window.pwned = 1</script> <img src=x onerror="window.pwned=1">';
    api = createMockApi(seed);
    setApi(api);
    app = mount(IssueDetail, { target: document.body, props: { params: { key: 'ZF-1' } } });
    await settle();
    const box = q('[aria-label=Description]');
    expect(box.querySelector('script, [onerror]')).toBeNull();
    expect(box.innerHTML).not.toContain('onerror');
  });

  it('sub_issue_cannot_be_the_issue_itself', async () => {
    await open('ZF-1');
    await type('[aria-label="Add sub-issue"]', 'Fixture task 1', 'input');
    const labels = [...document.querySelectorAll('[aria-label=Sub-issues] .results button')].map((b) => b.textContent);
    expect(labels.some((t) => t!.startsWith('ZF-1 '))).toBe(false);
    expect(labels.some((t) => t!.startsWith('ZF-10 '))).toBe(true);
    await click(button('ZF-10 '));
    expect((await get('ZF-10')).parent_id).toBe(1);
  });
});
