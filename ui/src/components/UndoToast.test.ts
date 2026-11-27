import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { attachKeys, type Ctx } from '../lib/commands';
import UndoToast from './UndoToast.svelte';
import { ICONS } from '../ui/icons';
import { dismiss, stepLabel } from './undoState.svelte';

// Glyph assertions below are the Mac ones whatever machine runs them.
beforeEach(() => void vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel'));

// Microtasks only: fake timers would stall a setTimeout-based wait.
const settle = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve();
  await tick();
};
const press = (target: Element, key: string, init: KeyboardEventInit = {}) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
// The newest message: messages stack, oldest first.
const toast = () => [...document.querySelectorAll<HTMLElement>('.toast')].at(-1) ?? null;
const toastButton = (name: string) =>
  [...document.querySelectorAll<HTMLElement>('.toast button')].find((b) => b.textContent?.trim().startsWith(name));

let api: ReturnType<typeof createMockApi>;
let ctx: Ctx;
let detach: () => void;
let app: ReturnType<typeof mount>;

// A real write through the api, then the signal the app sends after a write.
async function write() {
  await api.issues.create({ title: 'Undo me' });
  window.dispatchEvent(new Event('history:wrote'));
  await settle();
}

beforeEach(async () => {
  api = createMockApi();
  setApi(api);
  ctx = { route: '/', selection: [], focusedIssueId: null, openIssueId: null, api, navigate: () => {} };
  detach = attachKeys(() => ctx);
  dismiss();
  app = mount(UndoToast, { target: document.body });
  await settle();
});
afterEach(() => {
  detach();
  unmount(app);
  document.body.innerHTML = '';
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe('undo keys', () => {
  it('mod+z typed in a text field does not run undo', async () => {
    const undo = vi.spyOn(api.history, 'undo');
    const input = document.createElement('input');
    document.body.append(input);
    input.focus();
    press(input, 'z', { metaKey: true });
    await settle();
    expect(undo).not.toHaveBeenCalled();
  });

  it('mod+z outside a text field runs undo once, and u does too', async () => {
    const undo = vi.spyOn(api.history, 'undo').mockResolvedValue({ undo: null, redo: null });
    vi.spyOn(api.history, 'undoStatus').mockResolvedValue({ undo: 'a step', redo: null });
    press(document.body, 'z', { metaKey: true });
    await settle();
    expect(undo).toHaveBeenCalledTimes(1);
    press(document.body, 'u');
    await settle();
    expect(undo).toHaveBeenCalledTimes(2);
  });
});

describe('UndoToast', () => {
  it('a write that adds no step raises no toast when history was already non-empty on load', async () => {
    unmount(app);
    await api.issues.create({ title: 'Earlier session' });
    app = mount(UndoToast, { target: document.body });
    await settle();
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(toast()).toBeNull();
  });

  it('names a raw command step in plain words', async () => {
    vi.spyOn(api.history, 'undoStatus').mockResolvedValue({ undo: 'issues_bulk_update', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(toast()?.textContent).toContain('Updated issues');
    expect(toast()?.textContent).not.toContain('issues_bulk_update');
  });

  it('sentence-cases a raw command with no entry in the table', async () => {
    vi.spyOn(api.history, 'undoStatus').mockResolvedValue({ undo: 'widgets_frobnicate_all', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(toast()?.textContent).toContain('Widgets frobnicate all');
  });

  it('shows the step label with an Undo action after a write', async () => {
    await write();
    const { undo } = await api.history.undoStatus();
    expect(undo).toBeTruthy();
    expect(toast()?.textContent).toContain(stepLabel(undo!));
    expect(toastButton('Undo')?.textContent).not.toMatch(/cmd/i);
    expect(toastButton('Undo')?.querySelector('kbd')?.textContent).toBe('⌘Z');
  });

  it('shows a prose step label from the backend as it is', async () => {
    vi.spyOn(api.history, 'undoStatus').mockResolvedValue({ undo: 'Set status to Done on 3 issues', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(toast()?.textContent).toContain('Set status to Done on 3 issues');
  });

  it('does not use the bell for a step or an undo', async () => {
    await write();
    const probe = document.createElement('svg');
    probe.innerHTML = ICONS.bell; // the browser's own serialisation, so the comparison is exact
    const bell = probe.innerHTML;
    expect(bell).not.toBe('');
    expect(toast()?.querySelector('.lead')?.innerHTML).not.toContain(bell);
    toastButton('Undo')!.click();
    await settle();
    expect(toast()?.querySelector('.lead')?.innerHTML).not.toContain(bell);
  });

  it('a display-only view change raises no toast', async () => {
    vi.spyOn(api.history, 'undoStatus').mockResolvedValue({ undo: 'views_save', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(toast()).toBeNull();
  });

  it('hides itself after 6 seconds', async () => {
    vi.useFakeTimers();
    await write();
    expect(toast()).not.toBeNull();
    vi.advanceTimersByTime(5_999);
    await settle();
    expect(toast()).not.toBeNull();
    vi.advanceTimersByTime(1);
    await settle();
    expect(toast()).toBeNull();
  });

  it('a second message stacks under the first instead of replacing it', async () => {
    vi.spyOn(api.history, 'undoStatus').mockResolvedValueOnce({ undo: 'issues_update', redo: null }).mockResolvedValueOnce({ undo: 'issues_delete', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    expect(document.querySelectorAll('.toast').length).toBe(2);
    expect(document.querySelectorAll('.toast')[0].textContent).toContain('Updated issue');
  });

  it('each message leaves six seconds after its own arrival', async () => {
    vi.useFakeTimers();
    vi.spyOn(api.history, 'undoStatus').mockResolvedValueOnce({ undo: 'issues_update', redo: null }).mockResolvedValueOnce({ undo: 'issues_delete', redo: null });
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    vi.advanceTimersByTime(4_000);
    window.dispatchEvent(new Event('history:wrote'));
    await settle();
    vi.advanceTimersByTime(2_500);
    await settle();
    expect(document.querySelectorAll('.toast').length).toBe(1);
    expect(toast()?.textContent).toContain('Deleted issue');
  });

  it('clicking Undo runs undo and then offers Redo', async () => {
    await write();
    const undo = vi.spyOn(api.history, 'undo');
    toastButton('Undo')!.click();
    await settle();
    expect(undo).toHaveBeenCalledTimes(1);
    expect(toast()?.textContent).toContain('Undone');
    expect(toastButton('Redo')).toBeDefined();
  });

  it('redo after undo runs redo once', async () => {
    await write();
    toastButton('Undo')!.click();
    await settle();
    const redo = vi.spyOn(api.history, 'redo');
    press(document.body, 'Z', { metaKey: true, shiftKey: true });
    await settle();
    expect(redo).toHaveBeenCalledTimes(1);
  });

  it('shows the backend error text when undo is refused', async () => {
    await write();
    vi.spyOn(api.history, 'undo').mockRejectedValue({ kind: 'Conflict', message: 'Undo refused: ABC-1 changed since' });
    toastButton('Undo')!.click();
    await settle();
    expect(toast()?.textContent).toContain('Undo refused: ABC-1 changed since');
    expect(toast()?.dataset.tone).toBe('error');
  });
});
