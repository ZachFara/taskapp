<script lang="ts">
  // One field of a project row, drawn the same in every project layout.
  import type { FieldRef, ProjectRow } from '../../lib/bindings';
  import { PRIORITIES, priorityGlyph } from '../../lib/priority';
  import { percent, statusGlyph, type PCtx } from '../../lib/projectViews';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import { dayText } from './dates';

  let { field, row, ctx }: { field: FieldRef; row: ProjectRow; ctx: PCtx } = $props();
  const p = $derived(row.project);
  const status = $derived(ctx.statuses.find((s) => s.id === p.status_id));
  const prop = $derived(field.Property === undefined ? undefined : p.properties[field.Property]);
  const text = (v: unknown) => (v === undefined || v === null || v === '' ? '' : Array.isArray(v) ? v.join(', ') : v === true ? 'Yes' : v === false ? 'No' : String(v));
</script>

{#if field.Property !== undefined}
  <span class="t">{text(prop)}</span>
{:else if field.Fixed === 'Status'}
  {#if status}<span class="t"><StatusGlyph category={statusGlyph(status)} size={14} />{status.name}</span>{/if}
{:else if field.Fixed === 'StatusCategory'}
  <span class="t">{status?.category ?? ''}</span>
{:else if field.Fixed === 'Priority'}
  <span class="t"><PriorityGlyph level={priorityGlyph(p.priority)} size={14} />{p.priority ? PRIORITIES[p.priority] : ''}</span>
{:else if field.Fixed === 'Label'}
  <span class="t">{#each ctx.labels.filter((l) => p.label_ids.includes(l.id)) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{/each}</span>
{:else if field.Fixed === 'Progress'}
  <span class="t"><span class="meter" role="img" aria-label={`${row.issue_done} of ${row.issue_total} done`}><span class="fill" style:width={`${percent(row)}%`}></span></span><span class="m">{row.issue_done} of {row.issue_total}</span></span>
{:else if field.Fixed === 'IssueCount'}
  <span class="t n">{row.issue_total}</span>
{:else if field.Fixed === 'StartDate'}
  <span class="t m">{p.start_date ? dayText(p.start_date) : 'No start'}</span>
{:else if field.Fixed === 'TargetDate'}
  <span class="t m">{p.target_date ? dayText(p.target_date) : 'No target'}</span>
{:else if field.Fixed === 'Archived'}
  <span class="t m">{p.archived_at ? 'Archived' : ''}</span>
{:else if field.Fixed === 'Title'}
  <span class="t">{p.name}</span>
{/if}

<style>
  .t { display: inline-flex; align-items: center; gap: var(--s-3); min-width: 0; flex-wrap: wrap; }
  .m { color: var(--muted); }
  .n { font-variant-numeric: tabular-nums; }
  .meter { display: block; width: 96px; height: 6px; border-radius: var(--r-round); background: var(--active); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); }
</style>
