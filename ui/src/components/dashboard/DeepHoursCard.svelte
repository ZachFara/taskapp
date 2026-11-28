<script lang="ts">
  import type { CardProps } from './cards';

  let { metrics }: CardProps = $props();
  const deep = $derived(metrics?.deep ?? null);
  const total = $derived(deep?.total_hours ?? 0);
  const peak = $derived(Math.max(0.5, ...(deep?.buckets.map((b) => b.hours ?? 0) ?? [])));
  const hours = (h: number) => `${Math.round(h * 10) / 10} h`;
</script>

{#if !deep || total === 0}
  <p class="quiet">No deep focus in this range. Start a focus session on an issue marked deep: Depth in its details, then its Focus button.</p>
{:else}
  <p class="count">{hours(total)} deep, {deep.streak_days} {deep.streak_days === 1 ? 'day' : 'days'} in a row</p>
  <ol class="buckets" aria-label="Deep hours per {metrics.bucket.toLowerCase()}">
    {#each deep.buckets as b (b.start)}
      <li><span class="when">{b.start}</span><span class="track"><span class="bar" style:width={`${((b.hours ?? 0) / peak) * 100}%`}></span></span><span class="n">{hours(b.hours ?? 0)}</span></li>
    {/each}
  </ol>
{/if}

<style>
  .quiet, .count { margin: 0 0 var(--s-4); color: var(--muted); font-size: var(--fs-sm); }
  .buckets { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-2); font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
  .buckets li { display: grid; grid-template-columns: auto 1fr auto; gap: var(--s-3); align-items: center; }
  .when, .n { color: var(--muted); white-space: nowrap; }
  .track { height: var(--s-2); background: var(--border); border-radius: var(--s-1); overflow: hidden; }
  .bar { display: block; height: 100%; background: var(--signal); }
</style>
