import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, tick, unmount } from 'svelte';
import { api, setApi, type Api } from '../lib/api';
import { createMockApi, fixtures, type MockSeed } from '../lib/api.mock';
import type { Project } from '../lib/bindings';
import { humanDate } from './detail/format';
import ProjectPage from './ProjectPage.svelte';

const base = fixtures.projects[0];
const seed = (projects: Partial<Project>[] = [{}], pathSlug = base.slug): MockSeed => ({
  ...fixtures,
  projects: projects.map((p, i) => ({ ...base, id: i + 1, slug: i ? `p${i + 1}` : pathSlug, name: `Project ${i + 1}`, ...p })),
});

let mounted: ReturnType<typeof mount>[] = [];
const open = async (path: string, source: MockSeed | Api = seed()) => {
  history.replaceState(null, '', path);
  setApi(source instanceof Object && 'pins' in source ? source : createMockApi(source));
  const target = document.createElement('div');
  document.body.append(target);
  const slug = decodeURIComponent(path.split('/')[2] ?? '');
  mounted.push(mount(ProjectPage, { target, props: { params: { slug } } }));
  await vi.waitFor(() => expect(target.textContent).not.toBe('')); await vi.waitFor(() => expect(target.querySelector('[aria-busy]')).toBeNull());
  return target;
};

