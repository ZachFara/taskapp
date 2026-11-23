import { describe, expect, it } from 'vitest';
import Reminders from './Reminders.svelte';
import { mountSettings, pick, press, statusText } from './testing';

const lists = async () => [['r-1', 'Inbox'], ['r-2', 'Errands']] as [string, string][];

describe('Reminders', () => {
  it('reminders_list_picker_saves_setting', async () => {
    const { target, calls, api } = await mountSettings(Reminders, { reminders: { listLists: lists } });
    await press(target, 'Authorize');
    expect(calls).toContain('reminders.listLists');
    await pick(target, 'Inbox list', 'Errands');
    expect(await api.settings.get('reminders_list_id')).toBe('r-2');
  });

  it('says it is not set up until access is granted', async () => {
    const { target } = await mountSettings(Reminders, { reminders: { listLists: lists } });
    expect(statusText(target)).toMatch(/not set up/i);
    await press(target, 'Authorize');
    expect(statusText(target)).toMatch(/connected/i);
  });

  it('shows the error when access is refused', async () => {
    const denied = async () => {
      throw { kind: 'Denied', message: 'Reminders access was refused' };
    };
    const { target } = await mountSettings(Reminders, { reminders: { listLists: denied } });
    await press(target, 'Authorize');
    expect(target.querySelector('[role="alert"]')?.textContent).toContain('refused');
  });
});
