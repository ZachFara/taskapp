<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Event, Issue, NextAction, Project, StaleIssue } from '../lib/bindings';
  import { addDays, localDay } from '../lib/gantt';
  import { isOn } from '../lib/features.svelte';
  import Button from '../ui/Button.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import { workspaceZone } from './views/dates';
  import { STEPS, allDone, back, finish, next, review, toggleDone } from './review/review.svelte';

  let loaded = $state(false);
  let error = $state('');
  let inbox = $state<Issue[]>([]);
  let waiting = $state<Issue[]>([]);
  let stuck = $state<Project[]>([]);
  let stale = $state<StaleIssue[]>([]);
  let someday = $state<Issue[]>([]);
  let back7 = $state<Issue[]>([]);
  let due = $state<Issue[]>([]);
  let events = $state<Event[]>([]);

  // Open issues in statuses with these names (the Process Inbox names Delegated and Someday).
  const named = async (names: string[]) => {
    const ids = (await api.statuses.list()).filter((s) => names.includes(s.name.toLowerCase())).map((s) => s.id);
    if (ids.length === 0) return [];
    return (await api.views.query({ filter: { Cond: { field: { Fixed: 'Status' }, op: 'In', value: ids } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false })) as Issue[];
  };

  onMount(async () => {
    try {
      const today = localDay(new Date(), await workspaceZone());
      const [i, w, nx, st, s, tk, d, ev] = await Promise.all([
        api.inbox.list(),
        named(['delegated', 'waiting']),
        api.next.list(),
        api.stale.list(),
        named(['someday']),
        api.tickler.list(),
        api.views.query({ filter: { And: [{ Cond: { field: { Fixed: 'DueDate' }, op: 'Gte', value: 'today' } }, { Cond: { field: { Fixed: 'DueDate' }, op: 'Lte', value: 'today+7' } }] }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false }),
        api.events.list(today, addDays(today, 7)),
      ]);
      inbox = i;
      waiting = w;
      stuck = (nx as NextAction[]).filter((r) => r.stuck).map((r) => r.project);
      stale = st;
      someday = s;
      back7 = tk.filter((x) => x.hidden_until! <= addDays(today, 7));
      due = (d as Issue[]).filter((x) => x.completed_at === null);
      events = ev;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
    loaded = true;
  });

  const counts = $derived([
    `${inbox.length} in Inbox`,
    `${waiting.length} waiting`,
    `${stuck.length} ${stuck.length === 1 ? 'project' : 'projects'} without a next action`,
    `${stale.length} stale`,
    `${someday.length} in Someday`,
    `${back7.length} coming back`,
    `${due.length} due, ${events.length} ${events.length === 1 ? 'event' : 'events'}`,
  ]);
  const go = (e: MouseEvent, to: string) => (e.preventDefault(), navigate(to));
</script>

<PageHeader title="Weekly review" icon="icon:check" />

<main class="body">
  {#if error}
    <p class="error">{error}</p>
  {:else if !loaded}
    <Skeleton height="var(--row-h)" />
  {:else}
    <ol class="steps" aria-label="Review steps">
      {#each STEPS as name, i (name)}
        <li class:current={i === review.at} class:done={review.done.includes(i)}>{name}</li>
      {/each}
    </ol>

    <section class="step">
      <h2>{STEPS[review.at]}</h2>
      <p class="count">{counts[review.at]}</p>

      {#if review.at === 0}
        {#if isOn('process_inbox')}<p><a href="/process" onclick={(e) => go(e, '/process')}>Process Inbox</a></p>{/if}
        {@render issues(inbox)}
      {:else if review.at === 1}
        {@render issues(waiting)}
      {:else if review.at === 2}
        <ul class="plain">
          {#each stuck as p (p.id)}<li><a href={`/project/${p.slug}`} onclick={(e) => go(e, `/project/${p.slug}`)}>{p.name}</a></li>{/each}
        </ul>
      {:else if review.at === 3}
        {@render issues(stale.map((s) => s.issue))}
      {:else if review.at === 4}
        {@render issues(someday)}
      {:else if review.at === 5}
        {@render issues(back7)}
      {:else}
        <h3>Due</h3>
        {@render issues(due)}
        <h3>Events</h3>
        {#if events.length === 0}
          <p class="quiet">Nothing here.</p>
        {:else}
          <ul class="plain">
            {#each events as ev (ev.id)}<li><a href={`/event/${ev.id}`} onclick={(e) => go(e, `/event/${ev.id}`)}>{ev.title}</a> <span class="when">{ev.starts_at.slice(0, 10)}</span></li>{/each}
          </ul>
        {/if}
      {/if}
    </section>

    <footer class="actions">
      <Button onclick={back} disabled={review.at === 0}>Back</Button>
      <Button onclick={toggleDone} icon="check">Done for this step</Button>
      <Button onclick={next} disabled={review.at === STEPS.length - 1}>Next</Button>
      <Button variant="primary" onclick={finish} disabled={!allDone()}>Finish</Button>
    </footer>
  {/if}
</main>

{#snippet issues(list: Issue[])}
  {#if list.length === 0}
    <p class="quiet">Nothing here.</p>
  {:else}
    <ul class="plain">
      {#each list as x (x.id)}
        <li><a class="key" href={`/issue/${x.key}`} onclick={(e) => go(e, `/issue/${x.key}`)}>{x.key}</a> {x.title}</li>
      {/each}
    </ul>
  {/if}
{/snippet}

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-5); }
  .steps { display: flex; flex-wrap: wrap; gap: var(--s-2); margin: 0; padding: 0; list-style: none; font-size: var(--fs-sm); color: var(--muted); }
  .steps li { padding: var(--s-1) var(--s-3); border: 1px solid var(--border); border-radius: var(--r-sm, 4px); }
  .steps li.current { color: inherit; font-weight: 600; }
  .steps li.done::before { content: '✓ '; }
  h2 { margin: 0; font-size: var(--fs-lg); }
  h3 { margin: var(--s-4) 0 var(--s-2); font-size: var(--fs-sm); color: var(--muted); }
  .count, .quiet, .when { color: var(--muted); font-size: var(--fs-sm); margin: 0 0 var(--s-3); }
  .plain { margin: 0; padding: 0; list-style: none; display: grid; gap: var(--s-2); font-size: var(--fs-sm); }
  a { color: inherit; text-decoration: none; }
  a:hover { text-decoration: underline; }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; }
  .actions { display: flex; gap: var(--s-3); }
  .error { color: var(--danger, inherit); }
</style>
