<script lang="ts">
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { PRIORITY_OPTIONS } from '../../lib/priority';
  import type { IssueTemplate, Label, TemplateBody, TemplateFields, TemplateInput } from '../../lib/bindings';
  import { projects, propertyDefs, refreshAll, statuses } from '../../lib/stores.svelte';
  import Button from '../../ui/Button.svelte';
  import Checkbox from '../../ui/Checkbox.svelte';
  import Contour from '../../ui/Contour.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import RowActions from '../../ui/RowActions.svelte';
  import { rowActs, rowUi } from '../../lib/rowUi.svelte';
  import { patch } from '../../lib/rowActions';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import SectionHead from './SectionHead.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import Skeleton from '../../ui/Skeleton.svelte';
  import TextArea from '../../ui/TextArea.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  // Draft strings mirror the form; numbers and ids are parsed only when saved.
  type Draft = {
    id: number | null;
    name: string;
    slug: string;
    icon: string;
    color: string;
    title_prefix: string;
    description: string;
    status_id: string;
    priority: string;
    project_id: string;
    estimate: string;
    picked: Record<number, boolean>;
    properties: Record<string, string>;
    subs: string[];
  };

  const PRIORITIES = PRIORITY_OPTIONS.map(({ value, name }) => ({ value: String(value), label: name }));
  const SLUG = /^[a-z0-9-]+$/;

  const blank = (): Draft => ({
    id: null, name: '', slug: '', icon: '', color: '', title_prefix: '', description: '',
    status_id: '', priority: '', project_id: '', estimate: '', picked: {}, properties: {}, subs: [],
  });

  function fromTemplate(t: IssueTemplate): Draft {
    const f: TemplateFields = t.body.fields ?? {};
    return {
      id: t.id, name: t.name, slug: t.slug, icon: t.icon ?? '', color: t.color ?? '',
      title_prefix: t.body.title_prefix ?? '', description: f.description ?? '',
      status_id: f.status_id == null ? '' : String(f.status_id),
      priority: f.priority == null ? '' : String(f.priority),
      project_id: f.project_id == null ? '' : String(f.project_id),
      estimate: f.estimate_minutes == null ? '' : String(f.estimate_minutes),
      picked: Object.fromEntries((f.label_ids ?? []).map((id) => [id, true])),
      properties: Object.fromEntries(Object.entries(f.properties ?? {}).map(([k, v]) => [k, String(v ?? '')])),
      subs: (t.body.sub_issues ?? []).map((s) => s.title ?? ''),
    };
  }

  let list = $state<IssueTemplate[]>([]);
  let labels = $state<Label[]>([]);
  let loading = $state(true);
  let draft = $state<Draft>(blank());
  let open = $state(false);
  let attempted = $state(false);
  let confirming = $state(false);
  let error = $state('');
  let note = $state('');

  const slugError = $derived(
    !SLUG.test(draft.slug) ? 'Slug must be lowercase letters, digits or dashes.' : '',
  );
  const estimateError = $derived(draft.estimate && !/^\d+$/.test(draft.estimate) ? 'Estimate is whole minutes.' : '');

  async function load() {
    list = await api.templates.list();
    loading = false;
  }

  onMount(() => {
    void refreshAll();
    void api.labels.list().then((l) => (labels = l));
    void load();
  });

  function choose(t: IssueTemplate) {
    draft = fromTemplate(t);
    open = true; attempted = false; confirming = false; error = ''; note = '';
  }

  function startNew() {
    draft = blank();
    open = true; attempted = false; confirming = false; error = ''; note = '';
  }

  function toBody(): TemplateBody {
    const fields: TemplateFields = {};
    if (draft.description.trim()) fields.description = draft.description.trim();
    if (draft.status_id) fields.status_id = Number(draft.status_id);
    if (draft.priority) fields.priority = Number(draft.priority);
    if (draft.project_id) fields.project_id = Number(draft.project_id);
    const label_ids = labels.filter((l) => draft.picked[l.id]).map((l) => l.id);
    if (label_ids.length) fields.label_ids = label_ids;
    if (draft.estimate) fields.estimate_minutes = Number(draft.estimate);
    const properties = Object.fromEntries(Object.entries(draft.properties).filter(([, v]) => v.trim()));
    if (Object.keys(properties).length) fields.properties = properties;
    const body: TemplateBody = { fields, sub_issues: draft.subs.filter((s) => s.trim()).map((title) => ({ title })) };
    if (draft.title_prefix) body.title_prefix = draft.title_prefix;
    return body;
  }

  async function save() {
    attempted = true; error = ''; note = '';
    if (!draft.name.trim() || slugError || estimateError) return;
    const input: TemplateInput = {
      name: draft.name.trim(), slug: draft.slug, icon: draft.icon.trim() || null,
      color: draft.color || null, body: toBody(),
    };
    try {
      if (draft.id == null) {
        const t = await api.templates.create(input);
        draft.id = t.id;
      } else {
        await api.templates.update(draft.id, input);
      }
      await load();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function remove() {
    if (draft.id == null) return;
    try {
      await api.templates.delete(draft.id);
      open = false;
      await load();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  async function tryIt() {
    if (draft.id == null) return;
    error = '';
    try {
      const r = await api.templates.apply(draft.id, {});
      note = `Created ${r.issue.title}${r.sub_issues.length ? ` with ${r.sub_issues.length} sub-issues` : ''}.`;
      if (r.skipped.length) note += ` Skipped: ${r.skipped.join('; ')}.`;
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= draft.subs.length) return;
    const next = [...draft.subs];
    [next[i], next[j]] = [next[j], next[i]];
    draft.subs = next;
  }

  const remaining = (i: number) => draft.subs.length - 1 - i;
</script>

<SectionHead lede="A template prefills an issue and can add sub-issues.">
  {#snippet actions()}
    <Button variant="primary" icon="plus" tooltip={{ label: 'New template' }} onclick={startNew}>New template</Button>
  {/snippet}
</SectionHead>

<div class="tpl">
  <section class="rail" aria-label="Templates">
    {#if loading}
      <Skeleton height="var(--row-h)" />
      <Skeleton height="var(--row-h)" />
      <Skeleton height="var(--row-h)" />
    {:else if list.length === 0}
      <EmptyState title="No templates yet" hint="A template prefills an issue and can add sub-issues.">
        {#snippet action()}<Button variant="primary" onclick={startNew}>New template</Button>{/snippet}
      </EmptyState>
    {:else}
      <ul class="list">
        {#each list as t (t.id)}
          <li>
            <RowActions label={t.name} actions={rowActs({ kind: 'template', id: t.id, name: t.name })}>
              <IconColorPicker label={`Icon and colour of ${t.name}`} value={{ icon: t.icon || null, color: t.color || null }} onchange={(v) => patch({ kind: 'template', id: t.id, name: t.name }, { icon: v.icon, color: v.color })} bind:open={() => rowUi.picking === `template:${t.id}`, (o) => (rowUi.picking = o ? `template:${t.id}` : null)} />
              <button class="row grow" aria-current={open && draft.id === t.id ? 'true' : undefined} onclick={() => choose(t)}>
                <span class="name">{t.name}</span>
                <span class="slug">/{t.slug}</span>
              </button>
            </RowActions>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section class="editor" aria-label="Template editor">
    {#if !open}
      <EmptyState title="Pick a template" hint="Or start a new one. Every default you set is applied when an issue is created from it." />
    {:else}
      <header class="head">
        <div class="topo" aria-hidden="true"><Contour rings={7} size={120} /></div>
        <div class="title">
          <span class="kicker">{draft.id == null ? 'New template' : 'Template'}</span>
          <h2>{draft.name.trim() || 'Untitled'}</h2>
          <span class="slugline">/{draft.slug || 'slug'}</span>
        </div>
      </header>

      <div class="grid">
        <TextInput label="Template name" bind:value={draft.name} placeholder="Bug report" invalid={attempted && !draft.name.trim()} />
        <TextInput label="Template slug" bind:value={draft.slug} placeholder="bug" invalid={attempted && !!slugError} />
        <IconColorPicker
          label="Template icon and colour"
          value={{ icon: draft.icon || null, color: draft.color || null }}
          onchange={(v) => { draft.icon = v.icon ?? ''; draft.color = v.color ?? ''; }}
        />
        <TextInput label="Title prefix" bind:value={draft.title_prefix} placeholder="[Bug] " />
        <Select label="Default status" bind:value={draft.status_id} items={statuses.map((s) => ({ value: String(s.id), label: s.name }))} placeholder="Not set" />
        <Select label="Default priority" bind:value={draft.priority} items={PRIORITIES} placeholder="Not set" />
        <Select label="Default project" bind:value={draft.project_id} items={projects.map((p) => ({ value: String(p.id), label: p.name }))} placeholder="Not set" />
        <TextInput label="Default estimate (minutes)" bind:value={draft.estimate} type="number" invalid={attempted && !!estimateError} />
      </div>
      {#if attempted && slugError}<p class="err" role="alert">{slugError}</p>{/if}
      {#if attempted && estimateError}<p class="err" role="alert">{estimateError}</p>{/if}

      <TextArea label="Default description" bind:value={draft.description} rows={3} />

      {#if labels.length}
        <fieldset class="labels">
          <legend>Default labels</legend>
          {#each labels as l (l.id)}
            <Checkbox label={l.name} bind:checked={() => !!draft.picked[l.id], (v) => (draft.picked[l.id] = v)} />
          {/each}
        </fieldset>
      {/if}

      {#if propertyDefs.length}
        <div class="grid">
          {#each propertyDefs as def (def.key)}
            <TextInput label={`Property ${def.name}`} bind:value={() => draft.properties[def.key] ?? '', (v) => (draft.properties[def.key] = v)} />
          {/each}
        </div>
      {/if}

      <div class="subs">
        <h3>Sub-issues <small>{draft.subs.length}</small></h3>
        <ol>
          {#each draft.subs as _sub, i (i)}
            <li>
              <span class="ord">{i + 1}</span>
              <TextInput label={`Sub-issue ${i + 1} title`} bind:value={draft.subs[i]} />
              <IconButton icon="chevron-up" label={`Move sub-issue ${i + 1} up`} disabled={i === 0} onclick={() => move(i, -1)} />
              <IconButton icon="chevron-down" label={`Move sub-issue ${i + 1} down`} disabled={remaining(i) === 0} onclick={() => move(i, 1)} />
              <IconButton icon="trash" label={`Remove sub-issue ${i + 1}`} onclick={() => (draft.subs = draft.subs.filter((_, j) => j !== i))} />
            </li>
          {/each}
        </ol>
        <Button icon="plus" onclick={() => (draft.subs = [...draft.subs, ''])}>Add sub-issue</Button>
      </div>

      {#if error}<p class="err" role="alert">{error}</p>{/if}
      {#if note}<p class="note" role="status">{note}</p>{/if}

      <footer class="actions">
        <Button variant="primary" onclick={save}>Save template</Button>
        <Button disabled={draft.id == null} onclick={tryIt}>Try it</Button>
        {#if draft.id != null}
          <Button variant="danger" onclick={() => (confirming = true)}>Delete template</Button>
        {/if}
      </footer>
    {/if}
  </section>
</div>

<Dialog bind:open={confirming} title="Delete this template?" description="The template goes. Issues already made from it stay.">
  {#snippet actions()}
    <Button onclick={() => (confirming = false)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete</Button>
  {/snippet}
</Dialog>

<style>
  .tpl { display: grid; grid-template-columns: minmax(14rem, 1fr) minmax(0, 2.2fr); gap: var(--s-6); align-items: start; }
  .rail { display: flex; flex-direction: column; gap: var(--s-2); }
  .list { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: var(--s-1); width: 100%; }
  /* Each row spans the list, so its actions sit at the list's right edge. */
  .list > li { display: flex; }
  /* The list is a narrow column: the row's actions take the slug's place while the row is hovered or focused. */
  .list > li:hover .slug, .list > li:focus-within .slug { visibility: hidden; }
  .row {
    all: unset; box-sizing: border-box; width: 100%; display: flex; align-items: center; gap: var(--s-3);
    min-height: var(--row-h); padding: 0 var(--s-3); border-radius: var(--r-ctl); cursor: pointer;
    transition: background var(--dur-1) var(--ease);
  }
  .row:hover { background: var(--raised); }
  .row:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .row[aria-current='true'] { background: var(--selection); box-shadow: inset 3px 0 0 var(--signal); }
  .name { flex: 1; color: var(--ink); font-weight: 600; }
  .slug { color: var(--muted); font-size: var(--fs-sm); }

  .editor { display: flex; flex-direction: column; gap: var(--s-5); min-width: 0; }
  .head {
    position: relative; overflow: hidden; display: flex; align-items: flex-end; min-height: 8rem;
    padding: var(--s-6); border-radius: var(--r-sheet); background: var(--band-2); color: var(--band-ink);
  }
  .topo { position: absolute; right: calc(var(--s-6) * -1); top: calc(var(--s-6) * -1); color: var(--band-4); pointer-events: none; }
  .title { position: relative; display: flex; flex-direction: column; gap: var(--s-1); }
  .kicker { font-size: var(--fs-xs); text-transform: uppercase; letter-spacing: 0.08em; color: var(--band-count); }
  h2 { margin: 0; font-size: var(--fs-title); line-height: var(--lh-tight); }
  .slugline { font-size: var(--fs-md); color: var(--band-count); }

  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(12rem, 1fr)); gap: var(--s-4); }
  .labels { display: flex; flex-wrap: wrap; gap: var(--s-3); border: 0; margin: 0; padding: 0; }
  .labels legend { font-size: var(--fs-sm); color: var(--muted); margin-bottom: var(--s-2); }

  .subs h3 { margin: 0 0 var(--s-3); font-size: var(--fs-lg); }
  .subs small { color: var(--muted); font-weight: 400; }
  .subs ol { list-style: none; margin: 0 0 var(--s-3); padding: 0; display: flex; flex-direction: column; gap: var(--s-2); }
  .subs li { display: grid; grid-template-columns: 1.5rem minmax(0, 1fr) auto auto auto; gap: var(--s-2); align-items: end; }
  .ord { color: var(--muted); font-size: var(--fs-sm); padding-bottom: var(--s-3); text-align: right; }

  .err { margin: 0; color: var(--signal); font-size: var(--fs-sm); }
  .note { margin: 0; color: var(--positive); font-size: var(--fs-sm); }
  .actions { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-3); padding-top: var(--s-4); border-top: 1px solid var(--border); }
  .confirm { display: inline-flex; align-items: center; gap: var(--s-2); color: var(--muted); }

  @media (max-width: 40rem) { .tpl { grid-template-columns: 1fr; } }
  @media (prefers-reduced-motion: reduce) { .row { transition: none; } }
</style>
