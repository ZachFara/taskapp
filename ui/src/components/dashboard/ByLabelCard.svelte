<script lang="ts">
  import RankBars from './RankBars.svelte';
  import type { CardProps } from './cards';

  // The top five labels by completed (or points), the rest together as Other.
  let { metrics: m, measure }: CardProps = $props();
  const rows = $derived.by(() => {
    const all = m.by_label.map((l) => ({ label: l.name, value: measure === 'Points' ? l.points : l.completed })).sort((a, b) => b.value - a.value);
    const rest = all.slice(5).reduce((n, r) => n + r.value, 0);
    return rest > 0 ? [...all.slice(0, 5), { label: 'Other', value: rest }] : all.slice(0, 5);
  });
</script>

<RankBars {rows} label={`Completed per label, ${measure === 'Points' ? 'points' : 'issues'}`} />
