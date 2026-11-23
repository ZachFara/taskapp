import { describe, expect, it } from 'vitest';
import RelationKinds from './RelationKinds.svelte';
import { clickLabel, field, mountSettings, press, setField } from './testing';

async function add(target: Element, name: string, forward: string, reverse: string) {
  await setField(target, 'Kind name', name);
  await setField(target, 'Forward label', forward);
  await setField(target, 'Reverse label', reverse);
  await press(target, 'Add kind');
}

describe('RelationKinds', () => {
  it('lists the built-in kinds without edit controls', async () => {
    const { target } = await mountSettings(RelationKinds);
    for (const name of ['Blocks', 'Related', 'Duplicate', 'Follows', 'Caused by', 'Part of']) expect(target.textContent).toContain(name);
    expect(target.querySelector('[aria-label="Name of Blocks"]')).toBeNull();
  });

  it('creates a kind, renames it and deletes it after asking', async () => {
    const { target, calls } = await mountSettings(RelationKinds);
    await add(target, 'Mentors', 'mentors', 'mentored by');
    expect(calls.filter((c) => c === 'relation.kindsCreate')).toHaveLength(1);
    expect(field(target, 'Kind name').value).toBe('');
    await setField(target, 'Name of Mentors', 'Coaches');
    expect(calls.filter((c) => c === 'relation.kindsUpdate')).toHaveLength(1);
    await clickLabel('Delete Coaches');
    expect(calls).not.toContain('relation.kindsDelete');
    await press(document.body, 'Delete kind');
    expect(calls.filter((c) => c === 'relation.kindsDelete')).toHaveLength(1);
    expect(target.querySelector('[aria-label="Name of Coaches"]')).toBeNull();
  });

  it('a kind in use cannot be deleted and says why', async () => {
    const { target, api } = await mountSettings(RelationKinds);
    await add(target, 'Mentors', 'mentors', 'mentored by');
    await api.relations.add({ from: 1, to: 2, kind: 'mentors' });
    await clickLabel('Delete Mentors');
    await press(document.body, 'Delete kind');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/1 relations use Mentors/);
  });

  it('a duplicate name shows its message', async () => {
    const { target } = await mountSettings(RelationKinds);
    await add(target, 'Blocks', 'x', 'y');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/already exists/);
  });
});
