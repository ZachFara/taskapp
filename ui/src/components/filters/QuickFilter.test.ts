import { mount, tick } from 'svelte';
import { describe, expect, it } from 'vitest';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';
import { features } from '../../lib/features.svelte';
import QuickFilter from './QuickFilter.svelte';
import type { Chip } from './model';

const click = (text: string) =>
  [...document.querySelectorAll<HTMLElement>('button,[role="menuitem"]')].find((e) => e.textContent?.trim() === text)!.click();
const settle = async () => {
  await new Promise((r) => setTimeout(r, 20));
  await tick();
};

async function open(withGroup = true) {
  document.body.innerHTML = '';
  const mock = createMockApi();
  if (withGroup) {
    const g = await mock.label.groupsCreate('Context');
    for (const n of ['@home', '@phone']) await mock.labels.update({ ...(await mock.labels.create(n, 'blue', null)), group_id: g.id });
  }
  setApi(mock as unknown as Api);
  // The component edits this array in place.
  const state = { chips: [] as Chip[] };
  mount(QuickFilter, { target: document.body, props: { chips: state.chips, onclose: () => {} } });
  await settle();
  return { mock, state };
}

describe('QuickFilter Context button', () => {
  it('one click on a context filters to that label, a second click swaps it', async () => {
    features.contexts = true;
    const { mock, state } = await open();
    const [home, phone] = (await mock.labels.list()).slice(-2);
    click('Context');
    await settle();
    click('@home');
    await settle();
    expect(state.chips.map((c) => [c.field, c.op, c.value])).toEqual([[{ Fixed: 'Label' }, 'In', [home.id]]]);
    click('Context');
    await settle();
    click('@phone');
    await settle();
    expect(state.chips.map((c) => c.value)).toEqual([[phone.id]]);
  });

  it('is absent without a Context group or with the feature off', async () => {
    features.contexts = true;
    await open(false);
    expect([...document.querySelectorAll('button')].some((b) => b.textContent?.trim() === 'Context')).toBe(false);
    features.contexts = false;
    await open();
    expect([...document.querySelectorAll('button')].some((b) => b.textContent?.trim() === 'Context')).toBe(false);
    features.contexts = true;
  });
});
