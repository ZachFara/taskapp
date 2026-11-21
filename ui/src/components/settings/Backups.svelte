<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let dir = $state('');
  let snapshots = $state<string[]>([]);
  let error = $state('');

  onMount(async () => {
    dir = ((await api.settings.get('backup_dir')) as string | null) ?? '';
  });

  async function saveDir() {
    await api.settings.set('backup_dir', dir.trim());
  }

  async function snapshot() {
    error = '';
    if (dir.trim() === '') {
      error = 'Choose a backup directory first.';
      return;
    }
    try {
      snapshots = [await api.backup.snapshot(dir.trim(), 'manual'), ...snapshots];
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>Backups</h2>
<label>
  Backup directory
  <input aria-label="Backup directory" bind:value={dir} onchange={() => void saveDir()} />
</label>
<button onclick={snapshot}>Snapshot now</button>
{#if error}<p role="alert">{error}</p>{/if}
{#if snapshots.length}
  <ul>
    {#each snapshots as path (path)}
      <li>
        <code>{path}</code>
        <button disabled title="No restore command yet">Restore</button>
      </li>
    {/each}
  </ul>
{/if}
