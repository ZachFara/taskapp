<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, Milestone, Status } from '../../lib/bindings';
  import Button from '../../ui/Button.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { humanDate } from '../detail/format';

  let { projectId, milestones, issues, statuses, onchange }: {
    projectId: number;
    milestones: Milestone[];
    issues: Issue[];
    statuses: Status[];
    onchange: () => void;
  } = $props();

  // editing: a milestone id, 0 for a new one, null for none.
  let editing = $state<number | null>(null);
  let name = $state('');
  let date = $state('');

  const doneIds = $derived(new Set(statuses.filter((s) => s.category === 'Completed').map((s) => s.id)));
  const count = (m: Milestone) => {
    const mine = issues.filter((i) => i.milestone_id === m.id);
    return { done: mine.filter((i) => doneIds.has(i.status_id)).length, total: mine.length };
  };

  function start(m?: Milestone) {
    editing = m?.id ?? 0;
    name = m?.name ?? '';
    date = m?.target_date ?? '';
  }
  $effect(() => {
    const go = () => start();
    addEventListener('command:milestone.new', go);
    return () => removeEventListener('command:milestone.new', go);
  });
  async function save(e: Event) {
    e.preventDefault();
    const clean = name.trim();
    if (!clean) return;
    const target = date || null;
    if (editing) await api.projects.updateMilestone({ id: editing, project_id: projectId, name: clean, target_date: target });
    else await api.projects.createMilestone(projectId, clean, target);
    editing = null;
    onchange();
  }
  async function remove(m: Milestone) {
    await api.projects.deleteMilestone(m.id);
    onchange();
  }
</script>

<section class="ms" aria-label="Milestones">
  <div class="head">
    <h2>Milestones</h2>
    <IconButton icon="plus" label="Add milestone" onclick={() => start()} />
  </div>
  <ol>
    {#each milestones as m (m.id)}
      {@const c = count(m)}
      <li data-milestone={m.id}>
        {#if editing === m.id}
          {@render form()}
        {:else}
          <span class="stone" aria-hidden="true"></span>
          <div class="text">
            <span class="name">{m.name}</span>
            {#if m.target_date}<time>{humanDate(m.target_date)}</time>{/if}
          </div>
          <div class="bar" role="img" aria-label={`${c.done} of ${c.total} done`}>
            <span style:width={c.total ? `${(c.done / c.total) * 100}%` : '0%'}></span>
          </div>
          <span class="fig">{c.done}/{c.total}</span>
          <span class="acts">
            <IconButton icon="edit" label={`Edit ${m.name}`} onclick={() => start(m)} />
            <IconButton icon="trash" label={`Delete ${m.name}`} onclick={() => remove(m)} />
          </span>
        {/if}
      </li>
    {/each}
    {#if editing === 0}<li>{@render form()}</li>{/if}
  </ol>
  {#if !milestones.length && editing === null}
    <p class="none">No milestones yet. A milestone is a dated stone on the way to the target.</p>
  {/if}
</section>

{#snippet form()}
  <form onsubmit={save}>
    <TextInput label="Milestone name" bind:value={name} placeholder="Milestone name" />
    <TextInput label="Milestone date" type="date" bind:value={date} />
    <Button variant="primary" type="submit" tooltip={{ label: 'Save milestone' }}>Save</Button>
    <Button onclick={() => (editing = null)}>Cancel</Button>
  </form>
{/snippet}

<style>
  .ms { display: grid; gap: var(--s-4); }
  .head { display: flex; align-items: center; gap: var(--s-4); }
  h2 { font-size: var(--fs-sm); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  ol { list-style: none; margin: 0; padding: 0; display: grid; border-top: 1px solid var(--border); }
  li { display: flex; align-items: center; gap: var(--s-6); min-height: var(--row-h); padding: var(--s-2) 0; border-bottom: 1px solid var(--border); }
  .stone { flex: none; width: 14px; height: 8px; border-radius: var(--r-round); background: var(--stone); }
  .text { flex: 1; min-width: 0; display: flex; align-items: baseline; gap: var(--s-6); }
  .name { font-weight: 600; }
  time, .fig { font-size: var(--fs-sm); color: var(--muted); font-variant-numeric: tabular-nums; }
  .bar { flex: none; width: 96px; height: 4px; border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .bar span { display: block; height: 100%; background: var(--ink); }
  .acts { display: flex; opacity: 0; transition: opacity var(--dur-1) var(--ease); }
  li:hover .acts, li:focus-within .acts { opacity: 1; }
  form { display: flex; align-items: center; gap: var(--s-4); width: 100%; }
  .none { color: var(--muted); font-size: var(--fs-sm); }
</style>
