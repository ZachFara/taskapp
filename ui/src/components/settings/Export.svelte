<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import Switch from '../../ui/Switch.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  let enabled = $state(false);
  let dir = $state('');
  let remote = $state('');
  let error = $state('');
  let result = $state('');

  onMount(async () => {
    enabled = ((await api.settings.get('export_enabled')) as boolean | null) === true;
    dir = ((await api.settings.get('export_dir')) as string | null) ?? '';
    remote = ((await api.settings.get('export_remote')) as string | null) ?? '';
  });

  $effect(() => {
    const run = () => void save();
    addEventListener('command:settings.save', run);
    return () => removeEventListener('command:settings.save', run);
  });

  async function save() {
    if (enabled && dir.trim() === '') {
      error = 'Choose an export directory before turning export on.';
      return;
    }
    error = '';
    try {
      await api.settings.set('export_enabled', enabled);
      await api.settings.set('export_dir', dir.trim());
      await api.settings.set('export_remote', remote.trim());
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function exportNow() {
    error = '';
    try {
      result = '';
      await api.export.now();
      result = 'Exported.';
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<SectionHead lede="Keep a plain-file copy of every task in a folder, optionally pushed to a Git remote." />

<form class="st-card" onsubmit={(e) => { e.preventDefault(); void save(); }}>
  <span class="st-status" data-state={enabled ? 'ok' : undefined} data-status>
    {enabled ? (dir.trim() ? `Exporting to ${dir.trim()}` : 'Export on, no folder yet') : 'Export is off'}
  </span>
  <div class="st-actions">
    <Switch label="Enable export" bind:checked={enabled} />
    <span>Enable export to a folder</span>
  </div>
  <div class="st-form">
    <div class="st-field">
      <span>Export directory</span>
      <TextInput label="Export directory" placeholder="A folder such as ~/Documents/Cairn" bind:value={dir} />
    </div>
    <div class="st-field">
      <span>Git remote (optional)</span>
      <TextInput label="Export remote" placeholder="git@github.com:you/cairn-tasks.git" bind:value={remote} />
    </div>
  </div>
  <div class="st-actions">
    <Button type="submit" variant="primary">Save</Button>
    <Button onclick={exportNow}>Export now</Button>
    {#if result}<span class="st-hint">{result}</span>{/if}
    {#if error}<p role="alert" class="st-error">{error}</p>{/if}
  </div>
</form>
