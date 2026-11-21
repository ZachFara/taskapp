<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue } from '../../lib/bindings';

  let { selfId, label, pick }: { selfId: number; label: string; pick: (i: Issue) => void } = $props();
  let results = $state<Issue[]>([]);

  async function find(text: string) {
    // An issue is never offered as its own sub-issue or relation.
    results = text.trim() ? (await api.search.search(text, 8)).filter((i) => i.id !== selfId) : [];
  }
</script>

<input aria-label={label} placeholder={label} oninput={(e) => find(e.currentTarget.value)} />
<ul class="results">
  {#each results as r (r.id)}
    <li><button onclick={() => { results = []; pick(r); }}>{r.key} {r.title}</button></li>
  {/each}
</ul>
