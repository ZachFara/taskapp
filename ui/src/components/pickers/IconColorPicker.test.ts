import { afterEach, describe, expect, it } from 'vitest';
import { flushSync, mount, tick, unmount } from 'svelte';
import IconColorPicker from './IconColorPicker.svelte';

const flush = () => new Promise((r) => setTimeout(r, 0)).then(() => tick());
const live: { app: object; el: Element }[] = [];
afterEach(() => live.splice(0).forEach(({ app, el }) => { void unmount(app); el.remove(); }));

async function open(value = { icon: null as string | null, color: null as string | null }) {
  const emitted: { icon: string | null; color: string | null }[] = [];
  const el = document.body.appendChild(document.createElement('div'));
  const app = mount(IconColorPicker, { target: el, props: { value, label: 'Status icon', onchange: (v: never) => emitted.push(v) } });
  live.push({ app, el });
  await flush();
  el.querySelector<HTMLElement>('button[aria-label="Status icon"]')!.click();
  flushSync();
  await flush();
  return emitted;
}
const byLabel = (l: string) => document.body.querySelector<HTMLElement>(`[aria-label="${l}"]`)!;
const click = async (l: string) => { byLabel(l).click(); await flush(); };

describe('IconColorPicker', () => {
  it('emits icon:<id> keeping the colour', async () => {
    const out = await open({ icon: null, color: 'blue' });
    await click('flag');
    expect(out).toEqual([{ icon: 'icon:flag', color: 'blue' }]);
  });

  it('emits a palette key keeping the icon', async () => {
    const out = await open({ icon: 'icon:bolt', color: null });
    await click('purple');
    expect(out).toEqual([{ icon: 'icon:bolt', color: 'purple' }]);
  });

  it('search narrows the icon grid', async () => {
    await open();
    const input = byLabel('Search icons') as HTMLInputElement;
    input.value = 'cal';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await flush();
    expect(byLabel('calendar')).not.toBeNull();
    expect(document.body.querySelector('[aria-label="flag"]')).toBeNull();
  });

  it('finds and selects an additional icon without losing the colour', async () => {
    const out = await open({ icon: null, color: 'blue' });
    const input = byLabel('Search icons') as HTMLInputElement;
    input.value = 'rocket';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    await flush();
    const choice = byLabel('rocket');
    expect(choice.querySelector('svg path')).not.toBeNull();
    await click('rocket');
    expect(out).toEqual([{ icon: 'icon:rocket', color: 'blue' }]);
  });

  it('emits emoji:<x> from the list and from a pasted emoji', async () => {
    const out = await open({ icon: null, color: 'red' });
    await click('Emoji');
    const first = document.body.querySelector<HTMLElement>('[data-emoji]')!;
    first.click();
    await flush();
    expect(out[0]).toEqual({ icon: `emoji:${first.dataset.emoji}`, color: 'red' });
    const paste = byLabel('Paste an emoji') as HTMLInputElement;
    paste.value = '🦊';
    paste.dispatchEvent(new Event('input', { bubbles: true }));
    await flush();
    document.body.querySelector<HTMLElement>('form.paste button')!.click();
    await flush();
    expect(out[1]).toEqual({ icon: 'emoji:🦊', color: 'red' });
  });

  it('clearing emits nulls', async () => {
    const out = await open({ icon: 'icon:flag', color: 'blue' });
    await click('Clear icon and colour');
    expect(out).toEqual([{ icon: null, color: null }]);
  });
});
