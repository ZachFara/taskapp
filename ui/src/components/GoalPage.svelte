<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Goal, Project } from '../lib/bindings';
  import { STATUS_LABEL } from '../lib/goals';
  import { labelFor, shortcutFor } from '../lib/commands/registry';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Select from '../ui/Select.svelte';
  import GoalBar from './goals/GoalBar.svelte';
  import GoalDialog from './goals/GoalDialog.svelte';

  let { params }: { params: { id?: string } } = $props();
  const id = $derived(Number(params.id));

  // undefined while loading, null when there is no such goal.
  let goal = $state<Goal | null | undefined>(undefined);
  let projects = $state<Project[]>([]);
  let progress = $state<Record<number, [number, number]>>({});
  let editing = $state(false);
  let confirming = $state(false);
  let pick = $state('');
  let box = $state<HTMLElement>();

  const linked = $derived(goal ? projects.filter((p) => goal!.project_ids.includes(p.id)) : []);
  const free = $derived(goal ? projects.filter((p) => !goal!.project_ids.includes(p.id) && !p.archived_at) : []);
  const total = $derived(linked.reduce<[number, number]>((a, p) => [a[0] + (progress[p.id]?.[0] ?? 0), a[1] + (progress[p.id]?.[1] ?? 0)], [0, 0]));

  async function load() {
    try {
      const [g, all] = await Promise.all([api.goals.list().then((l) => l.find((x) => x.id === id) ?? null), api.projects.list()]);
      projects = all;
      progress = Object.fromEntries(await Promise.all(all.map(async (p) => [p.id, await api.projects.progress(p.id)] as const)));
      goal = g;
    } catch {
      goal = null;
    }
  }
  const setStatus = async (status: Goal['status']) => goal && (await api.goals.update({ ...goal, status }), await load());
  const save = async (v: { title: string; quarter: string; description: string }) => {
    await api.goals.update({ ...goal!, ...v });
    await load();
  };
  async function link(project: string) {
    if (!project) return;
    await api.goals.link(id, Number(project));
    pick = '';
    await load();
  }
  const unlink = async (p: Project) => (await api.goals.unlink(id, p.id), await load());
  async function remove() {
    confirming = false;
    await api.goals.delete(id);
    navigate('/goals');
  }

  $effect(() => {
    id;
    void load();
  });
  $effect(() => {
    const on: [string, () => void][] = [
      ['command:goal.edit', () => (editing = true)],
      ['command:goal.achieve', () => void setStatus('achieved')],
      ['command:goal.drop', () => void setStatus('dropped')],
      ['command:goal.reopen', () => void setStatus('active')],
      ['command:goal.delete', () => (confirming = true)],
      ['command:goal.link', () => box?.querySelector<HTMLElement>('button')?.focus()],
      ['command:view.refresh', () => void load()],
    ];
    on.forEach(([n, f]) => addEventListener(n, f));
    return () => on.forEach(([n, f]) => removeEventListener(n, f));
  });
</script>

{#if goal === null}
  <EmptyState title="Goal not found" hint="It may have been deleted." />
{:else if goal}
  <PageHeader title={goal.title} icon="icon:target">
    {#snippet actions()}
      <Button variant="quiet" icon="edit" tooltip={{ label: labelFor('goal.edit') ?? 'Edit goal', shortcut: shortcutFor('goal.edit') }} onclick={() => (editing = true)}>Edit</Button>
      {#if goal!.status === 'active'}
        <Button icon="check" onclick={() => setStatus('achieved')}>Achieved</Button>
        <Button onclick={() => setStatus('dropped')}>Drop</Button>
      {:else}
        <Button onclick={() => setStatus('active')}>Reopen</Button>
      {/if}
      <Button variant="danger" icon="trash" onclick={() => (confirming = true)}>Delete</Button>
    {/snippet}
  </PageHeader>

  <main class="body">
    <div class="meta">
      <Pill>{goal.quarter}</Pill>
      <Pill color={goal.status === 'achieved' ? 'green' : goal.status === 'dropped' ? 'gray' : undefined}>{STATUS_LABEL[goal.status]}</Pill>
      <GoalBar done={total[0]} total={total[1]} />
    </div>
    {#if goal.description}<p class="desc">{goal.description}</p>{/if}

    <section aria-label="Projects">
      <h2>Projects</h2>
      <ul>
        {#each linked as p (p.id)}
          <li data-project={p.id}>
            <a class="name" href={`/project/${p.slug}`} onclick={(e) => (e.preventDefault(), navigate(`/project/${p.slug}`))}>{p.name}</a>
            <GoalBar done={progress[p.id]?.[0] ?? 0} total={progress[p.id]?.[1] ?? 0} />
            <Button onclick={() => unlink(p)}>Unlink</Button>
          </li>
        {/each}
      </ul>
      {#if !linked.length}<p class="none">No project works towards this goal yet.</p>{/if}
      <div class="add" bind:this={box}>
        <Select label="Link a project" placeholder="Link a project" bind:value={() => pick, (v) => void link(v)} items={free.map((p) => ({ value: String(p.id), label: p.name }))} />
      </div>
    </section>
  </main>

  <GoalDialog bind:open={editing} {goal} onsave={save} />
  <Dialog bind:open={confirming} title="Delete this goal?" description="Its projects stay as they are. Undo brings the goal back.">
    {#snippet actions()}
      <Button onclick={() => (confirming = false)}>Cancel</Button>
      <Button variant="danger" onclick={remove}>Delete</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-6); max-width: 760px; }
  .meta { display: flex; align-items: center; gap: var(--s-4); }
  .desc { margin: 0; color: var(--ink); line-height: var(--lh-body); }
  h2 { margin: 0 0 var(--s-3); font-size: var(--fs-sm); font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; color: var(--muted); }
  ul { list-style: none; margin: 0; padding: 0; border-top: 1px solid var(--border); }
  li { display: flex; align-items: center; gap: var(--s-6); min-height: var(--row-h); padding: var(--s-2) 0; border-bottom: 1px solid var(--border); }
  .name { flex: 1; font-weight: 600; color: var(--ink); text-decoration: none; }
  .name:hover { text-decoration: underline; }
  .add { margin-top: var(--s-4); max-width: 280px; }
  .none { color: var(--muted); font-size: var(--fs-sm); }
</style>
