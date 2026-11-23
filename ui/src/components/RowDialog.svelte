<script lang="ts">
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import TextInput from '../ui/TextInput.svelte';

  // One dialog for a row's rename and delete actions. `onconfirm` gets the new name (rename) or '' (delete).
  let {
    open = $bindable(false),
    mode,
    title,
    value = '',
    description,
    onconfirm,
  }: { open?: boolean; mode: 'rename' | 'delete'; title: string; value?: string; description?: string; onconfirm: (v: string) => void } = $props();
  let draft = $state('');
  $effect(() => {
    if (open) draft = value;
  });
  const save = () => {
    const v = draft.trim();
    if (mode === 'rename' && v) onconfirm(v);
    if (mode === 'delete') onconfirm('');
  };
</script>

<Dialog bind:open {title} {description}>
  {#if mode === 'rename'}
    <form onsubmit={(e) => (e.preventDefault(), save())}>
      <TextInput label="Name" bind:value={draft} />
    </form>
  {/if}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (open = false)}>Cancel</Button>
    {#if mode === 'rename'}
      <Button variant="primary" onclick={save}>Rename</Button>
    {:else}
      <Button variant="danger" onclick={save}>Delete</Button>
    {/if}
  {/snippet}
</Dialog>
