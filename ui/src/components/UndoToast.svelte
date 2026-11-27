<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { shortcutFor } from '../lib/commands';
  import Button from '../ui/Button.svelte';
  import Toast from '../ui/Toast.svelte';
  import { dismiss, markLoaded, onWrote, runRedo, runUndo, undoToast } from './undoState.svelte';

  // The app dispatches `history:wrote` after a write; the toast then names the step it can undo.
  $effect(() => {
    void markLoaded();
    const wrote = () => void onWrote();
    window.addEventListener('history:wrote', wrote);
    return () => window.removeEventListener('history:wrote', wrote);
  });
</script>

{#each undoToast.items as m (m.id)}
  {@const created = /^Created (\S+)$/.exec(m.label)?.[1]}
  <Toast tone={m.tone} ondismiss={() => dismiss(m.id)}>
    {m.label}
    {#snippet action()}
      {#if m.action}
        <span class="on-dark">
          <Button variant="quiet" kbd={shortcutFor(m.action.run === 'undo' ? 'history.undo' : 'history.redo')} onclick={() => (m.action?.run === 'undo' ? runUndo(api) : runRedo(api))}>{m.action.text}</Button>
          {#if created}<Button variant="quiet" onclick={() => (dismiss(m.id), navigate(`/issue/${created}`))}>Open</Button>{/if}
        </span>
      {/if}
    {/snippet}
  </Toast>
{/each}

<style>
  /* The toast is a dark surface in both themes; the buttons borrow its ink so they stay readable. */
  .on-dark { display: inline-flex; gap: var(--s-2); --ink: var(--tip-ink); --control: var(--tip-ink); --active: transparent; }
</style>
