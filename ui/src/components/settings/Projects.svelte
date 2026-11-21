<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { projects, refreshAll } from '../../lib/stores.svelte';

  let slug = $state('');
  let name = $state('');
  let error = $state('');

  onMount(() => refreshAll());

  async function add() {
    error = '';
    try {
      await api.projects.create(slug.trim(), name.trim());
      slug = name = '';
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
</script>

<h2>Projects</h2>
<ul>
  {#each projects as p (p.id)}
    <li>{p.name} <code>{p.slug}</code></li>
  {/each}
</ul>

<form onsubmit={(e) => { e.preventDefault(); void add(); }}>
  <input aria-label="Project slug" placeholder="slug" bind:value={slug} />
  <input aria-label="Project name" placeholder="Name" bind:value={name} />
  <button type="submit">Add project</button>
</form>
{#if error}<p role="alert">{error}</p>{/if}
