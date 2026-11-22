<script lang="ts">
  import type { Snippet } from 'svelte';
  import Kbd from '../../ui/Kbd.svelte';
  import Popover from '../../ui/Popover.svelte';
  import { shortcutFor } from '../../lib/commands/registry';

  // One line of the property plate: name, current value as the trigger, the editor in a popover.
  // `openRow` is shared by the whole plate so only one editor is open and Esc can close it.
  let { id, name, openRow = $bindable(null), cmd, value, editor }: {
    id: string;
    name: string;
    openRow?: string | null;
    cmd?: string;
    value: Snippet;
    editor: Snippet<[() => void]>;
  } = $props();
  const keys = $derived(cmd ? shortcutFor(cmd)?.split('+') : undefined);
  const close = () => (openRow = null);
</script>

<div class="row">
  <span class="k">{name}</span>
  <Popover bind:open={() => openRow === id, (v) => (openRow = v ? id : openRow === id ? null : openRow)}>
    {#snippet trigger(props)}
      <button {...props} type="button" class="v" aria-label={name}>{@render value()}</button>
    {/snippet}
    {@render editor(close)}
  </Popover>
  <span class="kbd">{#if keys}<Kbd {keys} />{/if}</span>
</div>

<style>
  .row { display: grid; grid-template-columns: 6.5rem 1fr auto; align-items: center; gap: var(--s-4); min-height: var(--row-h); border-bottom: 1px dotted var(--control); }
  .k { font-size: var(--fs-xs); font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--muted); }
  .v { display: flex; align-items: center; gap: var(--s-4); min-height: var(--ctl-h-sm); padding: 0 var(--s-3); border: 0; border-radius: var(--r-sm); background: transparent; text-align: left; font-weight: 500; min-width: 0; }
  .v:hover { background: var(--active); }
  .kbd { display: flex; gap: var(--s-1); opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  .row:hover .kbd, .row:focus-within .kbd { opacity: 1; }
</style>
