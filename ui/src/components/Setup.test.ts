import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import Setup from './Setup.svelte';

type TestApi = Parameters<typeof setApi>[0];

const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const field = (label: string) => document.querySelector<HTMLInputElement>(`input[aria-label="${label}"]`)!;
async function type(label: string, text: string) {
  field(label).value = text;
  field(label).dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
async function submit() {
  document.querySelector<HTMLButtonElement>('button[type="submit"]')!.click();
  await settle();
}

const ondone = vi.fn();
let setup: ReturnType<typeof vi.fn>;
let app: ReturnType<typeof mount> | null = null;

beforeEach(async () => {
  setup = vi.fn(async (name: string, key: string, timezone: string) => ({ setup_done: true, name, key, timezone }));
  setApi({ workspace: { setup } } as unknown as TestApi);
  ondone.mockClear();
  vi.spyOn(Intl.DateTimeFormat.prototype, 'resolvedOptions').mockReturnValue({
    timeZone: 'Europe/Lisbon',
  } as Intl.ResolvedDateTimeFormatOptions);
  app = mount(Setup, { target: document.body, props: { ondone } });
  await settle();
});
afterEach(() => {
  unmount(app!);
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('Setup', () => {
  it('setup_default_key_is_ABC_with_preview', () => {
    expect(field('Workspace key').value).toBe('ABC');
    expect(document.body.textContent).toContain('ABC-1');
  });

  it('setup_key_is_uppercased_and_previewed', async () => {
    await type('Workspace key', 'xyz');
    expect(field('Workspace key').value).toBe('XYZ');
    expect(document.body.textContent).toContain('XYZ-1');
  });

  it('setup_invalid_key_blocks_submit_with_message', async () => {
    await type('Workspace name', 'Home');
    await type('Workspace key', 'A1');
    await submit();
    expect(setup).not.toHaveBeenCalled();
    expect(ondone).not.toHaveBeenCalled();
    expect(document.querySelector('[role=alert]')?.textContent).toMatch(/2 to 5 letters/);
  });

  it('setup_key_longer_than_five_letters_is_invalid', async () => {
    await type('Workspace key', 'ABCDEF');
    expect(field('Workspace key').getAttribute('aria-invalid')).toBe('true');
  });

  it('setup_timezone_defaults_from_intl', () => {
    expect(document.querySelector('.cx-select')?.textContent).toContain('Europe/Lisbon');
  });

  it('setup_empty_name_blocks_submit', async () => {
    await submit();
    expect(setup).not.toHaveBeenCalled();
    expect(document.querySelector('[role=alert]')?.textContent).toMatch(/Name the workspace/);
  });

  it('setup_submit_calls_workspace_setup_then_ondone', async () => {
    await type('Workspace name', 'Home');
    await type('Workspace key', 'ab');
    await submit();
    expect(setup).toHaveBeenCalledWith('Home', 'AB', 'Europe/Lisbon');
    expect(ondone).toHaveBeenCalledOnce();
  });
});
