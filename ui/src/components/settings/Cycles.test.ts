import { describe, expect, it } from 'vitest';
import Cycles from './Cycles.svelte';
import { mountSettings, press, setField } from './testing';

describe('Cycles', () => {
  it('creates a cycle from its dates', async () => {
    const { target, calls } = await mountSettings(Cycles);
    await setField(target, 'Cycle start', '2026-12-01');
    await setField(target, 'Cycle end', '2026-12-14');
    await press(target, 'Add cycle');
    expect(calls).toContain('cycles.create');
    expect(target.textContent).toContain('2026-12-01');
  });
});
