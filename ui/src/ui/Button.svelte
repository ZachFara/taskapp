<script lang="ts">
  import { mergeProps } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import Kbd from './Kbd.svelte';
  import Tooltip from './Tooltip.svelte';

  type Props = {
    variant?: 'primary' | 'quiet' | 'danger';
    icon?: string;
    kbd?: string;
    tooltip?: { label: string; shortcut?: string | string[] };
    disabled?: boolean;
    type?: 'button' | 'submit';
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
    // Trigger props from a bits-ui Popover/Menu/Tooltip trigger land here.
    [key: string]: unknown;
  };
  let { variant = 'quiet', icon, kbd, tooltip, disabled = false, type = 'button', onclick, children, ...rest }: Props = $props();
</script>

{#snippet face()}
  {#if icon}<Icon name={icon} />{/if}
  {@render children?.()}
  {#if kbd}<span class="hint" data-kbd-inherit><Kbd keys={kbd} /></span>{/if}
{/snippet}

{#if tooltip}
  <Tooltip label={tooltip.label} shortcut={tooltip.shortcut}>
    {#snippet children(props)}
      <button {...mergeProps(props, rest)} class="btn" data-variant={variant} {type} {disabled} {onclick}>{@render face()}</button>
    {/snippet}
  </Tooltip>
{:else}
  <button {...rest} class="btn" data-variant={variant} {type} {disabled} {onclick}>{@render face()}</button>
{/if}

<style>
  .btn {
    display: inline-flex;
    align-items: center;
    gap: var(--s-4);
    height: var(--ctl-h);
    padding: 0 var(--s-6);
    border: 1px solid transparent;
    border-radius: var(--r-ctl);
    font-weight: 600;
    white-space: nowrap;
    transition: background var(--dur-1) var(--ease), border-color var(--dur-1) var(--ease);
  }
  .btn:disabled { opacity: 1; background: var(--active); color: var(--muted); border-color: transparent; cursor: default; }
  [data-variant='primary'] { background: var(--accent); color: var(--accent-ink); }
  [data-variant='primary']:hover:not(:disabled) { background: color-mix(in srgb, var(--accent) 86%, var(--ink)); }
  [data-variant='quiet'] { background: transparent; color: var(--ink); border-color: var(--control); font-weight: 500; }
  [data-variant='quiet']:hover:not(:disabled) { background: var(--active); }
  [data-variant='danger'] { background: var(--signal); color: var(--signal-ink); }
  [data-variant='danger']:hover:not(:disabled) { background: color-mix(in srgb, var(--signal) 86%, var(--ink)); }
  .btn:active:not(:disabled) { transform: translateY(1px); }
  .hint { display: inline-flex; }
</style>
