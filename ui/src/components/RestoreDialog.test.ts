import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { commandById } from '../lib/commands';
import RestoreDialog from './RestoreDialog.svelte';

type TestApi = Parameters<typeof setApi>[0];
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};

let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('Restore from export dialog', () => {
  it('names what is replaced, then restores the typed source', async () => {
    const fromExport = vi.fn(async (s: string) => s);
    setApi({ restore: { fromExport } } as unknown as TestApi);
    const onrestored = vi.fn();
    app = mount(RestoreDialog, { target: document.body, props: { open: true, onrestored } });
    await settle();
    expect(document.body.textContent).toMatch(/replaces everything in this workspace/);
    expect(document.body.textContent).toMatch(/backup of the current database is saved first/);
    const input = document.querySelector<HTMLInputElement>('input[aria-label="Export source"]')!;
    input.value = '~/Documents/Cairn';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await settle();
    document.querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
    await settle();
    expect(fromExport).toHaveBeenCalledWith('~/Documents/Cairn', null, true);
    expect(onrestored).toHaveBeenCalledWith('~/Documents/Cairn');
  });

  it('is a registered command in the command menu', async () => {
    await import('../lib/commands/defs/settings/restore');
    expect(commandById('settings.restore')?.label).toBe('Restore from export…');
  });
});
