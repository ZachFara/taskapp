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
let settingsSet: ReturnType<typeof vi.fn>;
let fromExport: ReturnType<typeof vi.fn>;
let app: ReturnType<typeof mount> | null = null;

beforeEach(async () => {
  setup = vi.fn(async (name: string, key: string, timezone: string) => ({ setup_done: true, name, key, timezone }));
  settingsSet = vi.fn(async () => {});
  fromExport = vi.fn(async (source: string) => source);
  setApi({ workspace: { setup }, settings: { set: settingsSet, get: async () => null }, restore: { fromExport } } as unknown as TestApi);
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
  it('setup_focuses_the_workspace_name_field', () => {
    expect(document.activeElement).toBe(field('Workspace name'));
  });

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

  it('setup_submit_calls_workspace_setup_and_offers_next_steps', async () => {
    await type('Workspace name', 'Home');
    await type('Workspace key', 'ab');
    await submit();
    expect(setup).toHaveBeenCalledWith('Home', 'AB', 'Europe/Lisbon');
    expect(ondone).not.toHaveBeenCalled();
    const labels = [...document.querySelectorAll('button')].map((b) => b.textContent?.trim());
    expect(labels).toEqual(expect.arrayContaining(['Start empty', 'Import from CSV']));
  });

  it('start_empty_finishes_setup', async () => {
    await type('Workspace name', 'Home');
    await submit();
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Start empty')!.click();
    expect(ondone).toHaveBeenCalledOnce();
  });

  it('import_from_csv_opens_the_import_page', async () => {
    await type('Workspace name', 'Home');
    await submit();
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Import from CSV')!.click();
    expect(location.pathname).toBe('/settings/import');
    expect(ondone).toHaveBeenCalledOnce();
  });

  it('the chosen theme is saved through the settings api', async () => {
    await type('Workspace name', 'Home');
    [...document.querySelectorAll('button')].find((b) => b.textContent?.trim() === 'Dark')!.click();
    await settle();
    await submit();
    expect(settingsSet).toHaveBeenCalledWith('theme', 'dark');
  });

  it('the theme choice is a segmented control', () => {
    expect(document.querySelector('[aria-label="Theme"].cx-seg')).not.toBeNull();
  });

  const choose = async (label: string) => {
    [...document.querySelectorAll('[role=tab], button')].find((b) => b.textContent?.trim() === label)!.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
  };

  it('offers restoring from an export beside starting a new workspace', async () => {
    const tabs = [...document.querySelectorAll('[aria-label="First run"] button, [aria-label="First run"] [role=tab]')].map((b) => b.textContent?.trim());
    expect(tabs).toEqual(['Start a new workspace', 'Restore from an export']);
    await choose('Restore from an export');
    expect(document.querySelector('input[aria-label="Export source"]')).not.toBeNull();
    expect(document.querySelector('input[aria-label="Workspace name"]')).toBeNull();
  });

  it('restore sends the source and the machine-settings choice, then finishes setup', async () => {
    await choose('Restore from an export');
    await type('Export source', ' git@github.com:me/tasks.git ');
    await submit();
    expect(fromExport).toHaveBeenCalledWith('git@github.com:me/tasks.git', null, true);
    expect(setup).not.toHaveBeenCalled();
    expect(ondone).toHaveBeenCalledOnce();
  });

  it('restore without a source says so and calls nothing', async () => {
    await choose('Restore from an export');
    await submit();
    expect(fromExport).not.toHaveBeenCalled();
    expect(document.querySelector('[role=alert]')?.textContent).toMatch(/folder or the Git address/);
  });

  it('a failed restore shows the error and stays on the screen', async () => {
    fromExport.mockRejectedValueOnce({ kind: 'Invalid', message: 'invalid: statuses.yaml is broken' });
    await choose('Restore from an export');
    await type('Export source', '/exports/cairn');
    await submit();
    expect(document.querySelector('[role=alert]')?.textContent).toContain('statuses.yaml is broken');
    expect(ondone).not.toHaveBeenCalled();
    expect(document.querySelector<HTMLButtonElement>('button[type="submit"]')!.disabled).toBe(false);
  });
});
