import { expect, it } from 'vitest';
import { stepLabel } from './api.mock.stepLabel';

it('quick add is labelled like a plain create, not as a command name', () => {
  expect(stepLabel('issues_create_from_quickadd', [], { key: 'ABC-25' })).toBe('Created ABC-25');
});

it('folder moves say what moved, as the backend does', () => {
  expect(stepLabel('folders_place', [1, null, 0], null)).toBe('Moved folder');
  expect(stepLabel('folders_place_item', ['view', 3, null, null], null)).toBe('Moved view');
});
