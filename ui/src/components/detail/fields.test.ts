import { afterEach, expect, it } from 'vitest';
import { mount, unmount } from 'svelte';
import { createMockApi } from '../../lib/api.mock';
import { setApi } from '../../lib/api';
import { loadFields } from '../../lib/fields.svelte';
import IssueDetail from '../IssueDetail.svelte';

let app: object | undefined;
const settle = (ms = 30) => new Promise((r) => setTimeout(r, ms));
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

it('the details panel hides a field that is off, and lists the rest in the chosen order', async () => {
  const api = createMockApi();
  setApi(api);
  await api.settings.set('issue_fields', [{ field: 'due', on: false }, { field: 'labels', on: true }, { field: 'priority', on: true }]);
  await loadFields();
  const issue = await api.issues.create({ title: 'Detail subject' });
  app = mount(IssueDetail, { target: document.body, props: { params: { key: issue.key } } });
  await settle();
  const rows = [...document.querySelectorAll('.plate .row .k')].map((k) => k.textContent);
  expect(rows).not.toContain('Due');
  expect(rows.indexOf('Labels')).toBeLessThan(rows.indexOf('Priority'));
  expect(rows.indexOf('Priority')).toBeLessThan(rows.indexOf('Project'));
  expect(rows[0]).toBe('Status');
  await api.settings.set('issue_fields', []);
  await loadFields();
});
