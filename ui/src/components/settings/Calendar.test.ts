import { describe, expect, it } from 'vitest';
import Calendar from './Calendar.svelte';
import { mountSettings, press, setField } from './testing';

describe('Calendar', () => {
  it('calendar_picker_saves_setting', async () => {
    const { target, calls, api } = await mountSettings(Calendar);
    await press(target, 'Connect Google Calendar');
    expect(calls).toContain('gcal.connect');
    expect(calls).toContain('gcal.listCalendars');
    expect(target.textContent).toContain('dedicated calendar');
    await setField(target, 'Calendar', 'primary');
    expect(await api.settings.get('gcal_calendar_id')).toBe('primary');
  });
});
