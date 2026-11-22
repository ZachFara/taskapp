import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../../lib/api';
import { createMockApi, fixtures } from '../../lib/api.mock';
import type { Issue, SavedView } from '../../lib/bindings';
import CalendarView from './CalendarView.svelte';

// The calendar library measures the DOM, which jsdom lacks: the view gets a stub that records its options.
vi.mock('@event-calendar/core', async () => ({
  TimeGrid: { name: 'TimeGrid' },
  DayGrid: { name: 'DayGrid' },
  Interaction: { name: 'Interaction' },
  Calendar: (await import('../../test/CalendarStub.svelte')).default,
}));
// The stub's state for the calendar most recently mounted.
const stub = () => (globalThis as any).__calStub;

const view: SavedView = {
  id: 1,
  name: 'Calendar',
  position: 0,
  def: { filter: null, sorts: [], group_by: null, layout: 'Calendar', columns: [], include_archived: false },
};

const iso = (d: Date) => d.toISOString();
const HOUR = 60 * 60_000;

// The last events array the view handed the calendar.
const lastEvents = () => stub().setOption.mock.calls.filter(([k]: [string]) => k === 'events').at(-1)?.[1] ?? [];

// Fixtures plus one issue with a timed due in New York on the US DST start day (2026-03-08).
const withTimedDue = (): typeof fixtures => {
  const base = fixtures.issues[0];
  const timed: Issue = { ...base, id: 900, key: 'ZF-900', due: { DateTime: { utc: '2026-03-08T13:30:00Z', tz: 'America/New_York' } } };
  return { ...fixtures, issues: [...fixtures.issues, timed] };
};

let mock: ReturnType<typeof createMockApi>;

beforeEach(async () => {
  mock = createMockApi(withTimedDue());
  setApi(mock);
  // block-1: the block the move, delete and resize tests act on.
  await mock.blocks.create(1, '2026-03-10T09:00:00.000Z', '2026-03-10T10:00:00.000Z');
});

beforeEach(() => localStorage.clear());

afterEach(() => {
  cleanup();
  setApi(createMockApi());
  vi.restoreAllMocks();
});

// Load the blocks shown in the current range, as the library does on first render.
const openRange = () =>
  stub().options.datesSet({ start: new Date(2026, 2, 1), end: new Date(2026, 3, 1) });

