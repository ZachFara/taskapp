<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let dir = $state('');
  let files = $state<string[]>([]);
  let error = $state('');

  const fail = (e: unknown) => (error = (e as { message?: string }).message ?? String(e));

  async function load() {
    files = dir.trim() === '' ? [] : await api.backup.list(dir.trim());
  }

  onMount(async () => {
    dir = ((await api.settings.get('backup_dir')) as string | null) ?? '';
    await load();
  });

  async function saveDir() {
    await api.settings.set('backup_dir', dir.trim());
    await load();
  }

  async function snapshot() {
    error = '';
    if (dir.trim() === '') {
      error = 'Choose a backup directory first.';
      return;
    }
    try {
      await api.backup.snapshot(dir.trim(), 'manual');
      await load();
    } catch (e) {
      fail(e);
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
{#if files.length}
  <p>To restore, quit the app and run: taskapp backup restore &lt;file&gt;</p>
  <ul>
    {#each files as path (path)}
      <li><code>{path}</code></li>
    {/each}
  </ul>
{/if}
