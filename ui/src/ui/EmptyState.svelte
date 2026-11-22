<script lang="ts">
  import type { Snippet } from 'svelte';
  import Contour from './Contour.svelte';

  let { title, hint, action }: { title: string; hint?: string; action?: Snippet } = $props();
</script>

<div class="empty">
  <div class="art"><Contour /></div>
  <h2>{title}</h2>
  {#if hint}<p>{hint}</p>{/if}
  {#if action}<div class="act">{@render action()}</div>{/if}
</div>

<style>
  .empty { display: grid; justify-items: center; text-align: center; gap: var(--s-4); padding: var(--s-9) var(--s-7); }
  .art { color: var(--control); margin-bottom: var(--s-4); }
  h2 { font-size: var(--fs-lg); font-weight: 700; letter-spacing: -0.01em; }
  p { color: var(--muted); max-width: 42ch; }
  .act { margin-top: var(--s-4); }
</style>
