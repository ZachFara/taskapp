import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, tick, unmount } from 'svelte';
import { api, setApi, type Api } from '../lib/api';
import { createMockApi, fixtures, type MockSeed } from '../lib/api.mock';
import type { Project } from '../lib/bindings';
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
  it('doc_link_button_shown_only_when_set', async () => {
    const without = await open('/project/home');
    expect(without.querySelector('[data-testid="open-obsidian"]')).toBeNull();
    mounted.forEach((m) => void unmount(m));
    mounted = [];
    document.body.innerHTML = '';

    const withLink = await open('/project/home', seed([{ doc_link: 'obsidian://open?vault=Ark&file=Home' }]));
    const button = withLink.querySelector<HTMLButtonElement>('[data-testid="open-obsidian"]');
    expect(button?.textContent).toContain('Open doc');
  });

  it('doc_link_not_obsidian_or_https_renders_no_button', async () => {
    for (const doc_link of ['javascript:alert(1)', 'http://example.com/doc', 'file:///etc/passwd', 'ftp://x']) {
      const target = await open('/project/home', seed([{ doc_link }]));
      expect(target.querySelector('[data-testid="open-obsidian"]'), doc_link).toBeNull();
      mounted.forEach((m) => void unmount(m));
      mounted = [];
      document.body.innerHTML = '';
    }
  });

  it('empty_description_hidden', async () => {
    const target = await open('/project/home', seed([{ description: null }]));
    expect(target.querySelector('[data-testid="description"]')).toBeNull();
    expect(target.textContent).toContain('Add description');
  });

  it('description_shown_when_present', async () => {
    const target = await open('/project/home', seed([{ description: 'Ship the rewrite' }]));
    expect(target.querySelector('[data-testid="description"]')?.textContent).toContain('Ship the rewrite');
    expect(target.textContent).not.toContain('Add description');
  });

  it('project_issues_filtered_to_project', async () => {
    const target = await open('/project/home');
    const mock = createMockApi(seed());
    const mine = await mock.issues.create({ title: 'U9 inside', project_id: 1 });
    const other = await mock.issues.create({ title: 'U9 outside', project_id: 2 });
    const t = await open('/project/home', mock);
    await vi.waitFor(() => expect(t.textContent).toContain(mine.title));
    expect(t.textContent).not.toContain(other.title);
  });

  it('header_shows_progress_milestones_and_dates', async () => {
    const mock = createMockApi(seed([{ start_date: '2026-12-01', target_date: '2027-01-31' }]));
    await mock.projects.createMilestone(1, 'Beta', '2026-12-15');
    const target = await open('/project/home', mock);
    const dates = [...target.querySelectorAll<HTMLInputElement>('input[type="date"]')].map((i) => i.value);
    expect(dates).toContain('2026-12-01');
    expect(dates).toContain('2027-01-31');
    expect(target.textContent).toMatch(/\d+ \/ \d+/);
    expect(target.textContent).toContain('Beta');
    expect(target.textContent).toContain('2026-12-15');
  });

  it('pin_toggle_pins_project', async () => {
    const target = await open('/project/home');
    target.querySelector<HTMLButtonElement>('button[aria-label="Pin project"]')!.click();
    await vi.waitFor(async () => {
      const pins = await api.pins.listResolved();
      expect(pins.map((p) => p.pin.target)).toEqual(['home']);
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
    const t = await open('/project/home', mock);
    expect(t.querySelector('[data-testid="progress"]')?.textContent).toMatch(/1\s*\/\s*2/);
  });

  it('layout_switch_changes_view', async () => {
    const t = await open('/project/home');
    const tab = (n: string) => [...t.querySelectorAll<HTMLElement>('[role="tab"]')].find((x) => x.textContent?.includes(n))!;
    expect(tab('List').getAttribute('data-state')).toBe('active');
    tab('Board').dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
    tab('Board').click();
    await vi.waitFor(() => expect(tab('Board').getAttribute('data-state')).toBe('active'));
  });

  it('milestone_crud', async () => {
    const t = await open('/project/home');
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
    expect(t.textContent).toContain('2027-02-01');

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
    const t = await open('/project/home');
    t.querySelector<HTMLButtonElement>('button[aria-label="Rename project"]')!.click();
    await tick();
    const el = t.querySelector<HTMLInputElement>('input[aria-label="Project name"]')!;
    el.value = 'U9 Renamed';
    el.dispatchEvent(new Event('input', { bubbles: true }));
    el.form!.requestSubmit();
    await vi.waitFor(async () => expect((await api.projects.list())[0].name).toBe('U9 Renamed'));
  });

  it('archive_needs_confirm', async () => {
    const t = await open('/project/home');
    t.querySelector<HTMLButtonElement>('button[aria-label="Archive project"]')!.click();
    await tick();
    expect((await api.projects.list())[0].archived_at).toBeNull();
    const ok = [...document.querySelectorAll<HTMLButtonElement>('button')].find((b) => b.textContent?.trim() === 'Archive')!;
    ok.click();
    await vi.waitFor(async () => expect((await api.projects.list())[0].archived_at).not.toBeNull());
  });
});
