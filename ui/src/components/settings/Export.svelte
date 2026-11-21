<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

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

<h2>Export</h2>
<form onsubmit={(e) => { e.preventDefault(); void save(); }}>
  <label>
    <input type="checkbox" aria-label="Enable export" bind:checked={enabled} />
    Enable export to a folder
  </label>
  <label>
    Export directory
    <input aria-label="Export directory" bind:value={dir} />
  </label>
  <label>
    Git remote (optional)
    <input aria-label="Export remote" bind:value={remote} />
  </label>
  {#if error}<p role="alert">{error}</p>{/if}
  <button type="submit">Save</button>
</form>
<button onclick={exportNow}>Export now</button>
{#if result}<p>{result}</p>{/if}
