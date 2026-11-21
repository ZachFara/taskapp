<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Cycle } from '../../lib/bindings';

  let cycles = $state<Cycle[]>([]);
  let name = $state('');
  let start = $state('');
  let end = $state('');
  let error = $state('');

  const load = async () => (cycles = await api.cycles.list());
  onMount(load);

  async function add() {
    error = '';
    try {
      await api.cycles.create(name.trim() || null, start, end);
      name = start = end = '';
      await load();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>Cycles</h2>
<ul>
  {#each cycles as c (c.id)}
    <li>#{c.number} {c.name ?? ''} {c.start_date} to {c.end_date}</li>
  {/each}
</ul>

<form onsubmit={(e) => { e.preventDefault(); void add(); }}>
  <input aria-label="Cycle name" placeholder="Name (optional)" bind:value={name} />
  <input aria-label="Cycle start" type="date" bind:value={start} />
  <input aria-label="Cycle end" type="date" bind:value={end} />
  <button type="submit">Add cycle</button>
</form>
{#if error}<p role="alert">{error}</p>{/if}
