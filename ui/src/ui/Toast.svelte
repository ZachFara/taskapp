<script lang="ts">
  import type { Snippet } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { fly } from 'svelte/transition';
  import { ms } from '../lib/motion.svelte';
  import Icon from './Icon.svelte';

  // The toast surface. A queue and timers belong to whoever owns the messages; this only draws one.
  let {
    tone = 'info',
    children,
    action,
    ondismiss,
  }: { tone?: 'info' | 'success' | 'error'; children: Snippet; action?: Snippet; ondismiss?: () => void } = $props();
  const icon = { info: null, success: 'check', error: 'flag' } as const;
</script>

<div class="toast" in:fly|global={{ y: 12, duration: ms('--dur-3'), easing: cubicOut }} out:fly|global={{ y: 12, duration: ms('--dur-out') }} role={tone === 'error' ? 'alert' : 'status'} data-tone={tone}>
  {#if icon[tone]}<span class="lead"><Icon name={icon[tone]} /></span>{/if}
  <span class="msg">{@render children()}</span>
  {#if action}<span class="act">{@render action()}</span>{/if}
  {#if ondismiss}<button class="x" type="button" aria-label="Dismiss" onclick={ondismiss}><Icon name="close" size={12} /></button>{/if}
</div>

<style>
  .toast {
    display: flex;
    align-items: center;
    gap: var(--s-5);
    min-width: 260px;
    max-width: 420px;
    padding: var(--s-4) var(--s-6);
    background: var(--tip-bg);
    color: var(--tip-ink);
    border-radius: var(--r-pop);
    box-shadow: var(--shadow-float);
    font-weight: 500;
  }
  .lead { display: flex; }
  [data-tone='success'] .lead { color: var(--positive); }
  [data-tone='error'] .lead { color: var(--signal); }
  .msg { flex: 1; }
  .act { font-weight: 700; }
  .x { display: flex; border: 0; background: transparent; color: inherit; opacity: 0.7; padding: var(--s-2); border-radius: var(--r-sm); }
  .x:hover { opacity: 1; }
</style>
