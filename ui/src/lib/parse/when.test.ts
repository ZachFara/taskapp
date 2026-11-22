import { describe, expect, it } from 'vitest';
import { formatDay, formatEstimate, parseDate, parseEstimate, todayIn } from './when';

// 2026-11-22 is a Sunday; noon UTC is the same calendar day in every zone from -11 to +11.
const now = new Date('2026-11-22T12:00:00Z');
const date = (text: string, tz = 'UTC', at = now) => parseDate(text, tz, at);

describe('parseDate', () => {
  const table: [string, string | null][] = [
    ['today', '2026-11-22'], ['Tomorrow', '2026-11-23'], ['  tomorrow  ', '2026-11-23'],
    ['mon', '2026-11-23'], ['fri', '2026-11-27'], ['friday', '2026-11-27'], ['sun', '2026-11-29'],
    ['next fri', '2026-11-27'], ['next mon', '2026-11-23'], ['next sun', '2026-11-29'],
    ['nov 28', '2026-11-28'], ['28 nov', '2026-11-28'], ['November 28', '2026-11-28'], ['nov 22', '2026-11-22'],
    ['nov 21', '2027-11-21'], ['jan 5', '2027-01-05'], ['feb 29', '2028-02-29'], ['dec 1', '2026-12-01'],
    ['2026-12-01', '2026-12-01'], ['2027-02-28', '2027-02-28'],
    ['+3d', '2026-11-25'], ['+2w', '2026-12-06'], ['+0d', '2026-11-22'], ['+45d', '2027-01-06'],
    ['none', null],
  ];
  it.each(table)('%s', (text, want) => expect(date(text)).toEqual({ value: want }));

  it.each(['', 'soonish', 'nov', 'nov 31', '2026-02-30', '2026-13-01', '+d', '-3d', '+3', 'next', 'next tomorrow', '28'])(
    '"%s" is not a date',
    (text) => expect(date(text)).toBeNull(),
  );

  it('"next weekday" is the day in next calendar week, plain weekday the soonest after today', () => {
    const wed = new Date('2026-11-25T12:00:00Z');
    expect(date('fri', 'UTC', wed)).toEqual({ value: '2026-11-27' });
    expect(date('next fri', 'UTC', wed)).toEqual({ value: '2026-12-04' });
    expect(date('wed', 'UTC', wed)).toEqual({ value: '2026-12-02' });
  });

  it('year rolls over', () => {
    const late = new Date('2026-12-30T12:00:00Z');
    expect(date('jan 2', 'UTC', late)).toEqual({ value: '2027-01-02' });
    expect(date('+3d', 'UTC', late)).toEqual({ value: '2027-01-02' });
    expect(date('dec 30', 'UTC', late)).toEqual({ value: '2026-12-30' });
  });

  it('uses the zone it is given for what today is', () => {
    const edge = new Date('2026-11-22T03:00:00Z');
    expect(todayIn('America/Los_Angeles', edge)).toBe('2026-11-21');
    expect(todayIn('Pacific/Auckland', edge)).toBe('2026-11-22');
    expect(date('today', 'America/Los_Angeles', edge)).toEqual({ value: '2026-11-21' });
    expect(date('tomorrow', 'Pacific/Auckland', edge)).toEqual({ value: '2026-11-23' });
    expect(date('+1d', 'America/Los_Angeles', edge)).toEqual({ value: '2026-11-22' });
  });
});

describe('parseEstimate', () => {
  const table: [string, number | null][] = [
    ['30m', 30], ['2h', 120], ['1h30m', 90], ['1.5h', 90], ['1d', 480], ['1d2h', 600], ['0.5d', 240], ['45M', 45], ['none', null],
  ];
  it.each(table)('%s', (text, want) => expect(parseEstimate(text)).toEqual({ value: want }));

  it.each(['', '0m', '0h', 'abc', '30', '1.5m', 'h', '2h2h', '30m1h', '-1h', '1h 30'])('"%s" is not an estimate', (text) =>
    expect(parseEstimate(text)).toBeNull(),
  );
});

describe('formatting', () => {
  it.each<[number, string]>([[30, '30m'], [60, '1h'], [90, '1h 30m'], [480, '8h'], [1500, '25h']])('%i minutes is %s', (m, want) =>
    expect(formatEstimate(m)).toBe(want),
  );
  it('writes a day with its weekday', () => expect(formatDay('2026-11-27')).toBe('Fri, Nov 27, 2026'));
});
