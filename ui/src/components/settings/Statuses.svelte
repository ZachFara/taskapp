<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { statuses, refreshAll } from '../../lib/stores.svelte';
  import type { StatusCategory } from '../../lib/bindings';

  const categories: StatusCategory[] = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'];
  let name = $state('');
  let category = $state<StatusCategory>('Unstarted');
  let color = $state('#888888');
  let error = $state('');

  onMount(() => refreshAll());

  async function add() {
    error = '';
    try {
      await api.statuses.create(name.trim(), category, color);
      name = '';
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>Statuses</h2>
<ul>
  {#each statuses as s (s.id)}
    <li><span style:color={s.color}>●</span> {s.name} <small>{s.category}</small></li>
  {/each}
</ul>

<form onsubmit={(e) => { e.preventDefault(); void add(); }}>
  <input aria-label="New status name" placeholder="Name" bind:value={name} />
  <select aria-label="New status category" bind:value={category}>
    {#each categories as c (c)}<option value={c}>{c}</option>{/each}
  </select>
  <input aria-label="New status color" type="color" bind:value={color} />
  <button type="submit">Add status</button>
</form>
{#if error}<p role="alert">{error}</p>{/if}
