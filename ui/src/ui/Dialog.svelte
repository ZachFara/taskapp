<script lang="ts">
  import { Dialog } from 'bits-ui';
  import type { Snippet } from 'svelte';
  import Button from './Button.svelte';

  let {
    open = $bindable(false),
    title,
    description,
    children,
    actions,
  }: { open?: boolean; title: string; description?: string; children?: Snippet; actions?: Snippet } = $props();
</script>

<Dialog.Root bind:open>
  <Dialog.Portal>
    <Dialog.Overlay class="cx-overlay" />
    <Dialog.Content class="cx-dialog">
      <Dialog.Title class="cx-dialog-title">{title}</Dialog.Title>
      {#if description}<Dialog.Description class="cx-dialog-desc">{description}</Dialog.Description>{/if}
      {#if children}<div class="body">{@render children()}</div>{/if}
      <div class="foot">
        {#if actions}{@render actions()}{:else}<Dialog.Close>{#snippet child({ props })}<Button variant="quiet" {...props}>Close</Button>{/snippet}</Dialog.Close>{/if}
      </div>
    </Dialog.Content>
  </Dialog.Portal>
</Dialog.Root>

<style>
  :global(.cx-overlay) { position: fixed; inset: 0; z-index: var(--z-dialog); background: var(--overlay); animation: cx-fade var(--dur-2) var(--ease); }
  :global(.cx-dialog) {
    position: fixed;
    z-index: var(--z-dialog);
    top: 18%;
    left: 50%;
    transform: translateX(-50%);
    width: min(480px, calc(100% - var(--s-9)));
    padding: var(--s-7);
    display: grid;
    gap: var(--s-4);
    background: var(--sheet);
    color: var(--ink);
    border: 1px solid var(--control);
    border-radius: var(--r-pop);
    box-shadow: var(--shadow-float);
    animation: cx-fade var(--dur-2) var(--ease);
  }
  :global(.cx-dialog-title) { font-size: var(--fs-lg); font-weight: 700; letter-spacing: -0.01em; }
  :global(.cx-dialog-desc) { color: var(--muted); }
  .body { margin-top: var(--s-2); }
  .foot { display: flex; justify-content: flex-end; gap: var(--s-4); margin-top: var(--s-4); }
  @keyframes cx-fade { from { opacity: 0; } }
</style>
