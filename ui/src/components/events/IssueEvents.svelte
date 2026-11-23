<script lang="ts">
  import { navigate } from '../../app/routes';
  import { api } from '../../lib/api';
  import type { Event, Issue } from '../../lib/bindings';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import SectionHeading from '../detail/SectionHeading.svelte';
  import { dayHeading, startDay } from './format';

  // The events linked to this issue, with add and remove. `act` runs a write and reloads, like the other sections.
  let { issue, act, rev }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number } = $props();
  let all = $state<Event[]>([]);
  let collapsed = $state(false);
  let pick = $state('');
  let sec = $state<HTMLElement>();

  const mine = $derived(all.filter((e) => e.issue_ids.includes(issue.id)));
  const others = $derived(all.filter((e) => !e.issue_ids.includes(issue.id)));

  $effect(() => {
    rev;
    api.events.list(null, null).then((l) => (all = l));
  });
  $effect(() => {
    if (!pick) return;
    const id = Number(pick);
    pick = '';
    act(api.events.link(id, issue.id));
  });
  // The command menu focuses the same control the section does.
  $effect(() => {
    const go = () => { collapsed = false; queueMicrotask(() => sec?.querySelector<HTMLElement>('[aria-label="Add to event"]')?.focus()); };
    addEventListener('command:issue.event.add', go);
    return () => removeEventListener('command:issue.event.add', go);
  });
</script>

<section aria-label="Events" bind:this={sec}>
  <SectionHeading name="Events" count={mine.length} {collapsed} ontoggle={() => (collapsed = !collapsed)} />
  {#if !collapsed}
    {#if !mine.length}<p class="none">No events</p>{/if}
    <ul>
      {#each mine as e (e.id)}
        <li>
          <button type="button" class="go" onclick={() => navigate(`/event/${e.id}`)}><em>{dayHeading(startDay(e))}</em> {e.title}</button>
          <IconButton icon="close" label="Remove from {e.title}" onclick={() => act(api.events.unlink(e.id, issue.id))} />
        </li>
      {/each}
    </ul>
    {#if others.length}
      <div class="add"><Select label="Add to event" bind:value={pick} placeholder="Add to event" items={others.map((e) => ({ value: String(e.id), label: `${dayHeading(startDay(e))}: ${e.title}` }))} /></div>
    {/if}
  {/if}
</section>

<style>
  ul { list-style: none; margin: 0; padding: 0; }
  .none { padding: var(--s-4) var(--s-5); color: var(--muted); }
  li { display: flex; align-items: center; height: var(--row-h); padding-right: var(--s-4); }
  .go { flex: 1; display: flex; gap: var(--s-4); align-items: center; height: 100%; padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; min-width: 0; }
  .go:hover { background: var(--active); }
  em { color: var(--muted); font-style: normal; font-size: var(--fs-sm); font-weight: 600; min-width: 6.5rem; }
  .add { padding: var(--s-4); }
</style>
