import { describe, expect, it } from 'vitest';
import Labels from './Labels.svelte';
import { mountSettings, press, setField } from './testing';

describe('Labels', () => {
  it('creates a label', async () => {
    const { target, calls } = await mountSettings(Labels);
    await setField(target, 'Label name', 'urgent');
    await press(target, 'Add label');
    expect(calls).toContain('labels.create');
    expect(target.textContent).toContain('urgent');
  });
});
