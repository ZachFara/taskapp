<script lang="ts" module>
  export type RowAction = { id: string; label: string; icon: string; danger?: boolean; run: () => void };
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import ContextMenu from './ContextMenu.svelte';
  import IconButton from './IconButton.svelte';
  import MenuItem from './MenuItem.svelte';

  // One "…" button at the row's right end, shown while the row is hovered or has focus. It opens the same actions menu
  // as right-click, Shift+F10 and the menu key. The name keeps the rest of the row, so a click on it opens the view.
  let { label, actions, children }: { label: string; actions: RowAction[]; children: Snippet } = $props();
  let open = $state(false);
  const key = (e: KeyboardEvent) => {
    if (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey)) {
      e.preventDefault();
      open = true;
    }
  };
</script>

<ContextMenu bind:open>
  {#snippet area()}
    <div class="rowactions" role="group" aria-label={label} onkeydown={key}>
      {@render children()}
      <div class="acts">
        <IconButton icon="more" label={`More actions for ${label}`} onclick={() => (open = true)} />
      </div>
    </div>
  {/snippet}
  {#each actions as a (a.id)}
    <MenuItem label={a.label} icon={a.icon} danger={a.danger} onSelect={a.run} />
  {/each}
</ContextMenu>

<style>
  :global(.cx-ctx-area) { display: contents; }
  .rowactions { position: relative; display: flex; align-items: center; flex: 1; min-width: 0; }
  /* The wide child (a link or ListRow) takes the width; the button takes its own slot, so nothing shifts on hover. */
  .rowactions > :global(.grow) { flex: 1; min-width: 0; }
  .acts { flex: none; display: flex; opacity: 0; pointer-events: none; transition: opacity var(--dur-1) var(--ease); }
  .rowactions:hover .acts,
  .rowactions:focus-within .acts { opacity: 1; pointer-events: auto; }
</style>
