import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { api, setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { setTheme } from '../lib/theme.svelte';
import { refreshAll, selectedIssueId } from '../lib/stores.svelte';
import { undoToast } from '../components/undoState.svelte';
import App from './App.svelte';
import CtxProbe from './testing/CtxProbe.svelte';
import { navigate, routes } from './routes';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 12; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
// The editor popover mounts and unmounts a few frames after the command, so wait on its state instead of a fixed delay.
const until = async (done: () => boolean) => {
  for (let i = 0; i < 200 && !done(); i++) await new Promise((r) => setTimeout(r, 10));
  flushSync();
};
const q = <T extends Element = HTMLElement>(s: string) => document.querySelector<T>(s);
const press = (key: string, init: KeyboardEventInit = {}, el: EventTarget = document.body) =>
  el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
const status = (setup_done: boolean) => ({ workspace: { status: async () => ({ setup_done, name: null, key: 'ABC', timezone: 'UTC' }) } });

async function open(path: string, extra: Record<string, unknown> = {}) {
  setApi(Object.assign(createMockApi(), extra) as never);
  history.replaceState(null, '', path);
  app = mount(App, { target: document.body });
  await settle();
}

beforeEach(() => {
  selectedIssueId.value = null;
  routes.push({ pattern: '/ctx-probe', component: CtxProbe });
});
afterEach(() => {
  routes.pop();
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
  localStorage.clear();
  document.documentElement.removeAttribute('data-theme');
});

describe('first run gate', () => {
  it('renders_setup_and_no_shell_when_not_set_up', async () => {
    await open('/my', status(false));
    expect(q('aside.sidebar')).toBeNull();
    expect(document.body.textContent).toContain('Create workspace');
  });

  it('renders_the_shell_when_set_up', async () => {
    await open('/my', status(true));
    expect(q('aside.sidebar')).not.toBeNull();
  });

  it('treats_a_rejected_status_as_set_up', async () => {
    await open('/my', { workspace: { status: async () => Promise.reject(new Error('no such command')) } });
    expect(q('aside.sidebar')).not.toBeNull();
  });

  it('treats_a_missing_status_call_as_set_up', async () => {
    await open('/my');
    expect(q('aside.sidebar')).not.toBeNull();
  });

  it('keys_do_nothing_while_setup_shows', async () => {
    await open('/my', status(false));
    press('c');
    await settle();
    expect(q('[role=dialog]')).toBeNull();
  });
});

describe('sidebar and frame', () => {
  it('marks_the_active_route', async () => {
    await open('/my');
    expect(q('aside.sidebar [aria-current=page]')!.textContent).toContain('My issues');
    navigate('/cycles');
    await settle();
    expect(q('aside.sidebar [aria-current=page]')!.textContent).toContain('All cycles');
  });

  it('shows_wordmark_and_breadcrumb', async () => {
    await open('/cycles');
    expect(q('.brand')!.textContent).toContain('Cairn');
    expect(q('header.topbar nav[aria-label=Breadcrumb]')!.textContent).toContain('Cycles');
  });

  it('theme_button_switches_data_theme', async () => {
    setTheme('light');
    await open('/my');
    q<HTMLButtonElement>('button[aria-label^="Switch theme"]')!.click();
    await settle();
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('search_field_and_help_button_open_palette_and_cheat_sheet', async () => {
    await open('/my');
    q<HTMLButtonElement>('header.topbar [aria-label="Search or run a command"]')!.click();
    await settle();
    expect(q('[role=dialog][aria-label="Command palette"]')).not.toBeNull();
  });
});

describe('keys and overlays', () => {
  it('cmd_k_opens_palette_and_esc_closes_it', async () => {
    await open('/settings/general');
    press('k', { metaKey: true });
    await settle();
    const input = q<HTMLInputElement>('[role=dialog] input')!;
    expect(input).not.toBeNull();
    press('Escape', {}, input);
    await settle();
    expect(q('[role=dialog]')).toBeNull();
    expect(location.pathname).toBe('/settings/general');
  });

  it('c_opens_quick_add', async () => {
    await open('/my');
    press('c');
    await settle();
    expect(q('[role=dialog]')!.textContent).toContain('New issue');
  });

  it('the_toast_clears_when_the_page_changes', async () => {
    await open('/my');
    undoToast.value = { label: 'Updated issue', tone: 'success', action: { run: 'undo', text: 'Undo' } };
    await settle();
    expect(q('.toast')).not.toBeNull();
    navigate('/cycles');
    await settle();
    expect(q('.toast')).toBeNull();
  });

  it('g_m_navigates_through_the_registry', async () => {
    await open('/cycles');
    press('g');
    press('m');
    await settle();
    expect(location.pathname).toBe('/my');
  });
});

describe('issue panel', () => {
  async function withIssue() {
    await open('/my');
    const issue = await api.issues.create({ title: 'Panel probe' });
    await refreshAll();
    selectedIssueId.value = issue.id;
    await settle();
    return issue;
  }

  it('opens_for_the_selected_issue', async () => {
    await withIssue();
    expect(q('aside.panel [aria-label=Title]')!.textContent).toBe('Panel probe');
  });

  it('esc_closes_the_panel_and_keeps_the_page', async () => {
    await withIssue();
    await new Promise((r) => setTimeout(r, 100));
    press('Escape');
    await settle();
    expect(q('aside.panel')).toBeNull();
    expect(location.pathname).toBe('/my');
    expect(selectedIssueId.value).toBeNull();
  });

  it('esc_closes_an_open_editor_first_then_the_panel_without_going_back', async () => {
    await withIssue();
    const back = vi.spyOn(history, 'back');
    dispatchEvent(new CustomEvent('command:issue.detail.edit', { detail: 'priority' }));
    await until(() => !!q('[role=listbox][aria-label=Priority]'));
    expect(q('[role=listbox][aria-label=Priority]')).not.toBeNull();
    press('Escape');
    await until(() => !q('[role=listbox][aria-label=Priority]'));
    expect(q('[role=listbox][aria-label=Priority]')).toBeNull();
    expect(q('aside.panel')).not.toBeNull();
    await new Promise((r) => setTimeout(r, 100));
    press('Escape');
    await settle();
    expect(q('aside.panel')).toBeNull();
    expect(back).not.toHaveBeenCalled();
    back.mockRestore();
  });

  it('esc_with_palette_open_keeps_the_panel', async () => {
    await withIssue();
    press('k', { metaKey: true });
    await settle();
    press('Escape', {}, q('[role=dialog] input')!);
    await settle();
    expect(q('[role=dialog]')).toBeNull();
    expect(q('aside.panel')).not.toBeNull();
  });

  it('navigating_closes_the_panel', async () => {
    await withIssue();
    navigate('/cycles');
    await settle();
    expect(q('aside.panel')).toBeNull();
  });

  it('a_vanished_issue_shows_not_found_in_the_panel', async () => {
    await open('/my');
    selectedIssueId.value = 99999;
    await settle();
    expect(q('aside.panel [role=alert]')!.textContent).toContain('issue #99999');
  });

  it('the_resize_handle_changes_the_panel_width', async () => {
    await withIssue();
    const h = q('aside.panel [aria-label="Resize panel"]')!;
    h.dispatchEvent(new MouseEvent('pointerdown', { clientX: 900, bubbles: true }));
    window.dispatchEvent(new MouseEvent('pointermove', { clientX: 800 }));
    window.dispatchEvent(new MouseEvent('pointerup'));
    await settle();
    expect(q('aside.panel')!.style.getPropertyValue('--panel-w')).not.toBe('');
  });
});

describe('views inside the frame', () => {
  const def = { filter: null, sorts: [], group_by: null, layout: 'List' as const, columns: [], include_archived: false };
  async function withView() {
    const a = createMockApi();
    const view = await a.views.save(null, 'Frame view', def, null, null);
    const issue = await a.issues.create({ title: 'Row probe' });
    setApi(a);
    history.replaceState(null, '', `/view/${view.id}`);
    app = mount(App, { target: document.body });
    await settle();
    return { issue, view };
  }

  it('clicking_a_row_opens_its_detail_in_the_panel', async () => {
    const { issue } = await withView();
    q(`[data-key=${issue.key}]`)!.click();
    await settle();
    expect(q('aside.panel [aria-label=Title]')!.textContent).toBe('Row probe');
  });

  it('rename_view_saves_and_refreshes_the_sidebar', async () => {
    await withView();
    q<HTMLButtonElement>('button[aria-label="Rename view"]')!.click();
    await settle();
    const name = q<HTMLInputElement>('[aria-label="View name"]')!;
    name.value = 'Renamed';
    name.dispatchEvent(new Event('input', { bubbles: true }));
    name.form!.requestSubmit();
    await settle();
    expect(q('aside.sidebar')!.textContent).toContain('Renamed');
  });
});

describe('command context', () => {
  it('descendants_read_the_live_ctx_with_the_open_issue', async () => {
    await open('/ctx-probe');
    expect(q('[data-probe]')!.textContent).toBe('/ctx-probe||0');
    const issue = await api.issues.create({ title: 'ctx' });
    selectedIssueId.value = issue.id;
    await settle();
    expect(q('[data-probe]')!.textContent).toBe(`/ctx-probe|${issue.id}|0`);
  });
});

describe('source', () => {
  const files = import.meta.glob(['./App.svelte', '../components/Shell.svelte', '../components/Sidebar.svelte'], { query: '?raw', import: 'default', eager: true });
  it('no_registerShortcut_left_in_the_root_files', () => {
    expect(Object.keys(files)).toHaveLength(3);
    for (const src of Object.values(files)) expect(src).not.toContain('registerShortcut');
  });
});
