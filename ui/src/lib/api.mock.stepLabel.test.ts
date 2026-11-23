import { expect, it } from 'vitest';
import { stepLabel } from './api.mock.stepLabel';

it('quick add is labelled like a plain create, not as a command name', () => {
  expect(stepLabel('issues_create_from_quickadd', [], { key: 'ABC-25' })).toBe('Created ABC-25');
});
