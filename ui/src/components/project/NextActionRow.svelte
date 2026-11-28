<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, Project } from '../../lib/bindings';
  import { isOn } from '../../lib/features.svelte';
  import OptionList from '../detail/OptionList.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PropertyRow from '../detail/PropertyRow.svelte';

  // The project's flagged next action: pick an open issue of the project, or clear the flag.
  let { project, openRow = $bindable(null) }: { project: Project; openRow?: string | null } = $props();
  let current = $state<Issue | null>(null);
  let stuck = $state(false);
  let open = $state<Issue[]>([]);

  async function load() {
    const [all, rows, statuses] = await Promise.all([
      api.next.list(),
      api.views.query({ filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: project.id } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }),
      api.statuses.list(),
    ]);
    const mine = all.find((r) => r.project.id === project.id);
    [current, stuck] = [mine?.issue ?? null, mine?.stuck ?? false];
    open = rows.filter((i) => !['Completed', 'Canceled'].includes(statuses.find((s) => s.id === i.status_id)?.category ?? ''));
  }
  $effect(() => {
    void project.id;
    void load();
    for (const e of ['rows:changed', 'history:wrote']) addEventListener(e, load);
    return () => ['rows:changed', 'history:wrote'].forEach((e) => removeEventListener(e, load));
  });
  const pick = async (id: string | number | null) => {
    openRow = null;
    await api.projects.setNext(project.id, id === null ? null : Number(id));
    await load();
  };
</script>

{#if isOn('next_actions')}
  <PropertyRow id="next" name="Next action" cmd="project.next" bind:openRow>
    {#snippet value()}
      {#if current}{current.key} {current.title}{:else}<span class="none">None</span>{#if stuck}<Pill color="red">Stuck</Pill>{/if}{/if}
    {/snippet}
    {#snippet editor()}
      <OptionList label="Next action" value={current?.id ?? null} options={open.map((i) => ({ id: i.id, name: `${i.key} ${i.title}` }))} none="No next action" onpick={pick} />
    {/snippet}
  </PropertyRow>
{/if}

<style>
  .none { color: var(--muted); font-weight: 400; }
</style>
