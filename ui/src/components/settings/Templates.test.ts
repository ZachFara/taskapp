import { describe, expect, it } from 'vitest';
import type { IssueTemplate } from '../../lib/bindings';
import Templates from './Templates.svelte';
import { field, mountSettings, press, setField } from './testing';

const tpl = (over: Partial<IssueTemplate> = {}): IssueTemplate => ({
  id: 1,
  name: 'Bug report',
  slug: 'bug',
  icon: null,
  color: null,
  body: { title_prefix: '[Bug] ', sub_issues: [{ title: 'Reproduce' }, { title: 'Fix' }] },
  ...over,
});

/** Opens a template from the list by its name (rows also carry the slug, so exact text does not match). */
const openRow = async (root: Element, name: string) => {
  const row = [...root.querySelectorAll('button')].find((b) => b.textContent?.includes(name));
  if (!row) throw new Error(`no template row "${name}"`);
  row.click();
  await new Promise((r) => setTimeout(r, 0));
};

const clickAria = (root: Element, label: string) => {
  const el = root.querySelector<HTMLElement>(`[aria-label="${label}"]`);
  if (!el) throw new Error(`no control labelled "${label}"`);
  el.click();
  return new Promise((r) => setTimeout(r, 0));
};

describe('Templates', () => {
  it('lists each template with its name and /slug', async () => {
    const { target } = await mountSettings(Templates, { templates: { list: async () => [tpl()] } });
    expect(target.textContent).toContain('Bug report');
    expect(target.textContent).toContain('/bug');
  });

  it('creates a template with one create call and the typed body', async () => {
    const sent: unknown[] = [];
    const { target, calls } = await mountSettings(Templates, {
      templates: { create: async (input: unknown) => { sent.push(input); return tpl({ id: 9, name: 'Meeting', slug: 'meeting' }); } },
    });
    await press(target, 'New template');
    await setField(target, 'Template name', 'Meeting');
    await setField(target, 'Template slug', 'meeting');
    await setField(target, 'Title prefix', 'Mtg: ');
    await press(target, 'Save template');
    expect(calls.filter((c) => c === 'templates.create')).toHaveLength(1);
    expect(sent[0]).toMatchObject({ name: 'Meeting', slug: 'meeting', body: { title_prefix: 'Mtg: ' } });
  });

  it('rejects a slug with capitals or spaces and does not call the api', async () => {
    const { target, calls } = await mountSettings(Templates);
    await press(target, 'New template');
    await setField(target, 'Template name', 'Meeting');
    await setField(target, 'Template slug', 'Bad Slug');
    await press(target, 'Save template');
    expect(target.querySelector('[role="alert"]')?.textContent).toMatch(/lowercase|slug/i);
    expect(calls).not.toContain('templates.create');
    expect(calls).not.toContain('templates.update');
  });

  it('updates the selected template with one update call', async () => {
    const { target, calls } = await mountSettings(Templates, { templates: { list: async () => [tpl()] } });
    await openRow(target, 'Bug report');
    await setField(target, 'Template name', 'Bug');
    await press(target, 'Save template');
    expect(calls.filter((c) => c === 'templates.update')).toHaveLength(1);
    expect(calls).not.toContain('templates.create');
  });

  it('deletes after confirmation with one delete call', async () => {
    const { target, calls } = await mountSettings(Templates, { templates: { list: async () => [tpl()] } });
    await openRow(target, 'Bug report');
    await press(target, 'Delete template');
    expect(calls).not.toContain('templates.delete');
    await press(target, 'Delete');
    expect(calls.filter((c) => c === 'templates.delete')).toHaveLength(1);
  });

  it('reorders sub-issues and saves the new order', async () => {
    const sent: { body: { sub_issues: { title: string }[] } }[] = [];
    const { target, calls } = await mountSettings(Templates, {
      templates: {
        list: async () => [tpl()],
        update: async (_id: unknown, input: { body: { sub_issues: { title: string }[] } }) => { sent.push(input); return tpl(); },
      },
    });
    await openRow(target, 'Bug report');
    await clickAria(target, 'Move sub-issue 2 up');
    await press(target, 'Save template');
    expect(calls).toContain('templates.update');
    expect(sent[0].body.sub_issues.map((s) => s.title)).toEqual(['Fix', 'Reproduce']);
  });

  it('try it applies the template once', async () => {
    const { target, calls } = await mountSettings(Templates, { templates: { list: async () => [tpl()] } });
    await openRow(target, 'Bug report');
    await press(target, 'Try it');
    expect(calls.filter((c) => c === 'templates.apply')).toHaveLength(1);
    expect(field(target, 'Template name').value).toBe('Bug report');
  });
});
