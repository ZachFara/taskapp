<script lang="ts">
  import type { Snippet } from 'svelte';
  import EntityIcon from './EntityIcon.svelte';

  let {
    title,
    icon,
    color,
    tabs,
    filters,
    actions,
  }: { title: string; icon?: string; color?: string; tabs?: Snippet; filters?: Snippet; actions?: Snippet } = $props();
</script>

<header class="page-header">
  <div class="top">
    {#if icon}<EntityIcon {icon} {color} />{/if}
    <h1>{title}</h1>
    {#if tabs}<div class="tabs">{@render tabs()}</div>{/if}
    <div class="grow"></div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
  {#if filters}<div class="filters">{@render filters()}</div>{/if}
</header>

<style>
  .page-header { flex: none; }
  .top { height: var(--topbar-h); display: flex; align-items: center; gap: var(--s-8); padding: 0 var(--s-7); }
  h1 { font-size: var(--fs-title); font-weight: 700; letter-spacing: -0.02em; line-height: var(--lh-tight); }
  .tabs { align-self: stretch; display: flex; }
  .grow { flex: 1; }
  .actions { display: flex; align-items: center; gap: var(--s-4); }
  .filters { height: 36px; display: flex; align-items: center; gap: var(--s-4); padding: 0 var(--s-7); }
</style>
