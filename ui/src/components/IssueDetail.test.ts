import { afterEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import { createMockApi } from '../lib/api.mock';
import { setApi, type Api } from '../lib/api';
import type { Issue } from '../lib/bindings';
import { attachKeys } from '../lib/commands';
import IssueDetail from './IssueDetail.svelte';

let app: object | undefined;
let api: Api;

const settle = (ms = 30) => new Promise((r) => setTimeout(r, ms));
/** Mounts the screen on an issue this test creates, so nothing depends on fixture ids or keys. */
async function open(patch: Partial<Parameters<Api['issues']['create']>[0]> = {}, props: Record<string, unknown> = {}, wrap?: (a: Api) => Api) {
  api = createMockApi();
  setApi(wrap ? wrap(api) : api);
  const issue = await api.issues.create({ title: 'Detail subject', ...patch });
  app = mount(IssueDetail, { target: document.body, props: { params: { key: issue.key }, ...props } });
  await settle();
  return issue;
}
const q = <T extends Element>(sel: string) => document.querySelector(sel) as T;
const byLabel = (label: string) => q<HTMLInputElement>(`[aria-label="${label}"]`);
async function type(label: string, value: string) {
  const el = byLabel(label);
  el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
const click = async (el: Element | null) => {
  (el as HTMLElement).click();
  await settle();
};
const buttons = () => [...document.querySelectorAll('button')];
const button = (text: string) => (buttons().find((b) => b.textContent!.trim() === text) ?? buttons().find((b) => b.textContent!.trim().startsWith(text)))!;
const row = (name: string) => byLabel(name) as HTMLElement;
const option = (label: string, text: string) => [...document.querySelectorAll(`[aria-label="${label}"] [role=option]`)].find((o) => o.textContent!.trim().endsWith(text))!;
const get = (id: number): Promise<Issue> => api.issues.get(id);
const command = async (name: string) => {
  dispatchEvent(new CustomEvent(`command:issue.detail.${name}`));
  await settle();
};

afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

describe('IssueDetail', () => {
  it('edits_title_inline', async () => {
    const i = await open();
    await click(byLabel('Title'));
    const spy = vi.spyOn(api.issues, 'update');
    await type('Title', 'Renamed');
    q<HTMLFormElement>('form').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).title).toBe('Renamed');
    expect(q('h2').textContent).toContain('Renamed');
  });

  it('priority_edit_calls_api_once_and_shows_value', async () => {
    const i = await open();
    const spy = vi.spyOn(api.issues, 'update');
    await click(row('Priority'));
    await click(option('Priority', 'High'));
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).priority).toBe(2);
    expect(row('Priority').textContent).toContain('High');
  });

  it('status_edit_calls_api_once_and_shows_value', async () => {
    const i = await open();
    const statuses = await api.statuses.list();
    const other = statuses.find((s) => s.id !== i.status_id)!;
    const spy = vi.spyOn(api.issues, 'update');
    await click(row('Status'));
    await click(option('Status', other.name));
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).status_id).toBe(other.id);
    expect(row('Status').textContent).toContain(other.name);
  });

  it('estimate_edit_calls_api_once_and_shows_value', async () => {
    const i = await open();
    const spy = vi.spyOn(api.issues, 'update');
    await click(row('Estimate'));
    await type('Estimate (min)', '90');
    q<HTMLFormElement>('form.stack').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).estimate_minutes).toBe(90);
    expect(row('Estimate').textContent).toContain('1h 30m');
  });

  it('due_date_edit_calls_api_once_and_clearing_sends_null', async () => {
    const i = await open();
    const spy = vi.spyOn(api.issues, 'update');
    await click(row('Due'));
    await type('Due date', '2026-12-05');
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).due).toEqual({ Date: '2026-12-05' });
    expect(row('Due').textContent).toContain('Dec 5');
    await type('Due date', '');
    expect(spy).toHaveBeenLastCalledWith(i.id, { due: null });
    expect((await get(i.id)).due).toBeNull();
  });

  it('label_toggle_calls_api_once_and_shows_value', async () => {
    const i = await open();
    const l = await api.labels.create('Urgent-ish', 'red', null);
    unmount(app!);
    document.body.innerHTML = '';
    app = mount(IssueDetail, { target: document.body, props: { params: { key: i.key } } });
    await settle();
    const spy = vi.spyOn(api.labels, 'addLabel');
    await click(row('Labels'));
    await click(q(`[aria-label="${l.name}"]`));
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).label_ids).toEqual([l.id]);
    expect(row('Labels').textContent).toContain(l.name);
  });

  it('custom_property_edit_shows_value', async () => {
    const i = await open();
    await api.properties.createDef?.({ key: 'note', name: 'Note', kind: 'Text', options: [], project_id: null, position: 9 } as never).catch(() => {});
    const defs = await api.properties.listDefs();
    const d = defs.find((x) => x.kind === 'Number')!;
    unmount(app!);
    document.body.innerHTML = '';
    app = mount(IssueDetail, { target: document.body, props: { params: { key: i.key } } });
    await settle();
    const spy = vi.spyOn(api.properties, 'setValue');
    await click(row(d.name));
    await type(`${d.name} value`, '7');
    q<HTMLFormElement>('form.stack').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(spy).toHaveBeenCalledTimes(1);
    expect(row(d.name).textContent).toContain('7');
  });

  it('markdown_is_sanitised', async () => {
    await open({ description: '<script>window.pwned = 1</script>\n\n<img src=x onerror="window.pwned=1">\n\n**bold**' });
    const box = q('[aria-label=Description]');
    expect(box.querySelector('script, [onerror]')).toBeNull();
    expect(box.innerHTML).not.toContain('onerror');
    expect(box.innerHTML).toContain('<strong>bold</strong>');
  });

  it('description_edit_preview_and_save', async () => {
    const i = await open({ description: 'old' });
    await click(byLabel('Edit description'));
    await type('Description text', 'new *text*');
    await click(button('Preview'));
    expect(q('[aria-label="Description preview"]').innerHTML).toContain('<em>text</em>');
    await click(button('Save'));
    expect((await get(i.id)).description).toBe('new *text*');
  });

  it('sub_issue_quick_add_and_progress', async () => {
    const i = await open();
    await type('New sub-issue title', 'Child one');
    q<HTMLFormElement>('[aria-label=Sub-issues] form').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    const kids = await api.issues.children(i.id);
    expect(kids.map((k) => k.title)).toEqual(['Child one']);
    expect(q('[aria-label=Sub-issues]').textContent).toContain('Child one');
    expect(q('[role=progressbar]').textContent).toContain('0 of 1 done');
  });

  it('sub_issue_picker_never_offers_the_issue_itself', async () => {
    const i = await open({ title: 'Selfish subject' });
    const other = await api.issues.create({ title: 'Selfish other' });
    await type('New sub-issue title', 'Selfish');
    const keys = [...document.querySelectorAll('[aria-label=Sub-issues] .results button')].map((b) => b.textContent);
    expect(keys.some((t) => t!.includes(i.key + ' '))).toBe(false);
    await click(button(other.key));
    expect((await get(other.id)).parent_id).toBe(i.id);
  });

  it('relation_add_and_remove', async () => {
    const i = await open({ title: 'Rel subject' });
    const other = await api.issues.create({ title: 'Rel other' });
    await type('Find issue', 'Rel other');
    await click(button(other.key));
    expect(await api.relations.forIssue(i.id)).toEqual([{ from: i.id, to: other.id, kind: 'Blocks' }]);
    expect(q('[aria-label=Relations]').textContent).toContain(`blocks ${other.key}`);
    await click(byLabel(`Remove relation to ${other.key}`));
    expect(await api.relations.forIssue(i.id)).toEqual([]);
  });

  it('comment_posts_and_history_is_dated', async () => {
    const i = await open();
    await type('New comment', 'hello **there**');
    await click(button('Comment'));
    expect(q('[aria-label=Comments]').innerHTML).toContain('<strong>there</strong>');
    expect(await api.comments.list(i.id)).toHaveLength(1);
    await click(byLabel('Title'));
    await type('Title', 'Changed');
    q<HTMLFormElement>('form').dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(q('[aria-label=History]').textContent).toContain('Changed');
    expect(q('[aria-label=History] h4').textContent).toMatch(/^[A-Z][a-z]{2} \d{1,2}(, \d{4})?$/);
  });

  it('pin_and_archive_actions', async () => {
    const i = await open();
    await click(byLabel('Pin issue'));
    expect((await api.pins.listResolved()).map((p) => p.pin.target)).toEqual([i.key]);
    await click(byLabel('Unpin issue'));
    expect(await api.pins.listResolved()).toEqual([]);
    await click(byLabel('Archive issue'));
    expect((await get(i.id)).archived_at).not.toBeNull();
    expect(q('.arch').textContent).toContain('Archived');
  });

  it('cycle_error_is_shown', async () => {
    await open({}, {}, (a) => ({ ...a, issues: { ...a.issues, update: async () => { throw { kind: 'Conflict', message: 'cycle in parent chain' }; } } }));
    await click(row('Priority'));
    await click(option('Priority', 'High'));
    expect(q('[role=alert]').textContent).toContain('cycle in parent chain');
  });

  it('esc_closes_editor_before_panel', async () => {
    const onclose = vi.fn();
    await open({}, { mode: 'panel', onclose });
    await click(row('Priority'));
    expect(option('Priority', 'High')).toBeTruthy();
    await command('close');
    expect(document.querySelector('[aria-label=Priority][role=listbox]')).toBeNull();
    expect(onclose).not.toHaveBeenCalled();
    await settle(100);
    await command('close');
    expect(onclose).toHaveBeenCalledTimes(1);
  });

  it('header_is_a_breadcrumb_and_the_title_shows_once', async () => {
    const i = await open({ title: 'Only once please', project_id: 1 });
    const crumb = q('header h1').textContent!;
    expect(crumb).toContain(i.key);
    expect(crumb).toContain('/');
    expect(crumb).not.toContain('Only once please');
    expect(q('.hero').textContent!.split('Only once please').length - 1).toBe(1);
  });

  it('panel_header_shows_only_the_key_with_the_project_as_a_tooltip', async () => {
    const i = await open({ title: 'Narrow', project_id: 1 }, { mode: 'panel', onclose: () => {} });
    const crumb = q('header h1');
    expect(crumb.textContent).toContain(i.key);
    expect(crumb.textContent).not.toContain('/');
    expect(crumb.querySelector('[title]')?.getAttribute('title')).toBeTruthy();
  });

  it('empty_sections_are_one_line_without_an_illustration', async () => {
    await open();
    for (const s of ['Sub-issues', 'Relations', 'Comments']) expect(q(`[aria-label=${s}]`).querySelector('.empty, svg.contour, [data-contour]')).toBeNull();
    expect(q('[aria-label=Sub-issues]').textContent).toContain('No sub-issues');
    expect(q('[aria-label=Relations]').textContent).toContain('No relations');
    expect(q('[aria-label=Sub-issues]').querySelectorAll('input').length).toBe(1);
  });

  it('sections_are_plain_headings_and_only_status_groups_get_bands', async () => {
    await open();
    expect(document.querySelectorAll('[data-step]').length).toBe(0);
    for (const s of ['Sub-issues', 'Relations', 'Comments', 'Activity']) {
      expect(q(`[data-section-heading="${s}"]`)).not.toBeNull();
    }
  });

  it('activity_has_no_add_button', async () => {
    await open();
    expect(byLabel('Add to Activity')).toBeNull();
  });

  it('due_time_uses_a_time_input', async () => {
    await open({ project_id: 1 });
    await click(row('Due'));
    await type('Due date', '2026-12-05');
    await click(q('[aria-label="Due time"]'));
    expect(byLabel('Due time of day').type).toBe('time');
  });

  it('milestone_edit_calls_api_once_and_shows_value', async () => {
    const i = await open({ project_id: 1 });
    const m = await api.projects.createMilestone(1, 'Beta', null);
    unmount(app!);
    document.body.innerHTML = '';
    app = mount(IssueDetail, { target: document.body, props: { params: { key: i.key } } });
    await settle();
    const spy = vi.spyOn(api.issues, 'update');
    await click(row('Milestone'));
    await click(option('Milestone', 'Beta'));
    expect(spy).toHaveBeenCalledTimes(1);
    expect((await get(i.id)).milestone_id).toBe(m.id);
    expect(row('Milestone').textContent).toContain('Beta');
  });

  it('relation_kind_select_chooses_blocked_by', async () => {
    const i = await open({ title: 'Kind subject' });
    const other = await api.issues.create({ title: 'Kind other' });
    // jsdom lacks what the select's pointer handling and scrolling call.
    Element.prototype.hasPointerCapture ??= () => false;
    Element.prototype.releasePointerCapture ??= () => {};
    Element.prototype.scrollIntoView ??= () => {};
    const trigger = byLabel('Relation kind');
    trigger.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, pointerType: 'mouse' }));
    await settle();
    const blockedBy = [...document.querySelectorAll('[role=option]')].find((o) => o.textContent!.trim() === 'blocked by')!;
    blockedBy.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse' }));
    blockedBy.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, button: 0, pointerType: 'mouse' }));
    await click(blockedBy);
    await type('Find issue', 'Kind other');
    await click(button(other.key));
    expect(await api.relations.forIssue(i.id)).toEqual([{ from: other.id, to: i.id, kind: 'Blocks' }]);
  });

  describe('through the real key registry', () => {
    let detach: () => void;
    const ctx = () => ({ route: '/issue/x', selection: [], focusedIssueId: null, openIssueId: null, api, navigate: () => {} });
    const press = async (key: string) => {
      document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
      await settle();
    };
    afterEach(() => detach?.());

    it('esc_closes_the_open_editor_first_then_the_panel', async () => {
      const onclose = vi.fn();
      await open({}, { mode: 'panel', onclose });
      detach = attachKeys(ctx);
      await click(row('Priority'));
      expect(q('[role=listbox][aria-label=Priority]')).not.toBeNull();
      await press('Escape');
      expect(q('[role=listbox][aria-label=Priority]')).toBeNull();
      expect(onclose).not.toHaveBeenCalled();
      await settle(100);
      await press('Escape');
      expect(onclose).toHaveBeenCalledTimes(1);
    });

    it('brackets_move_to_the_next_and_previous_issue', async () => {
      const onopen = vi.fn();
      const mid = await open({ title: 'Mid' }, { onopen });
      await api.issues.create({ title: 'After' });
      const keys = (await api.views.query({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false })).map((i) => i.key);
      // Remount so the neighbour list includes the issue created after mount.
      unmount(app!);
      document.body.innerHTML = '';
      app = mount(IssueDetail, { target: document.body, props: { params: { key: mid.key }, onopen } });
      await settle();
      detach = attachKeys(ctx);
      const at = keys.indexOf(mid.key);
      await press(']');
      expect(onopen).toHaveBeenLastCalledWith(keys[at + 1]);
      await press('[');
      expect(onopen).toHaveBeenLastCalledWith(keys[at - 1]);
    });
  });
});
