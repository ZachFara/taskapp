<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import Select from '../../ui/Select.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  const KEY = /^[A-Z]{2,5}$/;
  const systemZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const zoneNames = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf?.('timeZone') ?? [];
  const zoneItems = [systemZone, ...zoneNames.filter((z) => z !== systemZone)].map((z) => ({
    value: z,
    label: z === systemZone ? `${z} (system)` : z,
  }));
  const themeItems = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];
  const weekItems = [
    { value: 'sunday', label: 'Sunday' },
    { value: 'monday', label: 'Monday' },
    { value: 'saturday', label: 'Saturday' },
  ];

  let name = $state('');
  let key = $state('');
  let zone = $state(systemZone);
  let theme = $state('system');
  let week = $state('sunday');
  let error = $state('');
  let saved = $state(false);

  const read = (k: string) => api.settings.get(k) as Promise<string | null>;

  onMount(async () => {
    name = (await read('workspace_name')) ?? '';
    key = (await read('issue_prefix')) ?? '';
    zone = (await read('timezone')) ?? systemZone;
    theme = (await read('theme')) ?? 'system';
    week = (await read('week_start')) ?? 'sunday';
  });

  // Dark and light are set on the root so the tokens switch at once; "system" removes the override.
  function applyTheme(t: string) {
    if (t === 'system') delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = t;
  }

  async function save() {
    if (!KEY.test(key)) {
      error = 'Use 2 to 5 uppercase letters, such as ABC.';
      saved = false;
      return;
    }
    error = '';
    try {
      await api.settings.set('workspace_name', name.trim());
      await api.settings.set('issue_prefix', key);
      await api.settings.set('timezone', zone);
      await api.settings.set('week_start', week);
      await api.settings.set('theme', theme);
      applyTheme(theme);
      saved = true;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<div class="st-head">
  <h2>General</h2>
  <p class="st-lede">Name the workspace, choose its issue keys, and set how dates and the theme look.</p>
</div>

<form class="st-card" onsubmit={(e) => { e.preventDefault(); void save(); }}>
  <div class="st-form">
    <div class="st-field">
      <span>Workspace name</span>
      <TextInput label="Workspace name" bind:value={name} oninput={() => (saved = false)} />
    </div>
    <div class="st-field">
      <span>Workspace key</span>
      <TextInput label="Workspace key" bind:value={key} invalid={error !== ''} oninput={() => (saved = false)} />
      <span class="st-hint">New issues get keys such as {key || 'ABC'}-1. Existing issue keys keep working.</span>
    </div>
    <div class="st-field">
      <span>Time zone</span>
      <Select label="Time zone" items={zoneItems} bind:value={zone} />
      <span class="st-hint">Calendar alerts land at this time of day.</span>
    </div>
    <div class="st-field">
      <span>Week starts on</span>
      <Select label="Week starts on" items={weekItems} bind:value={week} />
    </div>
    <div class="st-field">
      <span>Theme</span>
      <Select label="Theme" items={themeItems} bind:value={theme} />
    </div>
  </div>
  <div class="st-actions">
    <Button type="submit" variant="primary">Save</Button>
    {#if error}<p role="alert" class="st-error">{error}</p>{/if}
    {#if saved}<span class="st-status" data-state="ok" data-status>Saved</span>{/if}
  </div>
</form>
