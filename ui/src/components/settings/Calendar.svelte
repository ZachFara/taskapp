<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import Select from '../../ui/Select.svelte';
  import Switch from '../../ui/Switch.svelte';

  let enabled = $state(false);
  let calendars = $state<[string, string][]>([]);
  let current = $state('');
  let error = $state('');
  // What the settings store holds; each effect writes only when the owner changed the value.
  let storedEnabled = false;
  let storedCurrent = '';

  const items = $derived(calendars.map(([id, title]) => ({ value: id, label: title })));
  const connected = $derived(calendars.length > 0);

  onMount(async () => {
    storedEnabled = enabled = ((await api.settings.get('gcal_enabled')) as boolean | null) === true;
    storedCurrent = current = ((await api.settings.get('gcal_calendar_id')) as string | null) ?? '';
  });

  $effect(() => {
    const on = enabled;
    if (on === storedEnabled) return;
    storedEnabled = on;
    void api.settings.set('gcal_enabled', on);
  });

  $effect(() => {
    const id = current;
    if (id === storedCurrent) return;
    storedCurrent = id;
    void api.settings.set('gcal_calendar_id', id);
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
</script>

<div class="st-head">
  <h2>Google Calendar</h2>
  <p class="st-lede">
    Use a dedicated calendar for these events, and view it in the Google Calendar phone app
    so reminders reach your phone.
  </p>
</div>

<div class="st-card">
  <span
    class="st-status"
    data-state={connected && enabled ? 'ok' : connected ? 'warn' : undefined}
    data-status
  >{!connected ? 'Not connected' : enabled ? 'Notifications on' : 'Connected, notifications off'}</span>
  <div class="st-actions">
    <Button variant="primary" onclick={connect}>Connect Google Calendar</Button>
  </div>
  <div class="st-actions">
    <Switch label="Enable Google Calendar" bind:checked={enabled} />
    <span>Enable notifications</span>
  </div>
  {#if connected}
    <div class="st-field">
      <span>Calendar</span>
      <Select label="Calendar" items={items} bind:value={current} placeholder="Choose a calendar" />
    </div>
  {/if}
  {#if error}<p role="alert" class="st-error">{error}</p>{/if}
</div>
