<script lang="ts" module>
  export type RowSub = { id: string; label: string; icon: string; indent?: number; run: () => void };
  /** With `sub`, the row opens a submenu of those choices instead of running. */
  export type RowAction = { id: string; label: string; icon: string; danger?: boolean; run: () => void; sub?: RowSub[] };
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';
  import { ContextMenu as Ctx } from 'bits-ui';
  import ContextMenu from './ContextMenu.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import MenuItem from './MenuItem.svelte';

  // One "…" button at the row's right end, shown while the row is hovered or has focus. It opens the same actions menu
  // as right-click, Shift+F10 and the menu key. The name keeps the rest of the row, so a click on it opens the view.
  // `button={false}` (the sidebar) shows no button at all: the row only highlights, and the menu is on right-click.
  let { label, actions, children, button = true }: { label: string; actions: RowAction[]; children: Snippet; button?: boolean } = $props();
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
      {#if button}
        <div class="acts">
          <IconButton icon="more" label={`More actions for ${label}`} onclick={() => (open = true)} />
        </div>
      {/if}
    </div>
  {/snippet}
  {#each actions as a (a.id)}
    {#if a.sub}
      <Ctx.Sub>
        <Ctx.SubTrigger class="cx-item">
          <span class="lead"><Icon name={a.icon} /></span>
          <span class="name">{a.label}</span>
          <Icon name="chevron-right" size={12} />
        </Ctx.SubTrigger>
        <Ctx.Portal>
          <Ctx.SubContent class="cx-surface cx-menu" data-kbd-inherit-items>
            {#each a.sub as s (s.id)}<MenuItem label={s.label} icon={s.icon} indent={s.indent} onSelect={s.run} />{/each}
          </Ctx.SubContent>
        </Ctx.Portal>
      </Ctx.Sub>
    {:else}
      <MenuItem label={a.label} icon={a.icon} danger={a.danger} onSelect={a.run} />
    {/if}
  {/each}
</ContextMenu>

<style>
  :global(.cx-ctx-area) { display: contents; }
  .lead { width: var(--icon); display: grid; color: var(--muted); }
  .name { flex: 1; }
  .rowactions { position: relative; display: flex; align-items: center; flex: 1; min-width: 0; }
  /* The wide child (a link or ListRow) takes the width; the button takes its own slot, so nothing shifts on hover. */
  .rowactions > :global(.grow) { flex: 1; min-width: 0; }
  .acts { flex: none; display: flex; opacity: 0; pointer-events: none; transition: opacity var(--dur-1) var(--ease); }
  .rowactions:hover .acts,
  .rowactions:focus-within .acts { opacity: 1; pointer-events: auto; }
</style>
