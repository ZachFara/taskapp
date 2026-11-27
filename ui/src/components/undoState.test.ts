import { afterEach, expect, it } from 'vitest';
import { dismiss, say, undoToast } from './undoState.svelte';

afterEach(() => dismiss());

it('the same message said twice shows once', () => {
  say({ label: 'Created ABC-25', tone: 'success' });
  say({ label: 'Created ABC-25', tone: 'success' });
  say({ label: 'Created ABC-26', tone: 'success' });
  expect(undoToast.items.map((m) => m.label)).toEqual(['Created ABC-25', 'Created ABC-26']);
});
