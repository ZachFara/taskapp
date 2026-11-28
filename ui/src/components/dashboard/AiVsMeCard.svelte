<script lang="ts">
  import RankBars from './RankBars.svelte';
  import type { CardProps } from './cards';

  // Change sources in plain words. Counts of issues whatever the measure: the engine gives created and completed as counts.
  const GROUP: Record<string, string> = { Ui: 'You, in the app', Cli: 'AI and tools', Agent: 'AI and tools', Migration: 'Imported' };
  const ORDER = ['You, in the app', 'AI and tools', 'Imported', 'Other'];
  let { metrics: m }: CardProps = $props();
  const groups = $derived(
    ORDER.map((name) => {
      const xs = m.by_source.filter((s) => (GROUP[s.source] ?? 'Other') === name);
      return { name, created: xs.reduce((n, s) => n + s.created, 0), completed: xs.reduce((n, s) => n + s.completed, 0) };
    }),
  );
</script>

<h3>Created</h3>
<RankBars rows={groups.map((g) => ({ label: g.name, value: g.created }))} label="Created, by who made the change" />
<h3>Completed</h3>
<RankBars rows={groups.map((g) => ({ label: g.name, value: g.completed }))} label="Completed, by who made the change" />

<style>
  h3 { margin: 0 0 var(--s-3); font-size: var(--fs-sm); color: var(--muted); font-weight: 600; }
  h3:not(:first-child) { margin-top: var(--s-5); }
</style>
