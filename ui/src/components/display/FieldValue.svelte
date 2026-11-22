<script lang="ts">
  // One renderer for a displayed field, shared by every layout so a custom property looks the same everywhere.
  import type { FieldRef, Issue, Value } from '../../lib/bindings';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import Icon from '../../ui/Icon.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import { priorityGlyph } from '../../lib/priority';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import { dayText, dueDay } from '../views/dates';
  import { issues } from '../../lib/stores.svelte';
  import { paletteKey, type Ctx } from './types';

  // `named` prefixes bare custom values (a number or date has no meaning without its field) where there is no column header.
  let { field, issue, ctx, named = false }: { field: FieldRef; issue: Issue; ctx: Ctx; named?: boolean } = $props();

  const day = (iso: string | null) => (iso ? dayText(iso.slice(0, 10)) : '');
  const estimate = (m: number | null) => (m === null ? '' : m >= 60 ? `${Math.floor(m / 60)}h${m % 60 ? ` ${m % 60}m` : ''}` : `${m}m`);
  const def = $derived(field.Property !== undefined ? ctx.defs.find((d) => d.key === field.Property) : undefined);
  const raw = $derived<Value>(field.Property !== undefined ? (issue.properties[field.Property] ?? null) : null);
  const optColor = (v: Value) => paletteKey(def?.options.find((o) => o.value === v)?.color);
  const stat = $derived(ctx.stats.get(issue.id));
  const milestone = $derived(ctx.milestones.find((m) => m.id === issue.milestone_id));
  const parent = $derived(issue.parent_id === null ? undefined : issues.find((i) => i.id === issue.parent_id));
  const status = $derived(ctx.statuses.find((s) => s.id === issue.status_id));
  const project = $derived(ctx.projects.find((p) => p.id === issue.project_id));
  const cycle = $derived(ctx.cycles.find((c) => c.id === issue.cycle_id));
  const labels = $derived(issue.label_ids.flatMap((id) => ctx.labels.find((l) => l.id === id) ?? []));
  const due = $derived(dueDay(issue, ctx.tz));
  const level = $derived(priorityGlyph(issue.priority));
</script>

{#if field.Property !== undefined}
  {#if raw === null || raw === ''}
    <span class="none"></span>
  {:else if def?.kind === 'Select'}
    <Pill color={optColor(raw)}>{raw}</Pill>
  {:else if def?.kind === 'MultiSelect' && Array.isArray(raw)}
    <span class="chips">{#each raw as v (String(v))}<Pill color={optColor(v)}>{v}</Pill>{/each}</span>
  {:else if def?.kind === 'Checkbox'}
    <span class="check" role="img" aria-label={raw ? 'Yes' : 'No'}>{#if raw}<Icon name="check" size={14} />{/if}</span>
  {:else if def?.kind === 'Url'}
    <a class="url" href={String(raw)} target="_blank" rel="noreferrer" onclick={(e) => e.stopPropagation()} aria-label={`Open ${raw}`}><Icon name="link" size={14} /></a>
  {:else if def?.kind === 'Date'}
    <span class="num">{#if named}<span class="muted lbl">{def.name}</span>{/if}{day(String(raw))}</span>
  {:else if def?.kind === 'Number'}
    <span class="num">{#if named}<span class="muted lbl">{def.name}</span>{/if}{raw}</span>
  {:else}
    <span class="text">{#if named}<span class="muted lbl">{def?.name}</span>{/if}{String(raw)}</span>
  {/if}
{:else if field.Fixed === 'Title'}
  <span class="text">{issue.title}</span>
{:else if field.Fixed === 'Key'}
  <span class="text muted">{issue.key}</span>
{:else if field.Fixed === 'Priority'}
  <PriorityGlyph {level} size={14} />
{:else if field.Fixed === 'Status'}
  <span class="with"><StatusGlyph category={status?.category ?? 'Backlog'} label={status?.name ?? 'Unknown'} /><span class="text">{status?.name}</span></span>
{:else if field.Fixed === 'Label'}
  <span class="chips">
    {#each labels.slice(0, 2) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{/each}
    {#if labels.length > 2}<span class="muted">+{labels.length - 2}</span>{/if}
  </span>
{:else if field.Fixed === 'Project'}
  {#if project}<span class="with"><EntityIcon icon={project.icon ?? 'icon:project'} color={project.color} size={14} /><span class="text">{project.name}</span></span>{/if}
{:else if field.Fixed === 'Cycle'}
  {#if cycle}<span class="text muted">{cycle.name ?? `Cycle ${cycle.number}`}</span>{/if}
{:else if field.Fixed === 'Milestone'}
  {#if milestone}<span class="text muted">{milestone.name}</span>{/if}
{:else if field.Fixed === 'Parent'}
  {#if parent}<span class="text muted">{parent.key}</span>{/if}
{:else if field.Fixed === 'SubIssueProgress'}
  {#if stat && stat.sub_total > 0}<span class="num muted">{stat.sub_done}/{stat.sub_total}</span>{/if}
{:else if field.Fixed === 'RelationCount'}
  <span class="num muted">{stat?.relations ?? 0}</span>
{:else if field.Fixed === 'CommentCount'}
  <span class="num muted">{stat?.comments ?? 0}</span>
{:else if field.Fixed === 'DueDate'}
  {#if due}<span class="num">{dayText(due)}</span>{/if}
{:else if field.Fixed === 'StartDate'}
  <span class="num">{day(issue.start_date)}</span>
{:else if field.Fixed === 'Estimate'}
  <span class="num">{estimate(issue.estimate_minutes)}</span>
{:else if field.Fixed === 'Points'}
  {#if ctx.points && issue.points !== null}<span class="num">{issue.points}</span>{/if}
{:else if field.Fixed === 'CreatedAt'}
  <span class="num">{day(issue.created_at)}</span>
{:else if field.Fixed === 'UpdatedAt'}
  <span class="num">{day(issue.updated_at)}</span>
{/if}

<style>
  .num { font-variant-numeric: tabular-nums; }
  .text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .lbl { margin-right: var(--s-2); }
  .muted { color: var(--muted); }
  .with, .chips { display: inline-flex; align-items: center; gap: var(--s-3); min-width: 0; overflow: hidden; }
  .with { color: var(--muted); }
  .check { display: inline-flex; color: var(--accent); }
  .url { display: inline-flex; color: var(--accent); }
</style>
