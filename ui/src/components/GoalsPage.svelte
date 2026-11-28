<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Goal, Project } from '../lib/bindings';
  import { byQuarter, STATUS_LABEL } from '../lib/goals';
  import { labelFor, shortcutFor } from '../lib/commands/registry';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import GoalBar from './goals/GoalBar.svelte';
  import GoalDialog from './goals/GoalDialog.svelte';

  let goals = $state<Goal[]>([]);
  let projects = $state<Project[]>([]);
  let progress = $state<Record<number, [number, number]>>({});
  let loaded = $state(false);
  let creating = $state(false);

  async function load() {
    [goals, projects] = await Promise.all([api.goals.list(), api.projects.list()]);
    progress = Object.fromEntries(await Promise.all(goals.map(async (g) => [g.id, await api.goals.progress(g.id)] as const)));
    loaded = true;
  }
  async function create(v: { title: string; quarter: string; description: string }) {
    const g = await api.goals.create(v.title, v.quarter, v.description || null);
    navigate(`/goal/${g.id}`);
  }
  const names = (g: Goal) => g.project_ids.map((id) => projects.find((p) => p.id === id)?.name).filter(Boolean).join(', ');

  $effect(() => {
    void load();
    const on: [string, () => void][] = [['command:goal.new', () => (creating = true)], ['command:view.refresh', () => void load()]];
    on.forEach(([n, f]) => addEventListener(n, f));
    return () => on.forEach(([n, f]) => removeEventListener(n, f));
  });
</script>

<PageHeader title="Goals" icon="icon:target">
  {#snippet actions()}
    <Button icon="plus" tooltip={{ label: labelFor('goal.new') ?? 'New goal', shortcut: shortcutFor('goal.new') }} onclick={() => (creating = true)}>New goal</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if loaded && goals.length === 0}
    <EmptyState title="No goals yet" hint="A goal is a few outcomes for a quarter. Link projects to it and its progress follows their issues." />
  {/if}
  {#each byQuarter(goals) as [quarter, list] (quarter)}
    <section aria-label={quarter}>
      <h2>{quarter}</h2>
      <ul>
        {#each list as g (g.id)}
          <li>
            <a class="row" href={`/goal/${g.id}`} data-goal={g.id} onclick={(e) => (e.preventDefault(), navigate(`/goal/${g.id}`))}>
              <span class="title" data-status={g.status}>{g.title}</span>
              {#if g.status !== 'active'}<Pill color={g.status === 'achieved' ? 'green' : 'gray'}>{STATUS_LABEL[g.status]}</Pill>{/if}
              <span class="projects">{names(g)}</span>
              {#if progress[g.id]}<GoalBar done={progress[g.id][0]} total={progress[g.id][1]} />{/if}
            </a>
          </li>
        {/each}
      </ul>
    </section>
  {/each}
</main>

<GoalDialog bind:open={creating} onsave={create} />

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-7); max-width: 900px; }
  h2 { margin: 0 0 var(--s-3); font-size: var(--fs-sm); font-weight: 700; letter-spacing: 0.08em; color: var(--muted); }
  ul { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  li { border-bottom: 1px solid var(--border); }
  .row { display: flex; align-items: center; gap: var(--s-6); min-height: var(--row-h); padding: var(--s-2) var(--s-3); color: var(--ink); text-decoration: none; }
  .row:hover { background: var(--active); }
  .title { font-weight: 600; flex: none; }
  .title[data-status='dropped'] { color: var(--muted); text-decoration: line-through; }
  .projects { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); font-size: var(--fs-sm); }
</style>
