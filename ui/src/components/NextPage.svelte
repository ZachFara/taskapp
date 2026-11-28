<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Issue, NextAction } from '../lib/bindings';
  import { isOn } from '../lib/features.svelte';
  import { localDay } from '../lib/gantt';
  import { workspaceZone } from './views/dates';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  let rows = $state<NextAction[] | null>(null);
  // Waiting issues whose follow-up day has come are next actions too, listed above the projects.
  let followUps = $state<Issue[]>([]);
  const load = async () => {
    rows = (await api.next.list()).filter((r) => r.issue || r.stuck);
    if (!isOn('waiting')) return void (followUps = []);
    const [tz, waiting] = await Promise.all([workspaceZone(), api.waiting.list()]);
    const today = localDay(new Date(), tz);
    followUps = waiting.filter((i) => i.follow_up !== null && i.follow_up <= today);
  };
  onMount(() => {
    void load();
    for (const e of ['rows:changed', 'history:wrote']) addEventListener(e, load);
    return () => ['rows:changed', 'history:wrote'].forEach((e) => removeEventListener(e, load));
  });
  const go = (e: MouseEvent, to: string) => (e.preventDefault(), navigate(to));
</script>

<PageHeader title="Next actions" icon="icon:flag" />

<main class="body">
  {#if rows === null}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0 && followUps.length === 0}
    <EmptyState title="No next actions yet" hint="Open a project and set its next action, or right-click an issue and choose Make next action." />
  {:else}
    {#if followUps.length}
      <section class="follow" aria-label="Follow-ups due">
        <h2>Follow-ups due</h2>
        <ul>
          {#each followUps as i (i.id)}
            <li data-key={i.key}><a class="key" href={`/issue/${i.key}`} onclick={(e) => go(e, `/issue/${i.key}`)}>{i.key}</a> {i.title}{#if i.waiting_on} <span class="pick">waiting on {i.waiting_on}</span>{/if}</li>
          {/each}
        </ul>
      </section>
    {/if}
    <table class="table">
      <thead><tr><th scope="col">Project</th><th scope="col">Next action</th></tr></thead>
      <tbody>
        {#each rows as r (r.project.id)}
          <tr data-slug={r.project.slug}>
            <td><a class="name" href={`/project/${r.project.slug}`} onclick={(e) => go(e, `/project/${r.project.slug}`)}>{r.project.name}</a></td>
            <td>
              {#if r.issue}
                <a class="key" href={`/issue/${r.issue.key}`} onclick={(e) => go(e, `/issue/${r.issue!.key}`)}>{r.issue.key}</a> {r.issue.title}
              {:else}
                <Pill color="red">Stuck</Pill> <a class="pick" href={`/project/${r.project.slug}`} onclick={(e) => go(e, `/project/${r.project.slug}`)}>Choose a next action</a>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</main>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); }
  .table { width: 100%; border-collapse: collapse; }
  th { text-align: left; font-size: var(--fs-xs); font-weight: 600; color: var(--muted); padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); }
  td { height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  tbody tr:hover { background: var(--active); }
  .name { font-weight: 600; color: inherit; text-decoration: none; }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; text-decoration: none; }
  .pick { margin-left: var(--s-3); color: var(--muted); }
  .follow { padding: 0 var(--s-4) var(--s-5); }
  .follow h2 { margin: 0 0 var(--s-3); font-size: var(--fs-sm); font-weight: 600; color: var(--muted); }
  .follow ul { margin: 0; padding: 0; list-style: none; }
  .follow li { min-height: var(--row-h); display: flex; align-items: center; gap: var(--s-3); border-bottom: 1px solid var(--border); }
</style>
