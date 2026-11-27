import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Select from './Select.svelte';

let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});
const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const items = [
  { value: '1', label: 'Todo', glyph: { category: 'Unstarted' as const, color: 'blue' } },
  { value: '2', label: 'Done', glyph: { category: 'Completed' as const, icon: 'icon:folder', color: 'green' } },
  { value: '3', label: 'Plain' },
];
const open = async (props: Record<string, unknown>) => {
  app = mount(Select, { target: document.body, props: { label: 'Status', items, ...props } });
  flushSync();
  const t = document.querySelector<HTMLElement>('.cx-select')!;
  t.focus();
  t.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
  await settle();
};

describe('Select with glyphs', () => {
  it('draws each option with its status glyph or entity icon, none when absent', async () => {
    await open({ value: '' });
    const opts = [...document.querySelectorAll('[role=option]')];
    expect(opts).toHaveLength(3);
    expect(opts[0].querySelector('svg[data-category=Unstarted]')).not.toBeNull();
    expect(opts[1].querySelector('.entity')?.getAttribute('data-color')).toBe('green');
    expect(opts[2].querySelector('svg')).toBeNull();
  });
  it('shows the chosen option glyph in the trigger', async () => {
    app = mount(Select, { target: document.body, props: { label: 'Status', items, value: '1' } });
    flushSync();
    expect(document.querySelector('.cx-select svg[data-category=Unstarted]')).not.toBeNull();
  });
  it('picks with the keyboard', async () => {
    await open({ value: '' });
    const key = (k: string) => (document.activeElement as HTMLElement).dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }));
    key('ArrowDown');
    key('Enter');
    await settle();
    expect(document.querySelector('.cx-select .v')?.textContent).not.toBe('Select');
    expect(document.querySelector('[role=listbox]')).toBeNull();
  });
});
