<script lang="ts">
  import type { Issue, IssuePatch } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import { images, openLinks, renderMarkdown } from './markdown';
  import { checklistLines, toggleLine } from './checklist';
  import IconButton from '../../ui/IconButton.svelte';

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
  // A checkbox click rewrites its own line (the box numbering comes from markdown.ts). Anything else opens the editor.
  function tick(e: MouseEvent, src: string, write: (s: string) => void): boolean {
    const box = (e.target as Element | null)?.closest<HTMLInputElement>('input[data-check]');
    if (!box) return false;
    e.preventDefault();
    const item = checklistLines(src)[Number(box.dataset.check)];
    if (item) write(toggleLine(src, item.line));
    return true;
  }
</script>

<section aria-label="Description" bind:this={box}>
  {#if editing}
    <div class="modes" role="group" aria-label="Description mode">
      <Button variant={preview ? 'quiet' : 'primary'} onclick={() => (preview = false)}>Write</Button>
      <Button variant={preview ? 'primary' : 'quiet'} onclick={() => (preview = true)}>Preview</Button>
    </div>
    {#if preview}
      <div class="md" aria-label="Description preview" use:openLinks use:images={draft} onclick={(e) => tick(e, draft, (s) => (draft = s))}>{@html renderMarkdown(draft)}</div>
    {:else}
      <TextArea label="Description text" images rows={8} bind:value={draft} />
    {/if}
    <div class="acts"><Button variant="primary" onclick={commit}>Save</Button><Button onclick={() => (editing = false)}>Cancel</Button></div>
  {:else}
    <div class="view">
      <!-- Not a button: a checkbox inside a button is invalid. The text opens the editor; the pencil is the keyboard path. -->
      <div class="md" use:openLinks use:images={issue.description} onclick={(e) => { if (!tick(e, issue.description, (s) => save({ description: s }))) editing = true; }}>
        {#if issue.description}{@html renderMarkdown(issue.description)}{:else}<span class="hint">Add a description</span>{/if}
      </div>
      <IconButton icon="edit" label="Edit description" onclick={() => (editing = true)} />
    </div>
  {/if}
</section>

<style>
  section { display: grid; gap: var(--s-4); }
  .modes, .acts { display: flex; gap: var(--s-3); }
  .view { display: flex; gap: var(--s-3); align-items: flex-start; min-height: var(--s-8); padding: var(--s-4) var(--s-5); border: 1px dashed transparent; border-radius: var(--r-ctl); }
  .view:hover { border-color: var(--control); }
  .view > .md { flex: 1; min-width: 0; cursor: text; }
  .md :global(input[type=checkbox]) { margin: 0 var(--s-3) 0 0; accent-color: var(--accent); }
  .hint { color: var(--muted); }
  .md { line-height: var(--lh-body); }
  .md :global(p) { margin: 0 0 var(--s-4); }
  .md :global(code) { background: var(--active); border-radius: var(--r-sm); padding: 0 var(--s-2); }
  .md :global(a) { color: var(--accent); text-decoration: underline; }
  .md :global(img) { max-width: 100%; border-radius: var(--r-sm); cursor: zoom-in; }
</style>
