<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, Relation, RelationKind } from '../../lib/bindings';
  import IssuePicker from './IssuePicker.svelte';

  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  type Row = { rel: Relation; label: string; other: Issue };
  let rows = $state<Row[]>([]);
  let choice = $state<'blocks' | 'blocked_by' | 'related' | 'duplicate'>('blocks');

  const labelOf = (r: Relation) =>
    r.kind === 'Blocks' ? (r.from === issue.id ? 'blocks' : 'blocked by') : r.kind === 'Related' ? 'related' : 'duplicate';

  $effect(() => {
    rev;
    api.relations.forIssue(issue.id).then(async (rels) => {
      rows = await Promise.all(
        rels.map(async (rel) => ({ rel, label: labelOf(rel), other: await api.issues.get(rel.from === issue.id ? rel.to : rel.from) })),
      );
    });
  });

  function add(other: Issue) {
    const kind: RelationKind = choice === 'related' ? 'Related' : choice === 'duplicate' ? 'Duplicate' : 'Blocks';
    const [from, to] = choice === 'blocked_by' ? [other.id, issue.id] : [issue.id, other.id];
    act(api.relations.add({ from, to, kind }));
  }
</script>

<section aria-label="Relations">
  <h3>Relations</h3>
  <ul>
    {#each rows as r (`${r.rel.from}-${r.rel.to}-${r.rel.kind}`)}
      <li>{r.label} {r.other.key} {r.other.title}
        <button aria-label="Remove relation to {r.other.key}" onclick={() => act(api.relations.remove(r.rel))}>x</button>
      </li>
    {/each}
  </ul>
  <select aria-label="Relation kind" bind:value={choice}>
    <option value="blocks">blocks</option>
    <option value="blocked_by">blocked by</option>
    <option value="related">related</option>
    <option value="duplicate">duplicate</option>
  </select>
  <IssuePicker selfId={issue.id} label="Find issue" pick={add} />
</section>
