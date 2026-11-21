import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import { currentViewId } from '../../lib/stores.svelte';
import TableView from './TableView.svelte';

// Fixture view 1 shows Title, Effort, a column whose def is gone, and Area.
const seed = {
  ...fixtures,
  views: [
    {
      id: 1,
      name: 'All',
      position: 0,
      def: {
        filter: null,
        sorts: [],
        group_by: null,
        layout: 'Table' as const,
        columns: [{ Fixed: 'Title' as const }, { Property: 'effort' }, { Property: 'gone' }, { Property: 'area' }],
        include_archived: false,
      },
    },
  ],
};

let target: HTMLElement;
let app: ReturnType<typeof mount>;
let mock: ReturnType<typeof createMockApi>;

const flush = async () => {
  await tick();
  flushSync();
  await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
const cell = (key: string, col: string) =>
  target.querySelector<HTMLElement>(`tr[data-key="${key}"] td[data-col="${col}"]`)!;
const activeCell = () => {
  const td = target.querySelector<HTMLElement>('td[data-active]')!;
  return `${td.closest('tr')!.dataset.key}|${td.dataset.col}`;
};
const press = (el: Element, key: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
const type = (input: HTMLInputElement, text: string) => {
  input.value = text;
  input.dispatchEvent(new Event('input', { bubbles: true }));
};
const grid = () => target.querySelector('table')!;
const alertText = () => target.querySelector('[role="alert"]')?.textContent?.trim() ?? '';

async function open() {
  target = document.body.appendChild(document.createElement('div'));
  app = mount(TableView, { target });
  await flush();
  await vi.waitFor(() => expect(cell('ZF-1', 'prop:effort')?.textContent?.trim()).toBe('1'));
}

async function startEdit(key: string, col: string) {
  cell(key, col).click();
  press(grid(), 'Enter');
  await flush();
  return cell(key, col).querySelector<HTMLInputElement>('input')!;
}

beforeEach(() => {
  mock = createMockApi(seed);
  setApi(mock);
  currentViewId.value = 1;
});

afterEach(async () => {
  await unmount(app);
  target.remove();
  vi.restoreAllMocks();
});

describe('TableView', () => {
  it('edit_number_property_saves', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    await open();
    const input = await startEdit('ZF-1', 'prop:effort');
    type(input, '5');
    press(input, 'Enter');
    await vi.waitFor(() => expect(save).toHaveBeenCalledWith(1, 'effort', 5));
    await vi.waitFor(() => expect(cell('ZF-1', 'prop:effort').textContent?.trim()).toBe('5'));
  });

  it('non_numeric_number_is_rejected_before_api', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    await open();
    const input = await startEdit('ZF-1', 'prop:effort');
    type(input, 'abc');
    press(input, 'Enter');
    await flush();
    expect(save).not.toHaveBeenCalled();
    expect(alertText()).toContain('"abc" is not a number');
  });

  it('invalid_value_reverts_with_message', async () => {
    const save = vi.spyOn(mock.properties, 'setValue').mockRejectedValueOnce({ kind: 'Invalid', message: 'effort is out of range' });
    await open();
    const input = await startEdit('ZF-1', 'prop:effort');
    type(input, '7');
    press(input, 'Enter');
    await vi.waitFor(() => expect(alertText()).toBe('effort is out of range'));
    expect(save).toHaveBeenCalledWith(1, 'effort', 7);
    expect(cell('ZF-1', 'prop:effort').querySelector('input')).toBeNull();
    expect(cell('ZF-1', 'prop:effort').textContent?.trim()).toBe('1');
  });

  it('esc_while_editing_restores_without_api_call', async () => {
    const save = vi.spyOn(mock.properties, 'setValue');
    const update = vi.spyOn(mock.issues, 'update');
    await open();
    const input = await startEdit('ZF-1', 'prop:effort');
    type(input, '9');
    press(input, 'Escape');
    await flush();
    expect(cell('ZF-1', 'prop:effort').querySelector('input')).toBeNull();
    expect(cell('ZF-1', 'prop:effort').textContent?.trim()).toBe('1');
    expect(save).not.toHaveBeenCalled();
    expect(update).not.toHaveBeenCalled();
  });

  it('arrow_navigation', async () => {
    await open();
    expect(activeCell()).toBe('ZF-1|fixed:Title');
    press(grid(), 'ArrowRight');
    await flush();
    expect(activeCell()).toBe('ZF-1|prop:effort');
    press(grid(), 'ArrowDown');
    await flush();
    expect(activeCell()).toBe('ZF-2|prop:effort');
    press(grid(), 'ArrowUp');
    press(grid(), 'ArrowUp');
    await flush();
    expect(activeCell()).toBe('ZF-1|prop:effort');
    press(grid(), 'ArrowLeft');
    press(grid(), 'ArrowLeft');
    press(grid(), 'ArrowLeft');
    await flush();
    expect(activeCell()).toBe('ZF-1|fixed:Title');
  });

  it('add_column_updates_view_def', async () => {
    const save = vi.spyOn(mock.views, 'save');
    await open();
    const add = target.querySelector<HTMLSelectElement>('[data-testid="add-column"]')!;
    add.value = 'prop:review_on';
    add.dispatchEvent(new Event('change', { bubbles: true }));
    await vi.waitFor(() =>
      expect(save).toHaveBeenCalledWith(
        1,
        'All',
        expect.objectContaining({
          columns: [
            { Fixed: 'Title' },
            { Property: 'effort' },
            { Property: 'gone' },
            { Property: 'area' },
            { Property: 'review_on' },
          ],
        }),
      ),
    );
  });

  it('column_with_deleted_def_is_skipped', async () => {
    await open();
    const heads = [...target.querySelectorAll<HTMLElement>('thead th[data-col]')].map((th) => th.dataset.col);
    expect(heads).toEqual(['fixed:Title', 'prop:effort', 'prop:area']);
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });
});
