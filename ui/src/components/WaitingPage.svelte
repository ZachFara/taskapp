<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Issue } from '../lib/bindings';
  import { localDay } from '../lib/gantt';
  import { workspaceZone } from './views/dates';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  let rows = $state<Issue[] | null>(null);
  let today = $state('');
  const load = async () => {
    const [tz, list] = await Promise.all([workspaceZone(), api.waiting.list()]);
    [today, rows] = [localDay(new Date(), tz), list];
  };
  onMount(() => {
    void load();
    for (const e of ['rows:changed', 'history:wrote']) addEventListener(e, load);
    return () => ['rows:changed', 'history:wrote'].forEach((e) => removeEventListener(e, load));
  });
  // The list comes sorted: due follow-ups, later ones, then the issues with no day.
  const due = $derived((rows ?? []).filter((i) => i.follow_up !== null && i.follow_up <= today));
  const later = $derived((rows ?? []).filter((i) => i.follow_up !== null && i.follow_up > today));
  const undated = $derived((rows ?? []).filter((i) => i.follow_up === null));
  const go = (e: MouseEvent, to: string) => (e.preventDefault(), navigate(to));
</script>

{#snippet group(title: string, list: Issue[])}
  {#if list.length}
    <section class="group" aria-label={title}>
      <h2>{title} <span class="count">{list.length}</span></h2>
      <ul>
        {#each list as i (i.id)}
          <li data-key={i.key}>
            <a class="key" href={`/issue/${i.key}`} onclick={(e) => go(e, `/issue/${i.key}`)}>{i.key}</a>
            <span class="title">{i.title}</span>
            {#if i.waiting_on}<span class="who">waiting on {i.waiting_on}</span>{/if}
            {#if i.follow_up}<span class="when num">{i.follow_up}</span>{/if}
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{/snippet}

<PageHeader title="Waiting for" icon="icon:clock" />

<main class="body">
  {#if rows === null}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0}
    <EmptyState title="Nothing you are waiting on" hint="Open an issue and set Waiting on, or right-click it and choose Waiting on." />
  {:else}
    {@render group('Due follow-up', due)}
    {@render group('Later', later)}
    {@render group('No date', undated)}
  {/if}
</main>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-6); }
  .group h2 { margin: 0 0 var(--s-3); font-size: var(--fs-sm); font-weight: 600; color: var(--muted); }
  .count { font-variant-numeric: tabular-nums; }
  ul { margin: 0; padding: 0; list-style: none; }
  li { display: flex; flex-wrap: wrap; gap: var(--s-3); align-items: baseline; min-height: var(--row-h); padding: 0 var(--s-4); border-bottom: 1px solid var(--border); }
  li:hover { background: var(--active); }
  .key { color: var(--muted); font-variant-numeric: tabular-nums; text-decoration: none; }
  .title { font-weight: 500; }
  .who, .when { color: var(--muted); font-size: var(--fs-sm); }
  .num { font-variant-numeric: tabular-nums; }
</style>
