<script lang="ts" module>
  export type RowAction = { id: string; label: string; icon: string; danger?: boolean; run: () => void };
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import ContextMenu from './ContextMenu.svelte';
  import IconButton from './IconButton.svelte';
  import MenuItem from './MenuItem.svelte';

  // The row's actions sit over its right edge while the row is hovered or has focus. They take no layout space,
  // and they are not clickable while hidden. Right-click, Shift+F10 and the menu key open the same actions as a menu.
  // `menu={false}` for rows that already have their own menu.
  let {
    label,
    actions,
    menu = true,
    children,
  }: { label: string; actions: RowAction[]; menu?: boolean; children: Snippet } = $props();
  let open = $state(false);
  const key = (e: KeyboardEvent) => {
    if (menu && (e.key === 'ContextMenu' || (e.key === 'F10' && e.shiftKey))) {
      e.preventDefault();
      open = true;
    }
  };
</script>

{#snippet bar()}
  <div class="rowactions" role="group" aria-label={label} onkeydown={key}>
    {@render children()}
    <div class="acts">
      {#each actions as a (a.id)}
        <IconButton icon={a.icon} label={a.label} onclick={a.run} />
      {/each}
    </div>
  </div>
{/snippet}

{#if menu}
  <ContextMenu bind:open>
    {#snippet area()}{@render bar()}{/snippet}
    {#each actions as a (a.id)}
      <MenuItem label={a.label} icon={a.icon} danger={a.danger} onSelect={a.run} />
    {/each}
  </ContextMenu>
{:else}
  {@render bar()}
{/if}

<style>
  :global(.cx-ctx-area) { display: contents; }
  .rowactions { position: relative; display: flex; align-items: center; flex: 1; min-width: 0; }
  /* The wide child (a link or ListRow) takes the width; the actions overlay its right edge. */
  .rowactions > :global(.grow) { flex: 1; min-width: 0; }
  .acts {
    position: absolute;
    right: var(--s-2);
    background: var(--acts-bg, var(--active)); /* hides the text the buttons sit over; the host sets its own row colour */
    border-radius: var(--r-ctl);
    display: flex;
    gap: var(--s-1);
    opacity: 0;
    pointer-events: none;
    transition: opacity var(--dur-1) var(--ease);
  }
  .rowactions:hover .acts,
  .rowactions:focus-within .acts { opacity: 1; pointer-events: auto; }
</style>
