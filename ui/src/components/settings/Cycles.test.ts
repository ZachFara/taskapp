import { describe, expect, it } from 'vitest';
import Cycles from './Cycles.svelte';
import { clickLabel, field, mountSettings, press, setField } from './testing';

async function add(target: Element, name: string) {
  await setField(target, 'Cycle name', name);
  await setField(target, 'Cycle start', '2026-12-01');
  await setField(target, 'Cycle end', '2026-12-14');
  await press(target, 'Add cycle');
}

describe('Cycles', () => {
  it('creates a cycle from its dates once', async () => {
    const { target, calls } = await mountSettings(Cycles);
    await add(target, 'Winter');
    expect(calls.filter((c) => c === 'cycles.create')).toHaveLength(1);
    expect(field(target, 'Start of Winter').value).toBe('2026-12-01');
  });

  it('renames inline with one call', async () => {
    const { target, calls } = await mountSettings(Cycles);
    await add(target, 'Winter');
    await setField(target, 'Name of Winter', 'Snow');
    expect(calls.filter((c) => c === 'cycles.update')).toHaveLength(1);
  });

  it('changing a date saves it with one call', async () => {
    const { target, calls, api } = await mountSettings(Cycles);
    await add(target, 'Winter');
    await setField(target, 'End of Winter', '2026-12-20');
    expect(calls.filter((c) => c === 'cycles.update')).toHaveLength(1);
    expect((await api.cycles.list())[0].end_date).toBe('2026-12-20');
  });

  it('picker and delete each call once', async () => {
    const { target, calls } = await mountSettings(Cycles);
    await add(target, 'Winter');
    await clickLabel('Icon and colour of Winter');
    await clickLabel('moon');
    expect(calls.filter((c) => c === 'cycles.update')).toHaveLength(1);
    await clickLabel('Delete Winter');
    await press(document.body, 'Delete cycle');
    expect(calls.filter((c) => c === 'cycles.delete')).toHaveLength(1);
  });
});
