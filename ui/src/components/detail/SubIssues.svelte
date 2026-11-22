<script lang="ts">
  import { api } from '../../lib/api';
  import type { Issue, Status } from '../../lib/bindings';
  import EmptyState from '../../ui/EmptyState.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { pct } from './format';
  import IssuePicker from './IssuePicker.svelte';

  let { issue, act, rev, open }: { issue: Issue; act: (p: Promise<unknown>) => void; rev: number; open: (key: string) => void } = $props();
  let children = $state<Issue[]>([]);
  let statuses = $state<Status[]>([]);
  let title = $state('');
  let collapsed = $state(false);
  let add = $state<HTMLElement>();

  $effect(() => {
    rev;
    Promise.all([api.issues.children(issue.id), api.statuses.list()]).then(([c, s]) => ([children, statuses] = [c, s]));
  });
  const cat = (i: Issue) => statuses.find((s) => s.id === i.status_id)?.category ?? 'Backlog';
  const done = $derived(children.filter((c) => cat(c) === 'Completed').length);
  const attach = (child: Issue) => child.id !== issue.id && act(api.issues.update(child.id, { parent_id: issue.id }));
  const create = (e: Event) => {
    e.preventDefault();
    const t = title.trim();
    title = '';
    if (t) act(api.issues.create({ title: t, parent_id: issue.id, project_id: issue.project_id }));
  };
</script>

<section aria-label="Sub-issues">
  <GroupHeader name="Sub-issues" count={children.length} category="Unstarted" {collapsed} ontoggle={() => (collapsed = !collapsed)} onadd={() => add?.querySelector('input')?.focus()} />
  {#if !collapsed}
    {#if children.length}
      <div class="progress" role="progressbar" aria-label="Sub-issue progress" aria-valuemin="0" aria-valuemax="100" aria-valuenow={pct(done, children.length)}>
        <span class="bar" style:width={`${pct(done, children.length)}%`}></span>
        <span class="txt">{done} of {children.length} done</span>
      </div>
      <ul>
        {#each children as c (c.id)}
          <li><button type="button" onclick={() => open(c.key)}><StatusGlyph category={cat(c)} /><span class="key">{c.key}</span>{c.title}</button></li>
        {/each}
      </ul>
    {:else}
      <EmptyState title="Nothing broken down yet" hint="Split the work into sub-issues to track progress here." />
    {/if}
    <div class="add" bind:this={add}>
      <form onsubmit={create}><TextInput label="New sub-issue title" placeholder="New sub-issue, press Enter" bind:value={title} /></form>
      <IssuePicker selfId={issue.id} label="Add sub-issue" pick={attach} />
    </div>
  {/if}
</section>

<style>
  .progress { position: relative; height: var(--s-8); margin: var(--s-4) var(--s-4) 0; border-radius: var(--r-sm); background: var(--raised); overflow: hidden; }
  .bar { position: absolute; inset: 0 auto 0 0; background: var(--band-3); transition: width var(--dur-3) var(--ease); }
  .txt { position: relative; display: block; padding: 0 var(--s-5); line-height: var(--s-8); font-size: var(--fs-sm); font-weight: 700; color: var(--band-ink); }
  ul { list-style: none; padding: var(--s-2) 0; }
  li button { display: flex; align-items: center; gap: var(--s-5); width: 100%; height: var(--row-h); padding: 0 var(--s-5); border: 0; background: transparent; text-align: left; }
  li button:hover { background: var(--active); }
  .key { color: var(--muted); font-weight: 700; }
  .add { display: grid; gap: var(--s-4); padding: var(--s-4); }
</style>
