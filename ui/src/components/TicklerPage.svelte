<script lang="ts">
  import { onMount } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import type { Issue } from '../lib/bindings';
  import { addDays, localDay } from '../lib/gantt';
  import { workspaceZone } from './views/dates';
  import { comesBackText } from './detail/format';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';

  let rows = $state<Issue[] | null>(null);
  let today = $state('');
  const load = async () => {
    const [tz, list] = await Promise.all([workspaceZone(), api.tickler.list()]);
    [today, rows] = [localDay(new Date(), tz), list];
  };
  onMount(() => {
    void load();
    for (const e of ['rows:changed', 'history:wrote']) addEventListener(e, load);
    return () => ['rows:changed', 'history:wrote'].forEach((e) => removeEventListener(e, load));
  });
  // The list comes sorted by return day. "This week" is the next 7 days.
  const soon = $derived((rows ?? []).filter((i) => i.hidden_until! <= addDays(today, 7)));
  const later = $derived((rows ?? []).filter((i) => i.hidden_until! > addDays(today, 7)));
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
            <span class="when num">{comesBackText(i.hidden_until!)}</span>
          </li>
        {/each}
      </ul>
    </section>
  {/if}
{/snippet}

<PageHeader title="Tickler" icon="icon:clock" />

<main class="body">
  {#if rows === null}
    <Skeleton height="var(--row-h)" />
  {:else if rows.length === 0}
    <EmptyState title="Nothing is deferred" hint="Defer an issue and it waits here until its day, then returns to the Inbox." />
  {:else}
    {@render group('This week', soon)}
    {@render group('Later', later)}
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
  .when { color: var(--muted); font-size: var(--fs-sm); }
  .num { font-variant-numeric: tabular-nums; }
</style>
