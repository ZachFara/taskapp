<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Issue } from '../lib/bindings';
  import { archivedDef } from '../lib/deleteIssues.svelte';
  import { selection } from '../lib/selection.svelte';
  import CommandButton from './CommandButton.svelte';
  import { dateOf, humanDate } from './detail/format';
  import Checkbox from '../ui/Checkbox.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import TextInput from '../ui/TextInput.svelte';

  let rows = $state<Issue[] | null>(null);
  let q = $state('');
  const load = async () => (rows = (await api.views.query(archivedDef)) as Issue[]);
  const refresh = () => void load();
  onMount(() => {
    selection.clear();
    void load();
    for (const e of ['command:view.refresh', 'history:wrote']) addEventListener(e, refresh);
    return () => {
      ['command:view.refresh', 'history:wrote'].forEach((e) => removeEventListener(e, refresh));
      selection.clear();
    };
  });
  const shown = $derived(
    (rows ?? []).filter((i) => `${i.key} ${i.title}`.toLowerCase().includes(q.trim().toLowerCase())).sort((a, b) => (b.archived_at ?? '').localeCompare(a.archived_at ?? '')),
  );
  const all = $derived(shown.length > 0 && shown.every((i) => selection.selected.includes(i.id)));
  const open = (e: MouseEvent, key: string) => (e.preventDefault(), navigate(`/issue/${key}`));
</script>

<PageHeader title="Archive" icon="icon:archive">
  {#snippet actions()}
    <CommandButton command="archive.restore" />
    <CommandButton command="archive.delete" variant="danger" />
    <CommandButton command="archive.empty" variant="danger" />
  {/snippet}
</PageHeader>

<main class="body">
  {#if rows === null}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0}
    <EmptyState title="The archive is empty" hint="Archived issues are listed here, where you can restore them or delete them for good." />
  {:else}
    <TextInput label="Search the archive" placeholder="Search the archive" bind:value={q} />
    <ul class="list">
      <li class="head">
        <Checkbox label="Select all" checked={all} onchange={(on) => (on ? selection.selectAll(shown.map((i) => i.id)) : selection.clear())} />
        <span>{shown.length} archived</span>
      </li>
      {#each shown as i (i.id)}
        <li class="row" data-id={i.id}>
          <Checkbox label={`Select ${i.key}`} checked={selection.selected.includes(i.id)} onchange={() => selection.toggle(i.id)} />
          <a class="key" href={`/issue/${i.key}`} onclick={(e) => open(e, i.key)}>{i.key}</a>
          <span class="title">{i.title}</span>
          <span class="when">{i.archived_at ? humanDate(dateOf(i.archived_at)) : ''}</span>
        </li>
      {:else}
        <li class="none">No archived issue matches.</li>
      {/each}
    </ul>
  {/if}
</main>

<style>
  .body { display: grid; gap: var(--s-4); align-content: start; padding: 0 var(--s-7) var(--s-7); }
  .list { list-style: none; margin: 0; padding: 0; }
  li { display: flex; align-items: center; gap: var(--s-4); height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  .head { font-size: var(--fs-xs); font-weight: 600; color: var(--muted); }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; text-decoration: none; }
  .title { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { color: var(--muted); font-size: var(--fs-xs); }
  .none { color: var(--muted); }
</style>
