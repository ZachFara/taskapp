<script lang="ts">
  import type { CardProps } from './cards';
  import { pct } from './metrics';

  let { metrics: m, measure }: CardProps = $props();
  const c = $derived(m.cycle);
  const pts = $derived(measure === 'Points');
  const n = (v: number) => String(Math.round(v * 10) / 10);
  const stats = $derived(
    c
      ? [
          { k: 'Say/do', v: pct(pts ? c.say_do_points_pct : c.say_do_pct) },
          { k: 'Committed', v: n(pts ? c.committed_points : c.committed) },
          { k: 'Done', v: n(pts ? c.done_points : c.done) },
          { k: 'Added later', v: n(pts ? c.added_points : c.added) },
          { k: 'Removed later', v: n(pts ? c.removed_points : c.removed) },
          { k: 'Scope change', v: pct(pts ? c.scope_change_points_pct : c.scope_change_pct) },
        ]
      : [],
  );
</script>

{#if !c}
  <p class="note">Choose a cycle in Range to see what it committed to and how much changed after it started.</p>
{:else}
  <dl class="stats">
    {#each stats as s (s.k)}<div><dt>{s.k}</dt><dd>{s.v}</dd></div>{/each}
  </dl>
  <p class="note">Committed is what the cycle held at the end of its first day; done counts those that finished by its end (or today). Scope change is added plus removed, as a share of committed.</p>
  {#if c.committed === 0}<p class="note">Nothing was in this cycle on its first day, so the percentages are blank.</p>{/if}
{/if}

<style>
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: var(--s-5); margin: 0 0 var(--s-5); }
  dt { color: var(--muted); font-size: var(--fs-sm); }
  dd { margin: 0; font-size: var(--fs-title); font-weight: 800; letter-spacing: -0.02em; font-variant-numeric: tabular-nums; }
  .note { margin: 0 0 var(--s-3); color: var(--muted); font-size: var(--fs-sm); }
</style>
