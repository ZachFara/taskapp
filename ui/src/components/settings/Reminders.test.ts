import { describe, expect, it } from 'vitest';
import Reminders from './Reminders.svelte';
import { mountSettings, press, setField } from './testing';

describe('Reminders', () => {
  it('reminders_list_picker_saves_setting', async () => {
    const { target, calls, api } = await mountSettings(Reminders);
    await press(target, 'Authorize');
    expect(calls).toContain('reminders.listLists');
    await setField(target, 'Inbox list', '1');
    expect(await api.settings.get('reminders_list_id')).toBe('1');
  });
});
