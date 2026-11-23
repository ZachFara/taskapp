<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../lib/api';
  import type { Project } from '../lib/bindings';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import EntityIcon from '../ui/EntityIcon.svelte';
  import { navigate } from '../app/routes';
  import { dayText } from './views/dates';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import TextInput from '../ui/TextInput.svelte';

  type Row = Project & { done: number; total: number };
  let rows = $state<Row[]>([]);
  let loaded = $state(false);
  let creating = $state(false);
  let name = $state('');

  $effect(() => {
    const open = () => (creating = true);
    addEventListener('command:project.new', open);
    return () => removeEventListener('command:project.new', open);
  });

  async function load() {
    const list = await api.projects.list();
    const progress = await Promise.all(list.map((p) => api.projects.progress(p.id)));
    rows = list.map((p, i) => ({ ...p, done: progress[i][0], total: progress[i][1] }));
    loaded = true;
  }

  const slugOf = (s: string) => s.trim().toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');

  async function create() {
    const n = name.trim();
    if (!n) return;
    await api.projects.create(slugOf(n), n, null, null);
    name = '';
    creating = false;
    await load();
  }

  onMount(load);
</script>

<PageHeader title="Projects" icon="icon:project">
  {#snippet actions()}
    <a class="timeline" href="/projects/timeline">All projects timeline</a>
    <Button variant="primary" icon="plus" onclick={() => (creating = true)}>New project</Button>
  {/snippet}
</PageHeader>

<main class="body">
  {#if !loaded}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0}
    <EmptyState title="No projects yet" hint="A project groups issues toward a target date.">
      {#snippet action()}<Button variant="primary" icon="plus" onclick={() => (creating = true)}>New project</Button>{/snippet}
    </EmptyState>
  {:else}
    <table class="table">
      <thead>
        <tr><th scope="col">Project</th><th scope="col">Progress</th><th scope="col">Target</th><th scope="col" class="num">Issues</th></tr>
      </thead>
      <tbody>
        {#each rows as p (p.id)}
          <tr data-slug={p.slug} onclick={() => navigate(`/project/${p.slug}`)}>
            <td><span class="name"><EntityIcon icon={p.icon} color={p.color} /> {p.name}</span></td>
            <td>
              <span class="prog"><span class="meter" aria-label={`${p.done} of ${p.total} done`}><span class="fill" style:width={`${p.total ? (p.done / p.total) * 100 : 0}%`}></span></span><span class="muted">{p.done} of {p.total}</span></span>
            </td>
            <td class="muted">{p.target_date ? dayText(p.target_date) : 'No target'}</td>
            <td class="num">{p.total}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

<Dialog bind:open={creating} title="New project">
  {#snippet children()}
    <TextInput bind:value={name} label="Project name" placeholder="Project name" />
  {/snippet}
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (creating = false)}>Cancel</Button>
    <Button variant="primary" onclick={create}>Create project</Button>
  {/snippet}
</Dialog>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); }
  .timeline {
    display: inline-flex; align-items: center; height: var(--ctl-h); padding: 0 var(--s-5);
    border: 1px solid var(--control); border-radius: var(--r-ctl); background: var(--sheet);
    color: var(--ink); font-weight: 500; text-decoration: none;
  }
  .timeline:hover { border-color: var(--muted); }
  .table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--fs-xs); font-weight: 600; color: var(--muted); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); }
  td { height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  tbody tr:hover { background: var(--active); }
  .name { display: inline-flex; align-items: center; gap: var(--s-4); font-weight: 600; }
  .prog { display: inline-flex; align-items: center; gap: var(--s-4); }
  tbody tr { cursor: pointer; }
  .muted { color: var(--muted); }
  .num { text-align: right; font-variant-numeric: tabular-nums; }
  .meter { display: block; width: 160px; height: 6px; border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
</style>
