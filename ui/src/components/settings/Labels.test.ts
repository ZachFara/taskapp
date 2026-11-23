import { describe, expect, it } from 'vitest';
import { mount } from 'svelte';
import Labels from './Labels.svelte';
import RowDialogHost from '../RowDialogHost.svelte';
import '../../lib/commands';
import { clickLabel, field, mountSettings, press, setField } from './testing';

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
    await clickLabel('Delete urgent');
    expect(calls).not.toContain('labels.delete');
    await press(document.body, 'Delete');
    expect(calls.filter((c) => c === 'labels.delete')).toHaveLength(1);
  });

  it('a refused create shows its message', async () => {
    const { target } = await mountSettings(Labels);
    await add(target, 'urgent');
    await add(target, 'urgent');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/exists/);
  });

  it('row actions name themselves in a tooltip on hover', async () => {
    const { target } = await mountSettings(Labels);
    const del = target.querySelector<HTMLButtonElement>('button[aria-label^="Delete "]')!;
    del.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse' }));
    await new Promise((r) => setTimeout(r, 700));
    expect(document.querySelector('.cx-tip')?.textContent).toMatch(/^Delete /);
  });
});
