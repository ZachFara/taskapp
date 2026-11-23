<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue } from '../../lib/bindings';
  import TextInput from '../../ui/TextInput.svelte';

  // With `create`, Enter makes a new issue from the typed text; the matches below still attach an existing one.
  let { selfId, label, pick, create, placeholder = label }: { selfId: number; label: string; pick: (i: Issue) => void; create?: (title: string) => void; placeholder?: string } = $props();
  let results = $state<Issue[]>([]);
  let text = $state('');

  async function find(q: string) {
    // An issue is never offered as its own sub-issue, parent or relation.
    results = q.trim() ? (await api.search.search(q, 8)).filter((i) => i.id !== selfId) : [];
  }
</script>

<div class="picker">
  <form onsubmit={(e) => { e.preventDefault(); const t = text.trim(); if (create && t) { text = ''; results = []; create(t); } }}>
    <TextInput {label} {placeholder} type={create ? 'text' : 'search'} bind:value={text} oninput={() => find(text)} />
  </form>
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
