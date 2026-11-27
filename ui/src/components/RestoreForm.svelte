<script lang="ts">
  import { api } from '../lib/api';
  import Button from '../ui/Button.svelte';
  import Spinner from '../ui/Spinner.svelte';
  import Switch from '../ui/Switch.svelte';
  import TextInput from '../ui/TextInput.svelte';

  /** The source field, the machine-settings switch and the Restore button; the caller says what happens next. */
  let { onrestored }: { onrestored: (folder: string) => void | Promise<void> } = $props();

  let source = $state('');
  let applyLocal = $state(true);
  let busy = $state(false);
  let error = $state('');

  const isUrl = $derived(/^[a-z]+:\/\//i.test(source.trim()) || /^[^\s/@]+@[^\s/:]+:/.test(source.trim()));

  async function restore(e: SubmitEvent) {
    e.preventDefault();
    if (busy) return;
    if (!source.trim()) {
      error = 'Enter the export folder or the Git address of the repository.';
      return;
    }
    busy = true;
    error = '';
    try {
      const folder = await api.restore.fromExport(source.trim(), null, applyLocal);
      await onrestored(folder);
    } catch (err) {
      error = (err as { message?: string }).message ?? String(err);
    } finally {
      busy = false;
    }
  }
</script>

<form class="restore" onsubmit={restore} novalidate>
  <label class="row">
    <span>Export folder or Git address</span>
    <TextInput label="Export source" bind:value={source} placeholder="~/Documents/Cairn or git@github.com:you/cairn-tasks.git" invalid={!!error} />
  </label>
  <p class="hint">{isUrl ? 'The repository is cloned with your own Git sign-in; nothing is stored.' : 'A folder an earlier export wrote.'}</p>
  <div class="switch">
    <Switch label="Also restore this machine's settings" bind:checked={applyLocal} />
    <span>Also restore the export's remote and backup folder</span>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    {#if busy}<span class="progress" role="status"><Spinner size={14} label="Restoring" /> {isUrl ? 'Cloning and restoring…' : 'Restoring…'}</span>{/if}
    <Button variant="primary" type="submit" disabled={busy}>Restore</Button>
  </div>
</form>

<style>
  .restore { display: grid; gap: var(--s-6); }
  .row { display: grid; gap: var(--s-3); }
  .row > span { color: var(--muted); font-size: var(--fs-sm); font-weight: 500; }
  .hint { color: var(--muted); font-size: var(--fs-sm); }
  .switch { display: flex; align-items: center; gap: var(--s-4); font-size: var(--fs-sm); }
  .error { color: var(--signal); font-size: var(--fs-sm); }
  .actions { display: flex; justify-content: flex-end; align-items: center; gap: var(--s-4); flex-wrap: wrap; }
  .progress { display: inline-flex; align-items: center; gap: var(--s-3); color: var(--muted); font-size: var(--fs-sm); }
</style>
