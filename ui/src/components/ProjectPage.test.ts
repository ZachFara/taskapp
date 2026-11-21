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
  await vi.waitFor(() => expect(target.textContent).not.toMatch(/Loading/));
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
    expect(button?.textContent).toContain('Open in Obsidian');
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
    const keys = [...target.querySelectorAll('[data-issue-key]')].map((el) => el.getAttribute('data-issue-key'));
    const expected = fixtures.issues.filter((i) => i.project_id === 1).map((i) => i.key);
    expect(expected.length).toBeGreaterThan(0);
    expect(keys.sort()).toEqual(expected.sort());
  });

  it('header_shows_progress_milestones_and_dates', async () => {
    const mock = createMockApi(seed([{ start_date: '2026-12-01', target_date: '2027-01-31' }]));
    await mock.projects.createMilestone(1, 'Beta', '2026-12-15');
    const target = await open('/project/home', mock);
    expect(target.textContent).toContain('2026-12-01');
    expect(target.textContent).toContain('2027-01-31');
    expect(target.textContent).toMatch(/\d+ \/ \d+ done/);
    expect(target.textContent).toContain('Beta');
    expect(target.textContent).toContain('2026-12-15');
  });

  it('pin_toggle_pins_project', async () => {
    const target = await open('/project/home');
    target.querySelector<HTMLButtonElement>('[data-testid="pin-toggle"]')!.click();
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
});
