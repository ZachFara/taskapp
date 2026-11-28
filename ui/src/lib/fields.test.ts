import { expect, it } from 'vitest';
import { api, setApi } from './api';
import { createMockApi } from './api.mock';
import { listCommands, type Ctx } from './commands';
import { fieldOn, fixedOn, loadFields } from './fields.svelte';
import { normalize } from './issueFields';

const settle = () => new Promise((r) => setTimeout(r, 20));
const ctx = { route: '/view/1', layout: 'list', selection: [1], focusedIssueId: 1, openIssueId: null, navigate: () => {} } as unknown as Ctx;
const offered = (id: string) => listCommands(ctx).some((c) => c.id === id);

it('normalize keeps the stored order, drops unknown and repeated names, and appends the rest on', () => {
  const list = normalize([{ field: 'due', on: false }, { field: 'nope', on: true }, { field: 'due', on: true }, { field: 'labels', on: true }]);
  expect(list.slice(0, 2)).toEqual([{ field: 'due', on: false }, { field: 'labels', on: true }]);
  expect(list).toHaveLength(11);
  expect(list.slice(2).every((f) => f.on)).toBe(true);
});

it('fieldOn follows the setting; points also needs estimation; hidden values are kept', async () => {
  setApi(createMockApi());
  await loadFields();
  expect(fieldOn('due')).toBe(true);
  await api.settings.set('issue_fields', [{ field: 'due', on: false }, { field: 'points', on: true }]);
  await settle();
  expect([fieldOn('due'), fieldOn('points'), fixedOn('DueDate'), fixedOn('Title')]).toEqual([false, true, false, true]);
  await api.settings.set('estimation', 'time');
  await settle();
  expect(fieldOn('points')).toBe(false);
  await api.settings.set('estimation', 'both');
  await api.settings.set('issue_fields', []);
  await settle();
  expect(fieldOn('due')).toBe(true);
});

it('the settings write refuses unknown fields', async () => {
  setApi(createMockApi());
  await expect(api.settings.set('issue_fields', [{ field: 'status', on: false }])).rejects.toBeTruthy();
});

it('Set due date and its cheat sheet row disappear when Due is off', async () => {
  setApi(createMockApi());
  await api.settings.set('issue_fields', []);
  await settle();
  expect(offered('issue.due')).toBe(true);
  await api.settings.set('issue_fields', [{ field: 'due', on: false }]);
  await settle();
  expect(offered('issue.due')).toBe(false);
  expect(offered('issue.priority')).toBe(true);
  await api.settings.set('issue_fields', []);
});

it('quick add leaves the token of an off field in the title', async () => {
  const mock = createMockApi();
  setApi(mock);
  expect((await mock.issues.parseQuickadd('fix p1 ~1h')).priority).toBe(1);
  await mock.settings.set('issue_fields', [{ field: 'priority', on: false }, { field: 'estimate', on: false }]);
  const p = await mock.issues.parseQuickadd('fix p1 ~1h');
  expect([p.priority, p.estimate_minutes, p.title]).toEqual([null, null, 'fix p1 ~1h']);
});
