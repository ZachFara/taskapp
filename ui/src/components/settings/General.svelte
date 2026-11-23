<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import Select from '../../ui/Select.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { setTheme, theme } from '../../lib/theme.svelte';
  import Switch from '../../ui/Switch.svelte';
  import { estimationItems, scaleItems } from '../insights/estimation';

  const KEY = /^[A-Z]{2,5}$/;
  const systemZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const zoneNames = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf?.('timeZone') ?? [];
  // A stored zone the browser's list lacks (the backend's default, "UTC", is one) is still offered, so it shows.
  const zoneItems = $derived(
    [systemZone, ...new Set([zone, ...zoneNames].filter((z) => z && z !== systemZone))].map((z) => ({
      value: z,
      label: z === systemZone ? `${z} (system)` : z,
    })),
  );
  const themeItems = [
    { value: 'system', label: 'System' },
    { value: 'light', label: 'Light' },
    { value: 'dark', label: 'Dark' },
  ];
  const weekItems = [
    { value: 'sunday', label: 'Sunday' },
    { value: 'monday', label: 'Monday' },
  ];

  let name = $state('');
  let key = $state('');
  let zone = $state(systemZone);
  let themeChoice = $state<string>(theme.mode);
  let week = $state('sunday');
  let error = $state('');
  let saved = $state(false);
  let estimation = $state('both');
  let scale = $state('fibonacci');
  let charts = $state(true);
  let sparklines = $state(false);

  const read = (k: string) => api.settings.get(k) as Promise<string | null>;

  onMount(async () => {
    name = (await read('workspace_name')) ?? '';
    key = (await read('issue_prefix')) ?? '';
    zone = (await read('timezone')) || systemZone;
    week = (await read('week_start')) ?? 'sunday';
    estimation = (await read('estimation')) ?? 'both';
    scale = (await read('point_scale')) ?? 'fibonacci';
    charts = ((await api.settings.get('insights_charts')) ?? true) !== false;
    sparklines = (await api.settings.get('sidebar_sparklines')) === true;
  });

  $effect(() => {
    const run = () => void save();
    addEventListener('command:settings.save', run);
    
    return () => removeEventListener('command:settings.save', run);
  });

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
      await api.settings.set('theme', themeChoice);
      await api.settings.set('estimation', estimation);
      await api.settings.set('point_scale', scale);
      await api.settings.set('insights_charts', charts);
      await api.settings.set('sidebar_sparklines', sparklines);
      setTheme(themeChoice as 'light' | 'dark' | 'system', false);
      saved = true;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<SectionHead lede="Name the workspace, choose its issue keys, and set how dates and the theme look." />

<form class="st-card" onsubmit={(e) => { e.preventDefault(); void save(); }}>
  <div class="st-form">
    <div class="st-field">
      <span>Workspace name</span>
      <TextInput label="Workspace name" placeholder="Home" bind:value={name} oninput={() => (saved = false)} />
    </div>
    <div class="st-field">
      <span>Workspace key</span>
      <TextInput label="Workspace key" bind:value={key} invalid={error !== ''} oninput={() => (saved = false)} />
      <span class="st-hint">New issues get keys such as {key || 'ABC'}-1. Existing issue keys keep working.</span>
    </div>
    <div class="st-field">
      <span>Time zone</span>
      <Select label="Time zone" items={zoneItems} bind:value={zone} />
      <span class="st-hint">Calendar alerts land at this time of day. A change applies after restart.</span>
    </div>
    <div class="st-field">
      <span>Week starts on</span>
      <Select label="Week starts on" items={weekItems} bind:value={week} />
    </div>
    <div class="st-field">
      <span>Theme</span>
      <Select label="Theme" items={themeItems} bind:value={themeChoice} />
    </div>
    <div class="st-field">
      <span>Estimation</span>
      <Select label="Estimation" items={estimationItems} bind:value={estimation} />
      <span class="st-hint">None and Time hide points everywhere.</span>
    </div>
    <div class="st-field">
      <span>Point scale</span>
      <Select label="Point scale" items={scaleItems} bind:value={scale} disabled={estimation === 'none' || estimation === 'time'} />
    </div>
    <div class="st-field">
      <span>Progress charts</span>
      <Switch label="Progress charts" bind:checked={charts} />
      <span class="st-hint">Burn-up charts on projects and cycles. The insights panel on a view still opens on demand.</span>
    </div>
    <div class="st-field">
      <span>Sidebar sparklines</span>
      <Switch label="Sidebar sparklines" bind:checked={sparklines} />
    </div>
  </div>
  <div class="st-actions">
    <Button type="submit" variant="primary">Save</Button>
    {#if error}<p role="alert" class="st-error">{error}</p>{/if}
    {#if saved}<span class="st-status" data-state="ok" data-status>Saved</span>{/if}
  </div>
</form>
