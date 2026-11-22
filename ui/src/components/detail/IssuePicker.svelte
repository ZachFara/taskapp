<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue } from '../../lib/bindings';
  import TextInput from '../../ui/TextInput.svelte';

  let { selfId, label, pick }: { selfId: number; label: string; pick: (i: Issue) => void } = $props();
  let results = $state<Issue[]>([]);
  let text = $state('');

  async function find(q: string) {
    // An issue is never offered as its own sub-issue, parent or relation.
    results = q.trim() ? (await api.search.search(q, 8)).filter((i) => i.id !== selfId) : [];
  }
</script>

<div class="picker">
  <TextInput {label} placeholder={label} type="search" bind:value={text} oninput={() => find(text)} />
  {#if results.length}
    <ul class="results">
      {#each results as r (r.id)}
        <li><button type="button" onclick={() => { results = []; text = ''; pick(r); }}><span class="key">{r.key}</span> {r.title}</button></li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .picker { display: grid; gap: var(--s-2); min-width: 0; }
  .results { list-style: none; border: 1px solid var(--control); border-radius: var(--r-ctl); background: var(--sheet); overflow: hidden; }
  button { display: flex; gap: var(--s-4); width: 100%; height: var(--ctl-h); align-items: center; padding: 0 var(--s-4); border: 0; background: transparent; text-align: left; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  button:hover { background: var(--active); }
  .key { font-weight: 700; color: var(--muted); }
</style>
