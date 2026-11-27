<script lang="ts">
  import { Popover } from 'bits-ui';
  import type { Snippet } from 'svelte';

  let {
    trigger,
    children,
    open = $bindable(false),
  }: { trigger: Snippet<[Record<string, unknown>]>; children: Snippet; open?: boolean } = $props();
</script>

<Popover.Root bind:open>
  <Popover.Trigger>
    {#snippet child({ props })}{@render trigger(props)}{/snippet}
  </Popover.Trigger>
  <Popover.Portal>
    <Popover.Content class="cx-surface cx-popover" sideOffset={4} align="start" collisionPadding={10}>
      {@render children()}
    </Popover.Content>
  </Popover.Portal>
</Popover.Root>

<style>
  :global(.cx-popover) { min-width: 240px; padding: var(--s-6); }
</style>
