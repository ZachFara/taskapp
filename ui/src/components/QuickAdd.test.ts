import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { refreshAll } from '../lib/stores.svelte';
import QuickAdd from './QuickAdd.svelte';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const input = () => document.querySelector<HTMLInputElement>('input[data-quickadd]')!;
const chips = () => [...document.querySelectorAll<HTMLElement>('[data-chip]')].map((c) => c.textContent?.trim());
const options = () => [...document.querySelectorAll<HTMLElement>('[role=option]')].map((o) => o.textContent?.trim());

async function type(text: string) {
  input().value = text;
  input().dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
async function key(init: KeyboardEventInit) {
  input().dispatchEvent(new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init }));
  await settle();
}
const enter = () => key({ key: 'Enter' });

const onclose = vi.fn();
const oncreated = vi.fn();
let api: ReturnType<typeof createMockApi>;

function open(props: Record<string, unknown> = {}) {
  app = mount(QuickAdd, { target: document.body, props: { onclose, oncreated, ...props } });
  return settle();
}

beforeEach(async () => {
  api = createMockApi();
  setApi(api);
  onclose.mockClear();
  oncreated.mockClear();
  await open();
});
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('QuickAdd', () => {
  // The key that opened the dialog must not reach the field: it mounts empty and focused.
  it('quickadd_opened_by_c_starts_with_empty_field', async () => {
    expect(input().value).toBe('');
    expect(document.activeElement).toBe(input());
  });

  it('quickadd_preview_chips_reflect_parse', async () => {
    const parse = vi.spyOn(api.issues, 'parseQuickadd').mockResolvedValue({
      title: 'Buy milk',
      due: null,
      start_date: null,
      priority: 2,
      project_slug: 'home',
      label_names: ['errands'],
      estimate_minutes: 30,
    });
    await type('Buy milk p2 +home #errands ~30m');
    expect(parse).toHaveBeenLastCalledWith('Buy milk p2 +home #errands ~30m');
    expect(chips()).toEqual(['High', '+home', '#errands', '30m']);
  });

  it('quickadd_every_phrase_shows_a_repeat_chip_and_sets_repeat', async () => {
    await type('water plants every monday');
    expect(chips()).toContain('Every week on Mon');
    await enter();
    const made = oncreated.mock.calls[0][0].issue;
    expect(made.title).toBe('water plants');
    expect(made.repeat).toBe('every 1 week on mon from due');
  });

  it('quickadd_slash_lists_templates_and_inserts_slug', async () => {
    await api.templates.create({ name: 'Incident', slug: 'incident', icon: null, color: null, body: {} });
    await type('/');
    expect(options()).toContain('/incident Incident');
    ([...document.querySelectorAll('[role=option]')].find((o) => o.textContent?.includes('/incident')) as HTMLElement).click();
    await settle();
    expect(input().value).toBe('/incident ');
  });

  it('quickadd_enter_creates_and_closes', async () => {
    const create = vi.spyOn(api.issues, 'createFromQuickadd');
    await type('Buy milk');
    await enter();
    expect(create).toHaveBeenCalledWith('Buy milk');
    expect(oncreated).toHaveBeenCalledWith(expect.objectContaining({ issue: expect.objectContaining({ title: 'Buy milk' }) }));
    expect(onclose).toHaveBeenCalled();
  });

  it('quickadd_mod_enter_creates_and_opens_issue', async () => {
    await type('Ship it');
    await key({ key: 'Enter', metaKey: true });
    expect(onclose).toHaveBeenCalled();
    expect(location.pathname).toMatch(/^\/issue\//);
  });

  it('quickadd_shift_enter_creates_and_keeps_dialog_open', async () => {
    await type('First');
    await key({ key: 'Enter', shiftKey: true });
    expect(onclose).not.toHaveBeenCalled();
    expect(input().value).toBe('');
    expect(oncreated).toHaveBeenCalledTimes(1);
    expect(document.activeElement).toBe(input());
  });

  it('quickadd_whitespace_only_does_not_create', async () => {
    const create = vi.spyOn(api.issues, 'createFromQuickadd');
    await type('   ');
    await enter();
    expect(create).not.toHaveBeenCalled();
    expect(onclose).not.toHaveBeenCalled();
  });

  it('quickadd_create_error_keeps_text_and_shows_message', async () => {
    api.issues.createFromQuickadd = async () => {
      throw { kind: 'Invalid', message: 'title too long' };
    };
    await type('Buy milk');
    await enter();
    expect(input().value).toBe('Buy milk');
    expect(document.body.textContent).toContain('title too long');
    expect(onclose).not.toHaveBeenCalled();
  });

  it('quickadd_project_prefill_adds_project_token', async () => {
    await unmount(app!);
    document.body.innerHTML = '';
    await api.projects.create('quickadd-proj', 'Quickadd project', null, null);
    await refreshAll();
    await open({ project_slug: 'quickadd-proj' });
    expect(input().value).toBe('+quickadd-proj ');
  });

  it('quickadd_workspace_default_status_is_not_a_chip_but_another_status_is', async () => {
    const [first, second] = [...(await api.statuses.list())].sort((a, b) => a.position - b.position);
    await unmount(app!);
    document.body.innerHTML = '';
    await open({ status_id: first.id });
    expect(chips()).toEqual([]);
    await unmount(app!);
    document.body.innerHTML = '';
    await open({ status_id: second.id });
    expect(chips()).toEqual([second.name]);
  });

  it('quickadd_status_prefill_is_one_create_call_with_the_status_and_one_undo_step', async () => {
    const status = (await api.statuses.list())[1];
    await unmount(app!);
    document.body.innerHTML = '';
    await open({ status_id: status.id });
    const update = vi.spyOn(api.issues, 'update');
    const create = vi.spyOn(api.issues, 'create');
    const undo = vi.spyOn(api.history, 'undo');
    await type('Draft p2');
    await enter();
    expect(update).not.toHaveBeenCalled();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({ title: 'Draft', status_id: status.id, priority: 2 }));
    await oncreated.mock.calls[0][0].undo();
    expect(undo).toHaveBeenCalledTimes(1);
  });

  it('quickadd_template_with_status_prefill_is_one_apply_call_and_one_undo_step', async () => {
    const status = (await api.statuses.list())[1];
    await api.templates.create({ name: 'Bug', slug: 'bug', icon: null, color: null, body: { title_prefix: 'Bug: ', fields: {} as never, sub_issues: [] } });
    await unmount(app!);
    document.body.innerHTML = '';
    await open({ status_id: status.id });
    const update = vi.spyOn(api.issues, 'update');
    const apply = vi.spyOn(api.templates, 'apply');
    const undo = vi.spyOn(api.history, 'undo');
    await type('/bug Crash p2');
    await enter();
    expect(update).not.toHaveBeenCalled();
    expect(apply).toHaveBeenCalledTimes(1);
    expect(apply.mock.calls[0][1]).toEqual(expect.objectContaining({ title: 'Crash', status_id: status.id, priority: 2 }));
    expect(oncreated.mock.calls[0][0].issue).toEqual(expect.objectContaining({ title: 'Bug: Crash', status_id: status.id }));
    await oncreated.mock.calls[0][0].undo();
    expect(undo).toHaveBeenCalledTimes(1);
  });

  it('quickadd_defaults_show_chips_merge_on_create_and_allow_token_override', async () => {
    const statuses = await api.statuses.list();
    const status = statuses[2];
    const proj = await api.projects.create('alpha', 'Alpha', null, null);
    const other = await api.projects.create('beta', 'Beta', null, null);
    const label = await api.labels.create('triage', 'blue', null);
    await refreshAll();
    await unmount(app!);
    document.body.innerHTML = '';
    await open({
      defaults: {
        status_id: status.id,
        project_id: proj.id,
        priority: 2,
        label_ids: [label.id],
      },
    });
    expect(input().value).toBe('');
    expect(chips()).toEqual([status.name, 'High', '+alpha', '#triage']);
    const create = vi.spyOn(api.issues, 'create');
    await type('From view +beta p1');
    await enter();
    expect(create).toHaveBeenCalledWith(expect.objectContaining({
      title: 'From view',
      status_id: status.id,
      project_id: other.id,
      priority: 1,
      label_ids: [label.id],
    }));
  });

  it('quickadd_shows_possible_duplicates_as_the_title_is_typed_and_still_creates', async () => {
    await api.issues.create({ title: 'Migrate blog to new CMS' });
    await type('Migrate the blog');
    expect(document.querySelectorAll('[data-dupe]')).toHaveLength(0); // debounced
    await new Promise((r) => setTimeout(r, 300));
    await settle();
    const dupes = [...document.querySelectorAll<HTMLElement>('[data-dupe]')].map((d) => d.textContent);
    expect(dupes.some((d) => d?.includes('Migrate blog to new CMS'))).toBe(true);
    expect(dupes.length).toBeLessThanOrEqual(3);
    await enter();
    expect(oncreated).toHaveBeenCalledOnce();
  });

  it('quickadd_shows_no_duplicates_for_an_unrelated_title', async () => {
    await type('zzzz qqqq');
    await new Promise((r) => setTimeout(r, 300));
    await settle();
    expect(document.querySelectorAll('[data-dupe]')).toHaveLength(0);
  });
});
