import { describe, expect, it } from 'vitest';
import Statuses from './Statuses.svelte';
import { field, mountSettings, press, setField } from './testing';

describe('Statuses', () => {
  it('lists statuses in position order', async () => {
    const { target } = await mountSettings(Statuses);
    expect(target.textContent?.indexOf('Backlog')).toBeLessThan(target.textContent?.indexOf('Done') ?? -1);
  });

  it('creates a status', async () => {
    const { target, calls } = await mountSettings(Statuses);
    await setField(target, 'New status name', 'Review');
    await press(target, 'Add status');
    expect(calls).toContain('statuses.create');
    expect(field(target, 'New status name').value).toBe('');
    expect(target.textContent).toContain('Review');
  });
});
