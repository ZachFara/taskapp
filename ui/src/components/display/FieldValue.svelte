<script lang="ts">
  // One renderer for a displayed field, shared by every layout so a custom property looks the same everywhere.
  import type { FieldRef, Issue, PropertyDef, Value } from '../../lib/bindings';
  import EntityIcon from '../../ui/EntityIcon.svelte';
  import Icon from '../../ui/Icon.svelte';
  import Pill from '../../ui/Pill.svelte';
  import PriorityGlyph from '../../ui/PriorityGlyph.svelte';
  import { priorityGlyph } from '../../lib/priority';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import { dayText, dueDay } from '../views/dates';
  import { issues } from '../../lib/stores.svelte';
  import { paletteKey, type Ctx } from './types';
  import { checklistCount } from '../detail/checklist';

  // `named` prefixes bare custom values (a number or date has no meaning without its field) where there is no column header.
  // `all` shows every label (a view with Wrap on) instead of two and a count.
  let { field, issue, ctx, named = false, all = false }: { field: FieldRef; issue: Issue; ctx: Ctx; named?: boolean; all?: boolean } = $props();

  const day = (iso: string | null) => (iso ? dayText(iso.slice(0, 10)) : '');
  const estimate = (m: number | null) => (m === null ? '' : m >= 60 ? `${Math.floor(m / 60)}h${m % 60 ? ` ${m % 60}m` : ''}` : `${m}m`);
  const optColor = (def: PropertyDef | undefined, v: Value) => paletteKey(def?.options.find((o) => o.value === v)?.color);
</script>

{#if field.Property !== undefined}
  {@const def = ctx.defs.find((d) => d.key === field.Property)}
  {@const raw = (issue.properties[field.Property] ?? null) as Value}
  {#if raw === null || raw === ''}
    <span class="none"></span>
  {:else if def?.kind === 'Select'}
    <Pill color={optColor(def, raw)}>{raw}</Pill>
  {:else if def?.kind === 'MultiSelect' && Array.isArray(raw)}
    <span class="chips">{#each raw as v (String(v))}<Pill color={optColor(def, v)}>{v}</Pill>{/each}</span>
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
  {@const level = priorityGlyph(issue.priority)}
  <PriorityGlyph {level} size={14} />
{:else if field.Fixed === 'Status'}
  {@const status = ctx.statuses.find((s) => s.id === issue.status_id)}
  <span class="with"><StatusGlyph category={status?.category ?? 'Backlog'} icon={status?.icon} color={status?.color} label={status?.name ?? 'Unknown'} /><span class="text">{status?.name}</span></span>
{:else if field.Fixed === 'Label'}
  {@const labels = issue.label_ids.flatMap((id) => ctx.labels.find((l) => l.id === id) ?? [])}
  {@const max = all ? labels.length : 2}
  <span class="chips" class:all>
    {#each labels.slice(0, max) as l (l.id)}<Pill color={l.color}>{l.name}</Pill>{/each}
    {#if labels.length > max}<span class="muted">+{labels.length - max}</span>{/if}
  </span>
{:else if field.Fixed === 'Project'}
  {@const project = ctx.projects.find((p) => p.id === issue.project_id)}
  {#if project}<span class="with"><EntityIcon icon={project.icon ?? 'icon:project'} color={project.color} size={14} /><span class="text">{project.name}</span></span>{/if}
{:else if field.Fixed === 'Cycle'}
  {@const cycle = ctx.cycles.find((c) => c.id === issue.cycle_id)}
  {#if cycle}<span class="text muted">{cycle.name ?? `Cycle ${cycle.number}`}</span>{/if}
{:else if field.Fixed === 'Milestone'}
  {@const milestone = ctx.milestones.find((m) => m.id === issue.milestone_id)}
  {#if milestone}<span class="text muted">{milestone.name}</span>{/if}
{:else if field.Fixed === 'Parent'}
  {@const parent = issue.parent_id === null ? undefined : issues.find((i) => i.id === issue.parent_id)}
  {#if parent}<span class="text muted">{parent.key}</span>{/if}
{:else if field.Fixed === 'SubIssueProgress'}
  {@const stat = ctx.stats.get(issue.id)}
  {#if stat && stat.sub_total > 0}<span class="num muted">{stat.sub_done}/{stat.sub_total}</span>{/if}
{:else if field.Fixed === 'Checklist'}
  {@const list = checklistCount(issue.description)}
  {#if list.total}<span class="num muted with"><Icon name="check" size={12} />{list.done}/{list.total}</span>{/if}
{:else if field.Fixed === 'RelationCount'}
  {@const stat = ctx.stats.get(issue.id)}
  <span class="num muted">{stat?.relations ?? 0}</span>
{:else if field.Fixed === 'CommentCount'}
  {@const stat = ctx.stats.get(issue.id)}
  <span class="num muted">{stat?.comments ?? 0}</span>
{:else if field.Fixed === 'DueDate'}
  {@const due = dueDay(issue, ctx.tz)}
  {#if due}<span class="num">{dayText(due)}</span>{/if}
{:else if field.Fixed === 'StartDate'}
  <span class="num">{day(issue.start_date)}</span>
{:else if field.Fixed === 'Estimate'}
  <span class="num">{estimate(issue.estimate_minutes)}</span>
{:else if field.Fixed === 'Points'}
  {#if ctx.points && issue.points !== null}<span class="num">{issue.points}</span>{/if}
{:else if field.Fixed === 'Depth'}
  {#if issue.depth}<span class="muted">{issue.depth}</span>{/if}
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
  .with, .chips { display: flex; align-items: center; gap: var(--s-3); min-width: 0; overflow: hidden; }
  .chips.all { flex-wrap: wrap; overflow: visible; }
  .with { color: var(--muted); }
  .check { display: inline-flex; color: var(--accent); }
  .url { display: inline-flex; color: var(--accent); }
</style>
