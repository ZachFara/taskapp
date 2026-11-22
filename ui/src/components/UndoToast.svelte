<script lang="ts">
  import { api } from '../lib/api';
  import Button from '../ui/Button.svelte';
  import Toast from '../ui/Toast.svelte';
  import { dismiss, onWrote, runRedo, runUndo, undoToast } from './undoState.svelte';

  // The app dispatches `history:wrote` after a write; the toast then names the step it can undo.
  $effect(() => {
    const wrote = () => void onWrote();
    window.addEventListener('history:wrote', wrote);
    return () => window.removeEventListener('history:wrote', wrote);
  });
</script>

{#if undoToast.value}
  {@const m = undoToast.value}
  <Toast tone={m.tone} ondismiss={dismiss}>
    {m.label}{#if m.action} · {/if}
    {#snippet action()}
      {#if m.action}
        <Button variant="quiet" onclick={() => (m.action?.run === 'undo' ? runUndo(api) : runRedo(api))}>{m.action.text}</Button>
      {/if}
    {/snippet}
  </Toast>
{/if}
