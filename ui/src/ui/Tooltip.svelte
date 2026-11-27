<script lang="ts">
  import { Tooltip } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import Kbd from './Kbd.svelte';

  // Action name plus its shortcut. Opens after --delay-tooltip of hover, and at once on keyboard focus.
  let {
    label,
    shortcut,
    disabled = false,
    children,
  }: {
    label: string;
    shortcut?: string | string[];
    disabled?: boolean;
    children: Snippet<[Record<string, unknown>]>;
  } = $props();
  // The token is read once per tooltip; 500 only covers a page where tokens.css is absent.
  const delay = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--delay-tooltip')) || 500;
</script>

<Tooltip.Provider delayDuration={delay} skipDelayDuration={0}>
  <Tooltip.Root {disabled}>
    <Tooltip.Trigger>
      {#snippet child({ props })}{@render children(props)}{/snippet}
    </Tooltip.Trigger>
    <Tooltip.Portal>
      <Tooltip.Content class="cx-tip" sideOffset={6} data-kbd-inherit>
        <span>{label}</span>
        {#if shortcut}<Kbd keys={shortcut} />{/if}
      </Tooltip.Content>
    </Tooltip.Portal>
  </Tooltip.Root>
</Tooltip.Provider>

<style>
  :global(.cx-tip) {
    z-index: var(--z-tooltip);
    display: flex;
    align-items: center;
    gap: var(--s-4);
    padding: var(--s-2) var(--s-4);
    border-radius: var(--r-ctl);
    background: var(--tip-bg);
    color: var(--tip-ink);
    font: 500 var(--fs-sm) / 1.3 var(--font);
    white-space: nowrap;
  }
</style>
