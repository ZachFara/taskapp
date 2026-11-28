<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { deleteAsk } from '../lib/deleteIssues.svelte';
  import { selection } from '../lib/selection.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';

  // The one confirmation for deleting issues for good, from the archive page and from an issue's menu.
  let open = $state(false);
  let error = $state('');
  $effect(() => {
    open = deleteAsk.n > 0;
  });
  const what = $derived(`${deleteAsk.n} issue${deleteAsk.n === 1 ? '' : 's'}`);

  async function confirm() {
    try {
      if (deleteAsk.empty) await api.issues.emptyArchive();
      else await api.issues.delete(deleteAsk.ids);
      selection.clear();
      deleteAsk.n = 0;
      error = '';
      // An open issue that was just deleted has nothing left to show.
      if (location.pathname.startsWith('/issue/')) navigate('/archive');
      dispatchEvent(new Event('command:view.refresh'));
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<Dialog
  bind:open
  onclosed={() => (deleteAsk.n = 0)}
  title={`Delete ${what} permanently?`}
  description={`This removes ${what} for good, with their comments, relations, links, labels and focus sessions. It cannot be undone later; only Undo, right after, brings them back.`}
>
  {#if error}<p role="alert" class="error">{error}</p>{/if}
  {#snippet actions()}
    <Button onclick={() => (open = false)}>Cancel</Button>
    <Button variant="danger" onclick={confirm}>Delete {what}</Button>
  {/snippet}
</Dialog>

<style>
  .error { color: var(--signal); }
</style>
