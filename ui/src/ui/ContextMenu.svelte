<script lang="ts">
  import { ContextMenu } from 'bits-ui';
  import { setContext, type Snippet } from 'svelte';
  // The shared surface styles live in Menu.svelte; importing it keeps them when only ContextMenu is used.
  import './Menu.svelte';

  // `area` is the region that opens the menu on right-click; children are MenuItems (same API as Menu).
  let { area, children, open = $bindable(false) }: { area: Snippet; children: Snippet; open?: boolean } = $props();
  setContext('cx-menu', 'context');
</script>

<ContextMenu.Root bind:open>
  <ContextMenu.Trigger class="cx-ctx-area">{@render area()}</ContextMenu.Trigger>
  <ContextMenu.Portal>
    <ContextMenu.Content class="cx-surface cx-menu" data-kbd-inherit-items>
      {@render children()}
    </ContextMenu.Content>
  </ContextMenu.Portal>
</ContextMenu.Root>
