<script lang="ts">
  let { done, total }: { done: number; total: number } = $props();
</script>

<span class="bar" role="img" aria-label={`${done} of ${total} issues done`}><span style:width={total ? `${(done / total) * 100}%` : '0%'}></span></span>
<span class="fig">{done}/{total}</span>

<style>
  .bar { flex: none; width: 120px; height: 4px; border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); }
  .fig { font-size: var(--fs-sm); color: var(--muted); font-variant-numeric: tabular-nums; }
</style>
