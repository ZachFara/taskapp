<script lang="ts">
  import { DropdownMenu } from 'bits-ui';
  import type { Snippet } from 'svelte';

  // `trigger` receives the props to spread on the element that opens the menu; children are MenuItems.
  let {
    trigger,
    children,
    open = $bindable(false),
  }: { trigger: Snippet<[Record<string, unknown>]>; children: Snippet; open?: boolean } = $props();
</script>

<DropdownMenu.Root bind:open>
  <DropdownMenu.Trigger>
    {#snippet child({ props })}{@render trigger(props)}{/snippet}
  </DropdownMenu.Trigger>
  <DropdownMenu.Portal>
    <DropdownMenu.Content class="cx-surface cx-menu" sideOffset={4} align="start" data-kbd-inherit-items>
      {@render children()}
    </DropdownMenu.Content>
  </DropdownMenu.Portal>
</DropdownMenu.Root>

<style>
  :global(.cx-surface) {
    z-index: var(--z-popover);
    background: var(--sheet);
    color: var(--ink);
    border: 1px solid var(--control);
    border-radius: var(--r-pop);
    box-shadow: var(--shadow-float);
    animation: cx-pop var(--dur-2) var(--ease);
  }
  :global(.cx-menu) { min-width: 200px; padding: var(--s-2); }
  @keyframes cx-pop { from { opacity: 0; transform: translateY(-2px); } }
</style>
