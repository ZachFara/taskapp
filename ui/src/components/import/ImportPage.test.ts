import { describe, expect, it, vi } from 'vitest';
import { mountSettings, press } from '../settings/testing';
import ImportPage from './ImportPage.svelte';

const csv = [
  'Summary,Body,State,Tags,Points,Team',
  'Fix login,Users cannot sign in,Todo,auth,3,core',
  ',no title here,Todo,,,ops',
  'Ship report,,Done,,,ops',
].join('\n');

const plan = {
  source: 'csv',
  statuses: [],
  labels: ['auth'],
  projects: [],
  cycles: [],
  property_defs: [{ name: 'Team' }],
  issues: [{ external_id: 'csv:1', title: 'Fix login' }, { external_id: 'csv:3', title: 'Ship report' }],
  relations: [],
  warnings: ['Row 2 has a status not in the workspace; it will be created as Todo.'],
  skipped: [{ row: 2, reason: 'title is empty' }],
  unmapped: ['Team'],
};

/** Gives the CSV file input a file the way the browser does, then lets the component read it. */
async function upload(target: Element, text: string) {
  const input = target.querySelector<HTMLInputElement>('input[type="file"]')!;
  const file = new File([text], 'issues.csv', { type: 'text/csv' });
  Object.defineProperty(input, 'files', { value: [file], configurable: true });
  input.dispatchEvent(new Event('change', { bubbles: true }));
  await new Promise((r) => setTimeout(r, 0));
}

/** The text of the mapping row for one CSV column, so a test can read what it maps to. */
const rowFor = (target: Element, column: string) =>
  [...target.querySelectorAll('tr')].find((tr) => tr.textContent?.includes(column))?.textContent ?? '';

describe('ImportPage CSV', () => {
  it('guesses the column mapping from the header names', async () => {
    const { target } = await mountSettings(ImportPage);
    await upload(target, csv);
    expect(rowFor(target, 'Summary')).toContain('Title');
    expect(rowFor(target, 'Body')).toContain('Description');
    expect(rowFor(target, 'State')).toContain('Status');
    expect(rowFor(target, 'Tags')).toContain('Labels');
    expect(rowFor(target, 'Points')).toContain('Estimate');
    expect(rowFor(target, 'Team')).toContain('Ignore');
  });

  it('choosing_custom_property_writes_the_mapping', async () => {
    const csvPlan = vi.fn(async () => plan);
    const { target } = await mountSettings(ImportPage, { import: { csvPlan } });
    await upload(target, csv);
    const trigger = target.querySelector<HTMLElement>('[aria-label="Map Team"]')!;
    trigger.focus();
    trigger.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    await new Promise((r) => setTimeout(r, 0));
    const option = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((o) => o.textContent?.includes('Custom property'))!;
    for (const type of ['pointerdown', 'pointerup', 'click']) option.dispatchEvent(new MouseEvent(type, { bubbles: true }));
    await new Promise((r) => setTimeout(r, 0));
    expect(rowFor(target, 'Team')).toContain('Custom property');
    expect(target.querySelector('input[aria-label="Property name for Team"]')).not.toBeNull();
  });

  it('dry run renders the plan and the skipped rows, and sends the mapping', async () => {
    const csvPlan = vi.fn(async () => plan);
    const { target, api } = await mountSettings(ImportPage, { import: { csvPlan } });
    await upload(target, csv);
    await press(target, 'Dry run');
    expect(csvPlan).toHaveBeenCalledWith(csv, expect.arrayContaining([['Summary', 'title'], ['Body', 'description']]));
    expect(target.textContent).toContain('2 issues');
    expect(target.textContent).toContain('auth');
    expect(target.textContent).toContain('Row 2 has a status not in the workspace');
    expect(target.textContent).toContain('title is empty');
    expect(target.textContent).toContain('2');
    expect(api).toBeTruthy();
  });

  it('apply calls import_apply once with the plan that was shown', async () => {
    const apply = vi.fn(async () => ({ created: 2, warnings: [] }));
    const { target } = await mountSettings(ImportPage, {
      import: { csvPlan: async () => plan, apply },
    });
    await upload(target, csv);
    await press(target, 'Dry run');
    expect(apply).not.toHaveBeenCalled();
    await press(target, 'Import 2 issues');
    expect(apply).toHaveBeenCalledTimes(1);
    expect(apply).toHaveBeenCalledWith(plan);
    expect(target.textContent).toContain('Undo import');
    expect(target.textContent).toContain('View issues');
  });

  it('a failed apply shows the error, writes nothing and offers no undo', async () => {
    const { target } = await mountSettings(ImportPage, {
      import: {
        csvPlan: async () => plan,
        apply: async () => {
          throw { message: 'issue 3 is invalid' };
        },
      },
    });
    await upload(target, csv);
    await press(target, 'Dry run');
    await press(target, 'Import 2 issues');
    expect(target.textContent).toContain('issue 3 is invalid');
    expect(target.textContent).not.toContain('Undo import');
  });

  it('undo import calls undo', async () => {
    const undo = vi.fn(async () => ({ undo: null, redo: 'Import 2 issues' }));
    const { target } = await mountSettings(ImportPage, {
      import: { csvPlan: async () => plan, apply: async () => ({ created: 2, warnings: [] }) },
      history: { undo },
    });
    await upload(target, csv);
    await press(target, 'Dry run');
    await press(target, 'Import 2 issues');
    await press(target, 'Undo import');
    expect(undo).toHaveBeenCalledTimes(1);
  });
});

describe('ImportPage drop and mapping', () => {
  it('a CSV dropped on the drop area is read like a chosen file', async () => {
    const { target } = await mountSettings(ImportPage);
    const drop = target.querySelector<HTMLElement>('.drop')!;
    const dropped = new Event('drop', { bubbles: true, cancelable: true });
    Object.defineProperty(dropped, 'dataTransfer', { value: { files: [new File([csv], 'issues.csv', { type: 'text/csv' })] } });
    drop.dispatchEvent(dropped);
    await new Promise((r) => setTimeout(r, 0));
    expect(target.textContent).toContain('issues.csv');
    expect(rowFor(target, 'Summary')).toContain('Title');
  });

  it('a quoted header with a comma is one column', async () => {
    const { target } = await mountSettings(ImportPage);
    await upload(target, '"Title, long",Body\nFix,Users');
    expect(rowFor(target, 'Title, long')).toContain('Title');
  });
});

describe('ImportPage Linear', () => {
  it('explains the environment token and never asks for it in a field', async () => {
    const { target } = await mountSettings(ImportPage);
    await press(target, 'Linear');
    expect(target.textContent).toContain('LINEAR_API_KEY');
    expect(target.textContent).toContain('taskapp import linear --apply');
    expect(target.querySelector('input[type="password"]')).toBeNull();
    expect([...target.querySelectorAll('input')].some((i) => /token|key/i.test(i.getAttribute('aria-label') ?? ''))).toBe(false);
  });
});
