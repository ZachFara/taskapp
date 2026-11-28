import { mount, unmount } from 'svelte';
import { afterEach, expect, it } from 'vitest';
import type { Issue, Label } from '../../lib/bindings';
import FieldValue from './FieldValue.svelte';
import { emptyCtx } from './types';

const labels = ['Docs', 'Design', 'Feature', 'Ops'].map((name, i) => ({ id: i + 1, name, color: 'blue', icon: null }) as unknown as Label);
const issue = { label_ids: [1, 2, 3, 4], properties: {} } as unknown as Issue;
let app: ReturnType<typeof mount> | null = null;
afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

it('labels show two and a count, or every label when the view wraps', () => {
  app = mount(FieldValue, { target: document.body, props: { field: { Fixed: 'Label' }, issue, ctx: { ...emptyCtx, labels } } });
  expect(document.body.textContent).toContain('+2');
  unmount(app);
  app = mount(FieldValue, { target: document.body, props: { field: { Fixed: 'Label' }, issue, ctx: { ...emptyCtx, labels }, all: true } });
  for (const l of labels) expect(document.body.textContent).toContain(l.name);
  expect(document.body.textContent).not.toContain('+2');
});

it('checklist shows done of total only when the description has items', () => {
  const withList = { ...issue, description: '- [x] a\n- [ ] b\n- [ ] c' } as unknown as Issue;
  app = mount(FieldValue, { target: document.body, props: { field: { Fixed: 'Checklist' }, issue: withList, ctx: emptyCtx } });
  expect(document.body.textContent).toContain('1/3');
  unmount(app);
  document.body.innerHTML = '';
  app = mount(FieldValue, { target: document.body, props: { field: { Fixed: 'Checklist' }, issue: { ...issue, description: 'no list' } as unknown as Issue, ctx: emptyCtx } });
  expect(document.body.textContent).toBe('');
});

it('a title shows a quiet back marker only while the issue is still deferred', () => {
  const show = (hidden_until: string | null) => {
    document.body.innerHTML = '';
    app = mount(FieldValue, { target: document.body, props: { field: { Fixed: 'Title' }, issue: { ...issue, title: 'Car sticker', hidden_until } as unknown as Issue, ctx: emptyCtx } });
    const text = document.body.textContent;
    unmount(app);
    return text;
  };
  expect(show('2999-11-30')).toContain('back Nov 30');
  expect(show('2000-01-01')).not.toContain('back');
  expect(show(null)).not.toContain('back');
});
