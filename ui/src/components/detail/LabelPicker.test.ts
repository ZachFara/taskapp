import { flushSync, mount } from 'svelte';
import { describe, expect, it, vi } from 'vitest';
import type { Label } from '../../lib/bindings';
import LabelPicker from './LabelPicker.svelte';

const label = (id: number, name: string) => ({ id, name, color: 'gray' }) as unknown as Label;
const type = (target: Element, text: string, key?: string) => {
  const input = target.querySelector<HTMLInputElement>('input[aria-label="Find or create a label"]')!;
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
  if (key) input.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true }));
  flushSync();
};

describe('LabelPicker', () => {
  it('with no labels, typing a name and Enter creates it', async () => {
    const target = document.body.appendChild(document.createElement('div'));
    const oncreate = vi.fn();
    mount(LabelPicker, { target, props: { labels: [], selected: [], ontoggle: vi.fn(), oncreate } });
    expect(target.textContent).toContain('No labels yet');
    type(target, ' bug ', 'Enter');
    await vi.waitFor(() => expect(oncreate).toHaveBeenCalledWith('bug'));
  });

  it('typing filters the list, Enter on an exact name ticks it and offers no create', () => {
    const target = document.body.appendChild(document.createElement('div'));
    const [ontoggle, oncreate] = [vi.fn(), vi.fn()];
    mount(LabelPicker, { target, props: { labels: [label(1, 'Bug'), label(2, 'Docs')], selected: [], ontoggle, oncreate } });
    type(target, 'bu');
    expect(target.textContent).toContain('Bug');
    expect(target.textContent).not.toContain('Docs');
    expect(target.textContent).toContain('Create label "bu"');
    type(target, 'bug', 'Enter');
    expect(ontoggle).toHaveBeenCalledWith(1, true);
    expect(oncreate).not.toHaveBeenCalled();
  });
});
