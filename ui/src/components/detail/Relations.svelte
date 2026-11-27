<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, Relation, RelationKindDef } from '../../lib/bindings';
  import Collapse from '../../ui/Collapse.svelte';
  import SectionHeading from './SectionHeading.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import IssuePicker from './IssuePicker.svelte';

  let { issue, act, rev, open }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number; open: (key: string) => void } = $props();
  type Row = { rel: Relation; other: Issue };
  let rows = $state<Row[]>([]);
  // "<kind id>:f" reads from this issue, "<kind id>:r" from the other one.
  let choice = $state('blocks:f');
  let collapsed = $state(false);
  let kinds = $state<RelationKindDef[]>([]);

  const labelOf = (r: Relation) => {
    const k = kinds.find((x) => x.id === r.kind);
    return k ? (r.from === issue.id ? k.forward : k.reverse) : r.kind;
  };
  const items = $derived(kinds.flatMap((k) => [
    { value: `${k.id}:f`, label: k.forward },
    ...(k.symmetric ? [] : [{ value: `${k.id}:r`, label: k.reverse }]),
  ]));

  $effect(() => {
    rev;
    api.relation.kindsList().then((k) => (kinds = k));
  });
  $effect(() => {
    rev;
    api.relations.forIssue(issue.id).then(async (rels) => {
      rows = await Promise.all(
        rels.map(async (rel) => ({ rel, other: await api.issues.get(rel.from === issue.id ? rel.to : rel.from) })),
      );
    });
  });

  let sec = $state<HTMLElement>();
  function add(other: Issue) {
    const [kind, side] = choice.split(':');
    const [from, to] = side === 'r' ? [other.id, issue.id] : [issue.id, other.id];
    act(api.relations.add({ from, to, kind }));
  }
  // The command menu focuses the same control the section's button does.
  $effect(() => {
    const go = () => { collapsed = false; queueMicrotask(() => sec?.querySelector<HTMLElement>('.add input')?.focus()); };
    addEventListener('command:issue.relation.add', go);
    return () => removeEventListener('command:issue.relation.add', go);
  });
</script>


<section aria-label="Relations" bind:this={sec}>
  <SectionHeading name="Relations" count={rows.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  <Collapse open={!collapsed}>
    {#if !rows.length}<p class="none">No relations</p>{/if}
    <ul>
      {#each rows as r (`${r.rel.from}-${r.rel.to}-${r.rel.kind}`)}
        <li>
          <button type="button" class="go" onclick={() => open(r.other.key)}><em>{labelOf(r.rel)}</em> <b>{r.other.key}</b> {r.other.title}</button>
          <IconButton icon="close" label="Remove relation to {r.other.key}" onclick={() => act(api.relations.remove(r.rel))} />
        </li>
      {/each}
    </ul>
    <div class="add">
      <Select label="Relation kind" bind:value={choice} {items} />
      <IssuePicker selfId={issue.id} label="Find issue" pick={add} />
    </div>
  </Collapse>
</section>

<style>
  ul { list-style: none; }
  .none { padding: var(--s-4) var(--s-5); color: var(--muted); }
  li { display: flex; align-items: center; height: var(--row-h); padding-right: var(--s-4); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; }
  .go:hover { background: var(--active); }
  em { color: var(--muted); font-style: normal; font-size: var(--fs-xs); font-weight: 700; text-transform: uppercase; letter-spacing: 0.06em; min-width: 5.5rem; }
  .add { display: grid; grid-template-columns: 9rem 1fr; gap: var(--s-4); padding: var(--s-4); align-items: start; }
</style>
