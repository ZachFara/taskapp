<script lang="ts">
  // The "View as table" alternative to every chart: the same numbers, readable by a screen reader.
  let { caption, head, rows }: { caption: string; head: string[]; rows: (string | number)[][] } = $props();
</script>

<table>
  <caption>{caption}</caption>
  <thead><tr>{#each head as h, i (h)}<th scope="col" class:num={i > 0}>{h}</th>{/each}</tr></thead>
  <tbody>
    {#each rows as row (row[0])}
      <tr>{#each row as cell, i (i)}<td class:num={i > 0}>{cell}</td>{/each}</tr>
    {/each}
  </tbody>
</table>

<style>
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-sm); font-variant-numeric: tabular-nums; }
  caption { text-align: left; color: var(--muted); padding-bottom: var(--s-3); }
  th { color: var(--muted); font-weight: 600; text-align: left; padding: var(--s-2) var(--s-4); border-bottom: 1px solid var(--control); }
  td { padding: var(--s-2) var(--s-4); border-bottom: 1px solid var(--border); }
  .num { text-align: right; }
</style>
