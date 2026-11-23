import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import { dismiss } from '../components/undoState.svelte';
import App from './App.svelte';

// The real QuickAdd is replaced by a probe so the root's wiring (not its insides) is under test.
vi.mock('../components/QuickAdd.svelte', async () => ({ default: (await import('./testing/QuickAddProbe.svelte')).default }));

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 12; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const q = (s: string) => document.querySelector<HTMLElement>(s);
const press = (key: string, init: KeyboardEventInit = {}) =>
  document.body.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));

async function open() {
  setApi(createMockApi());
  history.replaceState(null, '', '/my');
  app = mount(App, { target: document.body });
  await settle();
}
afterEach(() => {
  dismiss();
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('app root overlays', () => {
  it('question_mark_opens_the_real_cheat_sheet', async () => {
    await open();
    expect(q('[role=dialog]')).toBeNull();
    press('?', { shiftKey: true });
    await settle();
    expect(q('[role=dialog]')!.textContent).toContain('Keyboard shortcuts');
  });

  it('created_issue_closes_quick_add_and_toasts_with_undo', async () => {
    await open();
    press('c');
    await settle();
    expect(q('[data-quickadd]')).not.toBeNull();
    q('[data-quickadd]')!.click();
    await settle();
    expect(q('[data-quickadd]')).toBeNull();
    const toast = q('[role=status]')!;
    expect(toast.textContent).toContain('Created ABC-7');
    expect(toast.textContent).toContain('Undo');
  });
});