describe('CalendarView', () => {
  it('drop_creates_60_minute_block', async () => {
    const create = vi.spyOn(mock.blocks, 'create');
    const { container } = render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    const start = new Date(2026, 2, 10, 9, 0);
    stub().dateFromPoint = vi.fn(() => ({ date: start, allDay: false }));
    const host = container.querySelector('[data-calendar]') as HTMLElement;
    await fireEvent.drop(host, { dataTransfer: { getData: () => '1' }, clientX: 10, clientY: 10 });
    await waitFor(() => expect(create).toHaveBeenCalledTimes(1));
    expect(create).toHaveBeenCalledWith(1, iso(start), iso(new Date(start.getTime() + HOUR)));
  });

  it('move_block_calls_api', async () => {
    const move = vi.spyOn(mock.blocks, 'move');
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    openRange();
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'block-1')).toBe(true));
    const start = new Date(2026, 2, 11, 10, 0);
    const end = new Date(start.getTime() + HOUR);
    const revert = vi.fn();
    await stub().options.eventDrop({ event: { id: 'block-1', start, end }, revert });
    expect(move).toHaveBeenCalledWith(1, iso(start), iso(end));
    expect(revert).not.toHaveBeenCalled();
  });

  it('delete_removes_block', async () => {
    const del = vi.spyOn(mock.blocks, 'delete');
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    openRange();
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'block-1')).toBe(true));
    stub().options.eventClick({ event: { id: 'block-1' } });
    await fireEvent.keyDown(window, { key: 'Delete' });
    await waitFor(() => expect(del).toHaveBeenCalledWith(1));
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'block-1')).toBe(false));
  });

  it('week_shows_timed_due_markers', async () => {
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    expect(stub().options.view).toBe('timeGridWeek');
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'due-900')).toBe(true));
    const marker = lastEvents().find((e: any) => e.id === 'due-900');
    expect(marker.allDay).toBeFalsy();
    expect(marker.editable).toBe(true);
    expect(marker.classNames).toContain('due');
  });

  it('month_view_shows_all_day_dues', async () => {
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    expect(stub().plugins).toContainEqual({ name: 'DayGrid' });
    const dateDue = fixtures.issues.find((i) => i.due && 'Date' in i.due)!;
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === `due-${dateDue.id}`)).toBe(true));
    const allDay = lastEvents().find((e: any) => e.id === `due-${dateDue.id}`);
    expect(allDay.allDay).toBe(true);
    expect(allDay.start).toBe(dateDue.due!.Date);
  });

  it('dst_due_shows_local_wall_clock', async () => {
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'due-900')).toBe(true));
    // 13:30 UTC is 09:30 in New York once DST has started that morning; the wall clock is what shows.
    expect(lastEvents().find((e: any) => e.id === 'due-900').start).toBe('2026-03-08T09:30:00');
  });

  it('timed_due_shows_on_its_day_in_its_own_zone', async () => {
    // 23:30 in Tokyo on 10 March is 14:30 UTC on 10 March, but 15:30 UTC on 10 March is 00:30 on 11 March in Tokyo.
    const late: Issue = { ...fixtures.issues[0], id: 901, key: 'ZF-901', due: { DateTime: { utc: '2026-03-10T15:30:00Z', tz: 'Asia/Tokyo' } } };
    mock = createMockApi({ ...fixtures, issues: [...fixtures.issues, late] });
    setApi(mock);
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'due-901')).toBe(true));
    expect(lastEvents().find((e: any) => e.id === 'due-901').start).toBe('2026-03-11T00:30:00');
  });

  it('drag_timed_due_changes_due_once', async () => {
    const update = vi.spyOn(mock.issues, 'update');
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'due-900')).toBe(true));
    // Dropped on 11 March at 09:30 local wall clock; New York is UTC-4 by then, so 13:30 UTC.
    const start = new Date(2026, 2, 11, 9, 30);
    const revert = vi.fn();
    await stub().options.eventDrop({ event: { id: 'due-900', start, end: new Date(start.getTime() + 15 * 60_000), allDay: false }, revert });
    expect(update).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenCalledWith(900, { due: { DateTime: { utc: '2026-03-11T13:30:00.000Z', tz: 'America/New_York' } } });
    expect(revert).not.toHaveBeenCalled();
  });

  it('drag_date_due_changes_day_only', async () => {
    const dateDue = fixtures.issues.find((i) => i.due && 'Date' in i.due)!;
    const update = vi.spyOn(mock.issues, 'update');
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === `due-${dateDue.id}`)).toBe(true));
    await stub().options.eventDrop({ event: { id: `due-${dateDue.id}`, start: new Date(2026, 2, 12), allDay: true }, revert: vi.fn() });
    expect(update).toHaveBeenCalledTimes(1);
    expect(update).toHaveBeenCalledWith(dateDue.id, { due: { Date: '2026-03-12' } });
  });

  it('drop_issue_on_month_day_sets_due', async () => {
    const create = vi.spyOn(mock.blocks, 'create');
    const update = vi.spyOn(mock.issues, 'update');
    const { container } = render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    stub().dateFromPoint = vi.fn(() => ({ date: new Date(2026, 2, 10), allDay: true }));
    const host = container.querySelector('[data-calendar]') as HTMLElement;
    await fireEvent.drop(host, { dataTransfer: { getData: () => '1' }, clientX: 10, clientY: 10 });
    await waitFor(() => expect(update).toHaveBeenCalledTimes(1));
    expect(update).toHaveBeenCalledWith(1, { due: { Date: '2026-03-10' } });
    expect(create).not.toHaveBeenCalled();
  });

  it('week_month_switch_persists', async () => {
    const first = render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    expect(stub().options.view).toBe('timeGridWeek');
    window.dispatchEvent(new CustomEvent('command:calendar.month'));
    await waitFor(() => expect(stub().setOption).toHaveBeenCalledWith('view', 'dayGridMonth'));
    first.unmount();
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    expect(stub().options.view).toBe('dayGridMonth');
  });

  it('next_command_moves_the_date', async () => {
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    window.dispatchEvent(new CustomEvent('command:calendar.next'));
    await waitFor(() => expect(stub().setOption).toHaveBeenCalledWith('date', expect.any(Date)));
    const moved = stub().setOption.mock.calls.filter(([k]: [string]) => k === 'date').at(-1)[1] as Date;
    expect(moved.getTime()).toBeGreaterThan(Date.now() + 6 * 24 * HOUR);
  });

  it('unscheduled_list_has_only_issues_without_due', async () => {
    render(CalendarView, { props: { view } });
    const list = await screen.findByRole('list', { name: 'Unscheduled' });
    const undated = withTimedDue().issues.filter((i) => !i.due);
    await waitFor(() => expect(list.querySelectorAll('li')).toHaveLength(undated.length));
    for (const i of undated) expect(list.textContent).toContain(i.title);
  });

  it('resize_to_zero_or_negative_length_is_refused', async () => {
    const move = vi.spyOn(mock.blocks, 'move');
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    openRange();
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'block-1')).toBe(true));
    const start = new Date(2026, 2, 11, 10, 0);
    for (const end of [start, new Date(start.getTime() - HOUR)]) {
      const revert = vi.fn();
      await stub().options.eventResize({ event: { id: 'block-1', start, end }, revert });
      expect(revert).toHaveBeenCalledTimes(1);
    }
    expect(move).not.toHaveBeenCalled();
  });

  it('failed_move_snaps_back_and_shows_error', async () => {
    vi.spyOn(mock.blocks, 'move').mockRejectedValue({ kind: 'Conflict', message: 'block overlaps another' });
    render(CalendarView, { props: { view } });
    await waitFor(() => expect(stub().options).toBeDefined());
    openRange();
    await waitFor(() => expect(lastEvents().some((e: any) => e.id === 'block-1')).toBe(true));
    const start = new Date(2026, 2, 11, 10, 0);
    const revert = vi.fn();
    await stub().options.eventDrop({ event: { id: 'block-1', start, end: new Date(start.getTime() + HOUR) }, revert });
    expect(revert).toHaveBeenCalledTimes(1);
    expect((await screen.findByRole('alert')).textContent).toContain('block overlaps another');
  });
});
