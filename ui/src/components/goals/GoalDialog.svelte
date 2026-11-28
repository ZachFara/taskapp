<script lang="ts">
  import type { Goal } from '../../lib/bindings';
  import { quarterOf } from '../../lib/goals';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  /** `goal` is the one being edited, or null for a new goal. */
  let { open = $bindable(false), goal = null, onsave }: { open?: boolean; goal?: Goal | null; onsave: (v: { title: string; quarter: string; description: string }) => Promise<void> } = $props();
  let title = $state('');
  let quarter = $state('');
  let description = $state('');
  let error = $state('');

  $effect(() => {
    if (!open) return;
    [title, quarter, description, error] = [goal?.title ?? '', goal?.quarter ?? quarterOf(), goal?.description ?? '', ''];
  });

  async function save() {
    error = '';
    try {
      await onsave({ title, quarter: quarter.trim(), description });
      open = false;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<Dialog bind:open title={goal ? 'Edit goal' : 'New goal'}>
  <form onsubmit={(e) => (e.preventDefault(), void save())}>
    <TextInput label="Goal title" placeholder="What do you want to reach?" bind:value={title} />
    <TextInput label="Goal quarter" placeholder="2027-Q1" bind:value={quarter} />
    <TextArea label="Goal description" placeholder="Why it matters (optional)" rows={3} bind:value={description} />
    {#if error}<p role="alert" class="error">{error}</p>{/if}
  </form>
  {#snippet actions()}
    <Button onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" disabled={!title.trim()} onclick={save}>{goal ? 'Save' : 'Create goal'}</Button>
  {/snippet}
</Dialog>

<style>
  form { display: grid; gap: var(--s-4); }
  .error { margin: 0; color: var(--warning); font-size: var(--fs-sm); }
</style>
