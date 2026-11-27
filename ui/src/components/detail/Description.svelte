<script lang="ts">
  import type { Issue, IssuePatch } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import { images, openLinks, renderMarkdown } from './markdown';

  // `editing` lives in the screen so Esc can close it before the panel; Esc discards the draft.
  let { issue, save, editing = $bindable(false) }: { issue: Issue; save: (p: IssuePatch) => void; editing?: boolean } = $props();
  let draft = $state('');
  let preview = $state(false);

  $effect(() => {
    if (editing) {
      draft = issue.description;
      preview = false;
    }
  });
  let box: HTMLElement | undefined = $state();
  // The key that opens the editor must not leak into the shortcut layer, so focus moves in with it.
  $effect(() => {
    if (editing && !preview) queueMicrotask(() => box?.querySelector('textarea')?.focus());
  });
  const commit = () => {
    editing = false;
    if (draft !== issue.description) save({ description: draft });
  };
</script>

<section aria-label="Description" bind:this={box}>
  {#if editing}
    <div class="modes" role="group" aria-label="Description mode">
      <Button variant={preview ? 'quiet' : 'primary'} onclick={() => (preview = false)}>Write</Button>
      <Button variant={preview ? 'primary' : 'quiet'} onclick={() => (preview = true)}>Preview</Button>
    </div>
    {#if preview}
      <div class="md" aria-label="Description preview" use:openLinks use:images={draft}>{@html renderMarkdown(draft)}</div>
    {:else}
      <TextArea label="Description text" images rows={8} bind:value={draft} />
    {/if}
    <div class="acts"><Button variant="primary" onclick={commit}>Save</Button><Button onclick={() => (editing = false)}>Cancel</Button></div>
  {:else}
    <button type="button" class="md view" aria-label="Edit description" onclick={() => (editing = true)} use:openLinks use:images={issue.description}>
      {#if issue.description}{@html renderMarkdown(issue.description)}{:else}<span class="hint">Add a description</span>{/if}
    </button>
  {/if}
</section>

<style>
  section { display: grid; gap: var(--s-4); }
  .modes, .acts { display: flex; gap: var(--s-3); }
  .view { display: block; width: 100%; min-height: var(--s-8); padding: var(--s-4) var(--s-5); border: 1px dashed transparent; border-radius: var(--r-ctl); background: transparent; text-align: left; font-weight: 400; }
  .view:hover { border-color: var(--control); }
  .hint { color: var(--muted); }
  .md { line-height: var(--lh-body); }
  .md :global(p) { margin: 0 0 var(--s-4); }
  .md :global(code) { background: var(--active); border-radius: var(--r-sm); padding: 0 var(--s-2); }
  .md :global(a) { color: var(--accent); text-decoration: underline; }
  .md :global(img) { max-width: 100%; border-radius: var(--r-sm); cursor: zoom-in; }
</style>
