import { describe, expect, it, vi } from 'vitest';
import { mount } from 'svelte';
import Labels from './Labels.svelte';
import RowDialogHost from '../RowDialogHost.svelte';
import '../../lib/commands';
import { clickLabel, field, mountSettings, pick, press, setField, toggle } from './testing';
import { setTooltips } from '../../lib/tooltips.svelte';

async function add(target: Element, name: string) {
  await setField(target, 'Label name', name);
  await press(target, 'Add label');
}

describe('Labels', () => {
  it('creates a label and clears the field', async () => {
    const { target, calls } = await mountSettings(Labels);
    await add(target, 'urgent');
    expect(calls.filter((c) => c === 'labels.create')).toHaveLength(1);
    expect(field(target, 'Label name').value).toBe('');
    expect(field(target, 'Name of urgent')).toBeTruthy();
  });

  it('renames inline with one call', async () => {
    const { target, calls } = await mountSettings(Labels);
    await add(target, 'urgent');
    await setField(target, 'Name of urgent', 'asap');
    expect(calls.filter((c) => c === 'labels.update')).toHaveLength(1);
  });

  it('picker sets icon and colour with one call', async () => {
    const { target, calls, api } = await mountSettings(Labels);
    await add(target, 'urgent');
    await clickLabel('Icon and colour of urgent');
    await clickLabel('bolt');
    expect(calls.filter((c) => c === 'labels.update')).toHaveLength(1);
    expect((await api.labels.list()).find((l) => l.name === 'urgent')!.icon).toBe('icon:bolt');
  });

  it('delete asks first, then calls once', async () => {
    const { target, calls } = await mountSettings(Labels);
    await add(target, 'urgent');
    mount(RowDialogHost, { target: document.body });
    await clickLabel('More actions for urgent');
    expect(calls).not.toContain('labels.delete');
    await vi.waitFor(() => [...document.querySelectorAll<HTMLElement>('[role="menuitem"]')].find((e) => e.textContent?.trim() === 'Delete urgent')!.click());
    await press(document.body, 'Delete');
    expect(calls.filter((c) => c === 'labels.delete')).toHaveLength(1);
  });

  it('creates a group, makes it exclusive and deletes it after asking', async () => {
    const { target, calls, api } = await mountSettings(Labels);
    await setField(target, 'Group name', 'Context');
    await press(target, 'Add group');
    expect(calls.filter((c) => c === 'label.groupsCreate')).toHaveLength(1);
    expect(field(target, 'Group name').value).toBe('');
    await toggle(target, 'Exclusive: Context');
    expect((await api.label.groupsList())[0].exclusive).toBe(true);
    await clickLabel('Delete group Context');
    expect(calls).not.toContain('label.groupsDelete');
    await press(document.body, 'Delete group');
    expect(calls.filter((c) => c === 'label.groupsDelete')).toHaveLength(1);
  });

  it('a label moves into a group from its row', async () => {
    const { target, api } = await mountSettings(Labels);
    await setField(target, 'Group name', 'Context');
    await press(target, 'Add group');
    await add(target, 'urgent');
    await pick(target, 'Group of urgent', 'Context');
    const g = (await api.label.groupsList())[0];
    expect((await api.labels.list()).find((l) => l.name === 'urgent')!.group_id).toBe(g.id);
  });

  it('a refused create shows its message', async () => {
    const { target } = await mountSettings(Labels);
    await add(target, 'urgent');
    await add(target, 'urgent');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/exists/);
  });

  it('row actions name themselves in a tooltip on hover', async () => {
    setTooltips(true);
    const { target } = await mountSettings(Labels);
    const more = target.querySelector<HTMLButtonElement>('button[aria-label^="More actions for "]')!;
    more.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse' }));
    await new Promise((r) => setTimeout(r, 700));
    expect(document.querySelector('.cx-tip')?.textContent).toMatch(/^More actions for /);
  });
});
