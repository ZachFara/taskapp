<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import Button from '../../ui/Button.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  let dir = $state('');
  let files = $state<string[]>([]);
  let loaded = $state(false);
  let error = $state('');

  const fail = (e: unknown) => (error = (e as { message?: string }).message ?? String(e));

  async function load() {
    files = dir.trim() === '' ? [] : await api.backup.list(dir.trim());
    loaded = true;
  }

  onMount(async () => {
    dir = ((await api.settings.get('backup_dir')) as string | null) ?? '';
    await load();
  });

  async function saveDir() {
    error = '';
    try {
      await api.settings.set('backup_dir', dir.trim());
      await load();
    } catch (e) {
      fail(e);
    }
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

<div class="st-head">
  <h2>Backups</h2>
  <p class="st-lede">Snapshots of the whole database, written to a folder you choose.</p>
</div>

<form class="st-card" onsubmit={(e) => { e.preventDefault(); void saveDir(); }}>
  <div class="st-field">
    <span>Backup directory</span>
    <TextInput label="Backup directory" bind:value={dir} />
  </div>
  <div class="st-actions">
    <Button type="submit">Save folder</Button>
    <Button variant="primary" onclick={snapshot}>Snapshot now</Button>
  </div>
  {#if error}<p role="alert" class="st-error">{error}</p>{/if}
</form>

<div class="st-card">
  {#if !loaded}
    <Skeleton height="var(--row-h)" />
  {:else if files.length}
    <p class="st-hint">To restore, quit the app and run: taskapp backup restore &lt;file&gt;</p>
    <ul class="st-list">
      {#each files as path (path)}
        <li><span class="st-path">{path}</span></li>
      {/each}
    </ul>
  {:else}
    <EmptyState title="No snapshots yet" hint="Snapshot now writes one to the folder above." />
  {/if}
</div>
