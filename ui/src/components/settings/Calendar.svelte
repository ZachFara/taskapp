<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let enabled = $state(false);
  let calendars = $state<[string, string][]>([]);
  let current = $state('');
  let error = $state('');

  onMount(async () => {
    enabled = ((await api.settings.get('gcal_enabled')) as boolean | null) === true;
    current = ((await api.settings.get('gcal_calendar_id')) as string | null) ?? '';
  });

  async function connect() {
    error = '';
    try {
      await api.gcal.connect();
      calendars = await api.gcal.listCalendars();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function pick(id: string) {
    current = id;
    await api.settings.set('gcal_calendar_id', id);
  }

  async function toggle(on: boolean) {
    enabled = on;
    await api.settings.set('gcal_enabled', on);
  }
</script>

<h2>Google Calendar</h2>
<p>
  Use a dedicated calendar for these events, and view it in the Google Calendar phone app
  so reminders reach your phone.
</p>
<label>
  <input type="checkbox" aria-label="Enable Google Calendar" checked={enabled} onchange={(e) => void toggle(e.currentTarget.checked)} />
  Enable notifications
</label>
<button onclick={connect}>Connect Google Calendar</button>
{#if calendars.length}
  <label>
    Calendar
    <select aria-label="Calendar" value={current} onchange={(e) => void pick(e.currentTarget.value)}>
      <option value="">Choose a calendar</option>
      {#each calendars as [id, title] (id)}<option value={id}>{title}</option>{/each}
    </select>
  </label>
{/if}
{#if error}<p role="alert">{error}</p>{/if}
