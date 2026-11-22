import { describe, expect, it } from 'vitest';
import Calendar from './Calendar.svelte';
import { mountSettings, pick, press, statusText, toggle } from './testing';

const calendars = async () => [['cal-1', 'Survey dates']] as [string, string][];

describe('Calendar', () => {
  it('calendar_picker_saves_setting', async () => {
    const { target, calls, api } = await mountSettings(Calendar, { gcal: { listCalendars: calendars } });
    await press(target, 'Connect Google Calendar');
    expect(calls).toContain('gcal.connect');
    expect(calls).toContain('gcal.listCalendars');
    expect(target.textContent).toContain('dedicated calendar');
    await pick(target, 'Calendar', 'Survey dates');
    expect(await api.settings.get('gcal_calendar_id')).toBe('cal-1');
  });

  it('the notifications switch saves gcal_enabled', async () => {
    const { target, api } = await mountSettings(Calendar);
    await toggle(target, 'Enable Google Calendar');
    expect(await api.settings.get('gcal_enabled')).toBe(true);
    expect(statusText(target)).toMatch(/not connected/i);
  });
});
