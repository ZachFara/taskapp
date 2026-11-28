<script lang="ts">
  import { api } from '../lib/api';
  import type { Issue, Project } from '../lib/bindings';
  import { isOn } from '../lib/features.svelte';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import OptionList from './detail/OptionList.svelte';

  // Watches every write: when a project's flagged next action was finished (or cancelled), offers to flag the following one.
  let flagged = new Map<number, number>(); // project id -> flagged issue id
  let open = $state(false);
  let asked = $state<{ project: Project; done: Issue; choices: Issue[] } | null>(null);
  let chain: Promise<unknown> = Promise.resolve();

  async function check() {
    if (!isOn('next_actions')) return void flagged.clear();
    const rows = await api.next.list();
    const now = new Map(rows.filter((r) => r.issue).map((r) => [r.project.id, r.issue!.id]));
    const lost = [...flagged].filter(([p]) => !now.has(p));
    flagged = now;
    if (asked || !lost.length) return;
    const statuses = await api.statuses.list();
    for (const [pid, iid] of lost) {
      const done = await api.issues.get(iid);
      if (done.project_id !== pid || done.archived_at || !['Completed', 'Canceled'].includes(statuses.find((s) => s.id === done.status_id)?.category ?? '')) continue;
      const project = rows.find((r) => r.project.id === pid)!.project;
      const issues = await api.views.query({ filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: pid } }, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
      const choices = issues.filter((i) => !['Completed', 'Canceled'].includes(statuses.find((s) => s.id === i.status_id)?.category ?? ''));
      if (!choices.length) continue;
      asked = { project, done, choices };
      open = true;
      return;
    }
  }
  $effect(() => {
    const run = () => void (chain = chain.then(check).catch(() => {}));
    run();
    addEventListener('history:wrote', run);
    return () => removeEventListener('history:wrote', run);
  });
  const pick = async (id: string | number | null) => {
    open = false;
    if (id !== null && asked) await api.projects.setNext(asked.project.id, Number(id));
  };
</script>

{#if asked}
  <Dialog bind:open onclosed={() => (asked = null)} title="Pick the next action" description={`${asked.done.key} is finished. What comes next in ${asked.project.name}?`}>
    <OptionList label="Next action" value={null} options={asked.choices.map((i) => ({ id: i.id, name: `${i.key} ${i.title}` }))} onpick={pick} />
    {#snippet actions()}<Button onclick={() => (open = false)}>Skip</Button>{/snippet}
  </Dialog>
{/if}
