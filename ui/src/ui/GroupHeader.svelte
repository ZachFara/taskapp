<script lang="ts">
  import type { StatusCategory } from '../lib/bindings';
  import Icon from './Icon.svelte';
  import type { Snippet } from 'svelte';
  import StatusGlyph from './StatusGlyph.svelte';

  // The legend band. The tone step follows the status category, so the list reads like map elevation:
  // the further along, the deeper the band. A group that is not a status (project, priority, cycle, label)
  // passes `tone` 1-4 and, if it likes, a `leading` snippet such as an EntityIcon.
  let {
    name,
    count,
    total = '',
    category,
    tone = 1,
    leading,
    collapsed = false,
    ontoggle,
    onadd,
  }: { name: string; count: number; total?: string; category?: StatusCategory; tone?: 1 | 2 | 3 | 4; leading?: Snippet; collapsed?: boolean; ontoggle?: () => void; onadd?: () => void } = $props();
  const steps = { Backlog: 1, Unstarted: 2, Started: 3, Completed: 4, Canceled: 1 } as const;
  const step = $derived(category ? steps[category] : tone);
</script>

<div class="band" style:--tone={`var(--band-${step})`} data-step={step}>
  <button class="toggle" type="button" aria-expanded={!collapsed} aria-label={`${collapsed ? 'Expand' : 'Collapse'} ${name}`} onclick={ontoggle}>
    <Icon name={collapsed ? 'chevron-right' : 'chevron-down'} size={12} />
    {#if category}<StatusGlyph {category} />{:else}{@render leading?.()}{/if}
    <span class="name">{name}</span>
    <span class="count">{count}</span>
    {#if total}<span class="count">{total}</span>{/if}
  </button>
  <button class="add" type="button" aria-label={`Add to ${name}`} onclick={onadd}><Icon name="plus" /></button>
</div>

<style>
  .band {
    height: var(--band-h);
    display: flex;
    align-items: center;
    gap: var(--s-5);
    padding: 0 var(--s-7) 0 var(--s-4);
    background: var(--tone);
    color: var(--band-ink);
  }
  .toggle, .add { border: 0; background: transparent; color: inherit; padding: 0; }
  .toggle { flex: 1; display: flex; align-items: center; gap: var(--s-5); height: 100%; text-align: left; font-weight: 700; letter-spacing: -0.005em; }
  .toggle :global(.icon) { color: var(--band-count); }
  .count { font-weight: 600; color: var(--band-count); }
  .add { display: flex; color: var(--band-count); border-radius: var(--r-ctl); }
  .add:hover { color: var(--band-ink); }
</style>
