<script lang="ts">
  import { api } from '../../lib/api';
  import type { Cycle, Issue, IssuePatch, Label, Milestone, Project, Status } from '../../lib/bindings';

  let { issue, save, act }: { issue: Issue; save: (p: IssuePatch) => void; act: (p: Promise<unknown>) => void } = $props();
  let statuses = $state<Status[]>([]);
  let projects = $state<Project[]>([]);
  let cycles = $state<Cycle[]>([]);
  let labels = $state<Label[]>([]);
  let milestones = $state<Milestone[]>([]);

  $effect(() => {
    Promise.all([api.statuses.list(), api.projects.list(), api.cycles.list(), api.labels.list()]).then(([s, p, c, l]) => {
      [statuses, projects, cycles, labels] = [s, p, c, l];
    });
  });
  $effect(() => {
    const id = issue.project_id;
    (id === null ? Promise.resolve([]) : api.projects.listMilestones(id)).then((m) => (milestones = m));
  });

  const num = (e: Event) => ((e.currentTarget as HTMLSelectElement).value === '' ? null : Number((e.currentTarget as HTMLSelectElement).value));
  const timed = $derived(issue.due?.DateTime !== undefined);
  const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const pad = (n: number) => String(n).padStart(2, '0');
  // Local date and time of the due value, for the inputs.
  const dueParts = $derived.by(() => {
    if (issue.due?.Date !== undefined) return { date: issue.due.Date, time: '09:00' };
    if (issue.due?.DateTime) {
      const d = new Date(issue.due.DateTime.utc);
      return { date: `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`, time: `${pad(d.getHours())}:${pad(d.getMinutes())}` };
    }
    return { date: '', time: '09:00' };
  });

  function setDue(date: string, time: string | null) {
    if (!date) return save({ due: null });
    save({ due: time === null ? { Date: date } : { DateTime: { utc: new Date(`${date}T${time}`).toISOString(), tz } } });
  }
</script>

{#snippet pick(label: string, value: number | null, options: { id: number; name: string }[], patch: (v: number | null) => IssuePatch, none = true)}
  <label>{label}
    <select value={value ?? ''} onchange={(e) => save(patch(num(e)))}>
      {#if none}<option value="">None</option>{/if}
      {#each options as o (o.id)}<option value={o.id}>{o.name}</option>{/each}
    </select>
  </label>
{/snippet}

<section aria-label="Fields">
  {@render pick('Status', issue.status_id, statuses, (v) => ({ status_id: v }))}
  {@render pick('Priority', issue.priority, [0, 1, 2, 3, 4].map((n) => ({ id: n, name: ['None', 'Urgent', 'High', 'Medium', 'Low'][n] })), (v) => ({ priority: v }), false)}
  {@render pick('Project', issue.project_id, projects, (v) => ({ project_id: v, milestone_id: null }))}
  {@render pick('Milestone', issue.milestone_id, milestones, (v) => ({ milestone_id: v }))}
  {@render pick('Cycle', issue.cycle_id, cycles.map((c) => ({ id: c.id, name: c.name ?? `Cycle ${c.number}` })), (v) => ({ cycle_id: v }))}
  <fieldset>
    <legend>Labels</legend>
    {#each labels as l (l.id)}
      <label><input type="checkbox" checked={issue.label_ids.includes(l.id)}
        onchange={(e) => act(e.currentTarget.checked ? api.labels.addLabel(issue.id, l.id) : api.labels.removeLabel(issue.id, l.id))} />{l.name}</label>
    {/each}
  </fieldset>
  <label>Start <input type="date" value={issue.start_date ?? ''} onchange={(e) => save({ start_date: e.currentTarget.value || null })} /></label>
  <label>Due <input type="date" value={dueParts.date} onchange={(e) => setDue(e.currentTarget.value, timed ? dueParts.time : null)} /></label>
  <label><input type="checkbox" aria-label="Due time" checked={timed} disabled={!dueParts.date}
    onchange={(e) => setDue(dueParts.date, e.currentTarget.checked ? dueParts.time : null)} />Time</label>
  {#if timed}
    <input type="time" aria-label="Due time of day" value={dueParts.time} onchange={(e) => setDue(dueParts.date, e.currentTarget.value)} />
  {/if}
  <label>Estimate (min) <input type="number" value={issue.estimate_minutes ?? ''}
    onchange={(e) => save({ estimate_minutes: e.currentTarget.value === '' ? null : Number(e.currentTarget.value) })} /></label>
</section>

<style>
  section { display: grid; gap: 8px; }
  label { display: grid; gap: 2px; font-size: 12px; }
</style>
