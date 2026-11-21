<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import type { Label } from '../../lib/bindings';

  let labels = $state<Label[]>([]);
  let name = $state('');
  let color = $state('#888888');
  let error = $state('');

  const load = async () => (labels = await api.labels.list());
  onMount(load);

  async function add() {
    error = '';
    try {
      await api.labels.create(name.trim(), color);
      name = '';
      await load();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>Labels</h2>
<ul>
  {#each labels as l (l.id)}
    <li><span style:color={l.color}>●</span> {l.name}</li>
  {/each}
</ul>

<form onsubmit={(e) => { e.preventDefault(); void add(); }}>
  <input aria-label="Label name" placeholder="Name" bind:value={name} />
  <input aria-label="Label color" type="color" bind:value={color} />
  <button type="submit">Add label</button>
</form>
{#if error}<p role="alert">{error}</p>{/if}
