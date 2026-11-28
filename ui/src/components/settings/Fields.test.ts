import { describe, expect, it } from 'vitest';
import Fields from './Fields.svelte';
import { field, mountSettings, toggle } from './testing';

const names = (t: Element) => [...t.querySelectorAll('.es-name')].map((n) => n.textContent);
const flush = () => new Promise((r) => setTimeout(r, 30));

describe('Fields settings', () => {
  it('lists every field on, then switches one off with one write that keeps the order', async () => {
    const { target, calls, api } = await mountSettings(Fields);
    expect(names(target)).toHaveLength(11);
    await toggle(target, 'Show Due');
    expect(calls.filter((c) => c === 'settings.set')).toHaveLength(1);
    const stored = (await api.settings.get('issue_fields')) as { field: string; on: boolean }[];
    expect(stored.find((f) => f.field === 'due')?.on).toBe(false);
    expect(stored.filter((f) => f.on)).toHaveLength(10);
    expect(field(target, 'Show Due').getAttribute('data-state')).toBe('unchecked');
  });

  it('moves a field down with its button and saves the new order', async () => {
    const { target, api } = await mountSettings(Fields);
    (field(target, 'Move Priority down') as HTMLElement).click();
    await flush();
    expect(names(target).slice(0, 2)).toEqual(['Project', 'Priority']);
    const stored = (await api.settings.get('issue_fields')) as { field: string }[];
    expect(stored.slice(0, 2).map((f) => f.field)).toEqual(['project', 'priority']);
  });
});