beforeEach(() => {
  history.replaceState(null, '', '/');
});
afterEach(() => {
  mounted.forEach((m) => void unmount(m));
  mounted = [];
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('ProjectPage', () => {
  it('keeps_a_heading_sort_in_the_project_toolbar_and_other_layouts', async () => {
    const mock = createMockApi(seed());
    await mock.issues.create({ title: 'Alpha project sort', project_id: 1 });
    await mock.issues.create({ title: 'Zebra project sort', project_id: 1 });
    const target = await open('/project/website', mock);
    await vi.waitFor(() => expect(target.textContent).toContain('Zebra project sort'));
    target.querySelector<HTMLButtonElement>('button[aria-label="Sort by Title"]')!.click();
    await vi.waitFor(() => expect(target.querySelector('[data-testid="issues-toolbar"]')?.textContent).toContain('Sort: Title'));
    target.querySelector<HTMLButtonElement>('button[aria-label="Sort by Title"]')!.click();
    await vi.waitFor(() => expect(target.querySelector('button[aria-label="Sort by Title"]')?.textContent).toContain('↓'));
    target.querySelector<HTMLButtonElement>('[role="tab"][aria-label="Table"]')!.click();
    await vi.waitFor(() => {
      const rows = [...target.querySelectorAll<HTMLElement>('tr[data-key]')].map((r) => r.textContent ?? '');
      expect(rows.findIndex((r) => r.includes('Zebra project sort'))).toBeLessThan(rows.findIndex((r) => r.includes('Alpha project sort')));
    });
  });

  it('shows_a_forecast_line_for_the_project', async () => {
    const target = await open('/project/website');
    await vi.waitFor(() => expect(target.querySelector('[data-testid="forecast"]')?.textContent).toMatch(/Not enough history|Nothing is open|At the current pace/));
  });

  it('doc_link_button_shown_only_when_set', async () => {
    const without = await open('/project/website');
    expect(without.querySelector('[data-testid="open-obsidian"]')).toBeNull();
    mounted.forEach((m) => void unmount(m));
    mounted = [];
    document.body.innerHTML = '';

    const withLink = await open('/project/website', seed([{ doc_link: 'obsidian://open?vault=Ark&file=Home' }]));
    const button = withLink.querySelector<HTMLButtonElement>('[data-testid="open-obsidian"]');
    expect(button?.textContent).toContain('Open doc');
  });

  it('doc_link_not_obsidian_or_https_renders_no_button', async () => {
    for (const doc_link of ['javascript:alert(1)', 'http://example.com/doc', 'file:///etc/passwd', 'ftp://x']) {
      const target = await open('/project/website', seed([{ doc_link }]));
      expect(target.querySelector('[data-testid="open-obsidian"]'), doc_link).toBeNull();
      mounted.forEach((m) => void unmount(m));
      mounted = [];
      document.body.innerHTML = '';
    }
  });

  it('empty_description_hidden', async () => {
    const target = await open('/project/website', seed([{ description: null }]));
    expect(target.querySelector('[data-testid="description"]')).toBeNull();
    expect(target.textContent).toContain('Add description');
  });

  it('description_shown_when_present', async () => {
    const target = await open('/project/website', seed([{ description: 'Ship the rewrite' }]));
    expect(target.querySelector('[data-testid="description"]')?.textContent).toContain('Ship the rewrite');
    expect(target.textContent).not.toContain('Add description');
  });

  it('description_click_opens_editor_but_a_link_does_not', async () => {
    const target = await open('/project/website', seed([{ description: 'Ship the [rewrite](https://example.com) soon' }]));
    const desc = target.querySelector<HTMLElement>('[data-testid="description"]')!;
    desc.querySelector('a')!.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    await tick();
    expect(target.querySelector('textarea')).toBeNull();
    desc.click();
    await vi.waitFor(() => expect(target.querySelector('textarea')?.value).toContain('Ship the [rewrite]'));
  });

  it('project_issues_filtered_to_project', async () => {
    const target = await open('/project/website');
    const mock = createMockApi(seed());
    const mine = await mock.issues.create({ title: 'U9 inside', project_id: 1 });
    const other = await mock.issues.create({ title: 'U9 outside', project_id: 2 });
    const t = await open('/project/website', mock);
    await vi.waitFor(() => expect(t.textContent).toContain(mine.title));
    expect(t.textContent).not.toContain(other.title);
  });

  it('header_shows_progress_milestones_and_dates', async () => {
    const mock = createMockApi(seed([{ start_date: '2026-12-01', target_date: '2027-01-31' }]));
    await mock.projects.createMilestone(1, 'Beta', '2026-12-15');
    const target = await open('/project/website', mock);
    expect(target.querySelector('[aria-label="Start date"]')?.textContent).toContain(humanDate('2026-12-01'));
    expect(target.querySelector('[aria-label="Target date"]')?.textContent).toContain(humanDate('2027-01-31'));
    expect(target.textContent).toMatch(/\d+ \/ \d+/);
    expect(target.textContent).toContain('Beta');
    expect(target.textContent).toContain(humanDate('2026-12-15'));
  });

  it('header_has_no_project_name_hero_has_it_once', async () => {
    const t = await open('/project/website');
    expect(t.querySelector('header h1')?.textContent).not.toContain('Project 1');
    expect(t.querySelector('.hero')?.textContent?.split('Project 1').length).toBe(2);
  });

  it('toolbar_sits_between_milestones_and_issues', async () => {
    const t = await open('/project/website');
    const at = (sel: string) => t.querySelector(sel)!;
    const before = (a: Element, b: Element) => !!(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    expect(at('header').querySelector('[aria-label="Group by"]')).toBeNull();
    expect(before(at('.hero'), at('[aria-label=Milestones]'))).toBe(true);
    expect(before(at('[aria-label=Milestones]'), at('[data-testid="issues-toolbar"]'))).toBe(true);
    expect(before(at('[data-testid="issues-toolbar"]'), at('.issues'))).toBe(true);
    expect(at('[data-testid="issues-toolbar"]').textContent).toContain('Group');
  });

  it('empty_dates_read_set_start_and_set_target', async () => {
    const t = await open('/project/website', seed([{ start_date: null, target_date: null }]));
    expect(t.querySelector('[aria-label="Start date"]')?.textContent).toContain('Set start');
    expect(t.querySelector('[aria-label="Target date"]')?.textContent).toContain('Set target');
    expect(t.querySelector('input[type="date"]')).toBeNull();
  });

  it('date_field_edit_saves_the_project', async () => {
    const t = await open('/project/website', seed([{ start_date: null }]));
    t.querySelector<HTMLButtonElement>('button[aria-label="Start date"]')!.click();
    await vi.waitFor(() => expect(document.querySelector('input[aria-label="Start date value"]')).not.toBeNull());
    const el = document.querySelector<HTMLInputElement>('input[aria-label="Start date value"]')!;
    el.value = '2026-12-01';
    el.dispatchEvent(new Event('input', { bubbles: true }));
    await vi.waitFor(async () => expect((await api.projects.list())[0].start_date).toBe('2026-12-01'));
  });

  it('pin_toggle_pins_project', async () => {
    const target = await open('/project/website');
    target.querySelector<HTMLButtonElement>('button[aria-label="Pin project"]')!.click();
    await vi.waitFor(async () => {
      const pins = await api.pins.listResolved();
      expect(pins.map((p) => p.pin.target)).toEqual(['website']);
    });
  });

  it('unknown_slug_shows_not_found', async () => {
    const target = await open('/project/nope');
    expect(target.textContent).toMatch(/Project not found/);
    expect(target.querySelector('[data-testid="open-obsidian"]')).toBeNull();
  });

  it('progress_counts_done_over_total', async () => {
    const mock = createMockApi({ ...seed(), issues: [] });
    const done = (await mock.statuses.list()).find((x) => x.category === 'Completed')!;
    const a = await mock.issues.create({ title: 'U9 a', project_id: 1 });
    await mock.issues.create({ title: 'U9 b', project_id: 1 });
    await mock.issues.update(a.id, { status_id: done.id });
    const t = await open('/project/website', mock);
    expect(t.querySelector('[data-testid="progress"]')?.textContent).toMatch(/1\s*\/\s*2/);
  });

  it('layout_switch_changes_view', async () => {
    const t = await open('/project/website');
    const tab = (n: string) => [...t.querySelectorAll<HTMLElement>('[role="tab"]')].find((x) => x.textContent?.includes(n))!;
    expect(tab('List').getAttribute('data-state')).toBe('active');
    tab('Board').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    tab('Board').click();
    await vi.waitFor(() => expect(tab('Board').getAttribute('data-state')).toBe('active'));
  });

  it('milestone_crud', async () => {
    const t = await open('/project/website');
    const type = async (label: string, v: string) => {
      const el = t.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
      el.value = v;
      el.dispatchEvent(new Event('input', { bubbles: true }));
      await tick();
    };
    const click = (label: string) =>
      (t.querySelector<HTMLButtonElement>(`button[aria-label="${label}"]`) ?? [...t.querySelectorAll('button')].find((b) => b.textContent?.trim() === label.replace(' milestone', '')))!.click();
    click('Add milestone');
    await tick();
    await type('Milestone name', 'U9 Alpha');
    await type('Milestone date', '2027-02-01');
    click('Save milestone');
    await vi.waitFor(() => expect(t.textContent).toContain('U9 Alpha'));
    expect(t.textContent).toContain(humanDate('2027-02-01'));

    click('Edit U9 Alpha');
    await tick();
    await type('Milestone name', 'U9 Beta');
    click('Save milestone');
    await vi.waitFor(() => expect(t.textContent).toContain('U9 Beta'));
    expect(t.textContent).not.toContain('U9 Alpha');

    click('Delete U9 Beta');
    await vi.waitFor(() => expect(t.textContent).not.toContain('U9 Beta'));
  });

  it('rename_saves_new_name', async () => {
    const t = await open('/project/website');
    t.querySelector<HTMLButtonElement>('button[aria-label="Rename project"]')!.click();
    await tick();
    const el = t.querySelector<HTMLInputElement>('input[aria-label="Project name"]')!;
    el.value = 'U9 Renamed';
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.form!.requestSubmit();
    await vi.waitFor(async () => expect((await api.projects.list())[0].name).toBe('U9 Renamed'));
  });

  it('archive_needs_confirm', async () => {
    const t = await open('/project/website');
    t.querySelector<HTMLButtonElement>('button[aria-label="Archive project"]')!.click();
    await tick();
    expect((await api.projects.list())[0].archived_at).toBeNull();
    const ok = [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Archive')!;
    ok.click();
    await vi.waitFor(async () => expect((await api.projects.list())[0].archived_at).not.toBeNull());
  });
});

describe('ProjectPage details', () => {
  it('shows status, priority and labels, and a priority edit is one update', async () => {
    const mock = createMockApi(seed());
    const update = vi.spyOn(mock.projects, 'update');
    const target = await open('/project/website', mock);
    expect(target.querySelector('section[aria-label="Project details"]')?.textContent).toContain('Planned');
    target.querySelector<HTMLButtonElement>('section[aria-label="Project details"] button[aria-label="Priority"]')!.click();
    await vi.waitFor(() => expect(document.querySelector('[role="option"]')).not.toBeNull());
    [...document.querySelectorAll<HTMLButtonElement>('[role="option"]')].find((o) => o.textContent?.includes('High'))!.click();
    await vi.waitFor(() => expect(update).toHaveBeenCalledOnce());
    expect(update.mock.calls[0][0].priority).toBe(2);
  });

  it('lists custom properties that apply to projects, not issue-only ones', async () => {
    const mock = createMockApi(seed());
    await mock.properties.createDef({ key: 'sponsor', name: 'Sponsor', kind: 'Text', options: [], project_id: null, position: 0, applies_to: 'Projects' });
    const target = await open('/project/website', mock);
    const plate = target.querySelector('section[aria-label="Project details"]')!;
    await vi.waitFor(() => expect(plate.querySelector('button[aria-label="Sponsor"]')).not.toBeNull());
    expect(plate.querySelector('button[aria-label="Effort"]')).toBeNull();
  });

  it('shows the goals the project is linked to', async () => {
    const mock = createMockApi(seed());
    const g = await mock.goals.create('Launch', '2027-Q1', null);
    await mock.goals.link(g.id, base.id);
    const target = await open('/project/website', mock);
    await vi.waitFor(() => expect(target.querySelector('[aria-label="Goals"]')?.textContent).toContain('2027-Q1 · Launch'));
  });
});
