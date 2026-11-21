<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';

  let dir = $state('');
  let files = $state<string[]>([]);
  let error = $state('');
  let restored = $state('');

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
    error = restored = '';
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

  async function restore(path: string) {
    error = restored = '';
    try {
      await api.backup.restore(path);
      restored = `Restored ${path}. Restart the app.`;
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
{#if restored}<p>{restored}</p>{/if}
{#if files.length}
  <ul>
    {#each files as path (path)}
      <li>
        <code>{path}</code>
        <button onclick={() => restore(path)}>Restore</button>
      </li>
    {/each}
  </ul>
{/if}
