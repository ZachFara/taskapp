import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { attachKeys, type Ctx } from '../lib/commands';
import UndoToast from './UndoToast.svelte';

// Microtasks only: fake timers would stall a setTimeout-based wait.
const settle = async () => {
  for (let i = 0; i < 20; i++) await Promise.resolve();
  await tick();
};
const press = (target: Element, key: string, init: KeyboardEventInit = {}) =>
  target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
const toast = () => document.querySelector<HTMLElement>('.toast');
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
  it('shows the step label with an Undo action after a write', async () => {
    await write();
    const { undo } = await api.history.undoStatus();
    expect(undo).toBeTruthy();
    expect(toast()?.textContent).toContain(undo!);
    expect(toast()?.textContent).toContain('Undo (cmd+Z)');
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
