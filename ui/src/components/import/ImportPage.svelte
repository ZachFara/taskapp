<script lang="ts">
  import { api } from '../../lib/api';
  import type { ImportPlan } from '../../lib/bindings';
  import { navigate } from '../../app/routes';
  import { runUndo } from '../undoState.svelte';
  import { parseCsv } from './csv';
  import Button from '../../ui/Button.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';
  import Select from '../../ui/Select.svelte';
  import Tabs from '../../ui/Tabs.svelte';
  import TextInput from '../../ui/TextInput.svelte';

  const FIELDS = [
    { value: 'ignore', label: 'Ignore' },
    { value: 'title', label: 'Title' },
    { value: 'description', label: 'Description' },
    { value: 'status', label: 'Status' },
    { value: 'priority', label: 'Priority' },
    { value: 'project', label: 'Project' },
    { value: 'labels', label: 'Labels' },
    { value: 'start_date', label: 'Start date' },
    { value: 'due_date', label: 'Due date' },
    { value: 'estimate', label: 'Estimate' },
    { value: 'property', label: 'Custom property' },
  ];

  // Header names the core also recognises; anything else starts as Ignore.
  const GUESS: Record<string, string> = {
    title: 'title', name: 'title', summary: 'title',
    description: 'description', body: 'description', notes: 'description',
    status: 'status', state: 'status',
    priority: 'priority',
    project: 'project',
    labels: 'labels', label: 'labels', tags: 'labels',
    start_date: 'start_date', 'start date': 'start_date', start: 'start_date',
    due_date: 'due_date', 'due date': 'due_date', due: 'due_date', deadline: 'due_date',
    estimate: 'estimate', points: 'estimate',
  };
  const guess = (header: string) => GUESS[header.trim().toLowerCase()] ?? 'ignore';
  const msg = (e: unknown) => (e as { message?: string }).message ?? String(e);

  let source = $state('csv');
  let fileName = $state('');
  let csvText = $state('');
  let mapping = $state<Record<string, string>>({});
  let propName = $state<Record<string, string>>({});
  // The plan is a snapshot of one mapping; a changed mapping makes it stale until the next dry run.
  let plan = $state.raw<ImportPlan | null>(null);
  let planPairs = $state('');
  let done = $state<{ created: number; warnings: string[] } | null>(null);
  let undone = $state(false);
  let error = $state('');

  const rows = $derived(parseCsv(csvText));
  const headers = $derived((rows[0] ?? []).map((h) => h.trim()));
  const sample = (i: number) => rows[1]?.[i]?.trim() ?? '';

  const pairs = $derived(
    headers.flatMap((h): [string, string][] => {
      const f = mapping[h] ?? 'ignore';
      if (f === 'ignore') return [];
      return [[h, f === 'property' ? `property:${propName[h]?.trim() ?? ''}` : f]];
    }),
  );
  const hasTitle = $derived(pairs.some(([, f]) => f === 'title'));
  const missingName = $derived(headers.some((h) => mapping[h] === 'property' && !propName[h]?.trim()));
  const canDryRun = $derived(hasTitle && !missingName);
  const planValid = $derived(plan !== null && planPairs === JSON.stringify(pairs));
  const count = $derived(plan?.issues?.length ?? 0);

  async function load(file: File) {
    csvText = await file.text();
    fileName = file.name;
    mapping = Object.fromEntries((parseCsv(csvText)[0] ?? []).map((h) => [h.trim(), guess(h)]));
    propName = {};
    plan = null;
    done = null;
    undone = false;
    error = '';
  }

  async function dryRun() {
    error = '';
    done = null;
    const sent = pairs;
    try {
      plan = (await api.import.csvPlan(csvText, sent)) as ImportPlan;
      planPairs = JSON.stringify(sent);
    } catch (e) {
      plan = null;
      error = msg(e);
    }
  }

  // Nothing is written until this press; a failed apply leaves the plan on screen and writes nothing.
  async function apply() {
    if (!plan) return;
    error = '';
    try {
      done = await api.import.apply(plan);
      undone = false;
    } catch (e) {
      error = msg(e);
    }
  }

  const onFile = (e: Event) => {
    const file = (e.currentTarget as HTMLInputElement).files?.[0];
    if (file) void load(file);
  };
  // The drop area takes a file the same way the picker does; the browser default (opening the file) is suppressed.
  const onDrop = (e: DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer?.files?.[0];
    if (file) void load(file);
  };

  async function undo() {
    error = '';
    try {
      await runUndo(api);
      undone = true;
    } catch (e) {
      error = msg(e);
    }
  }

  const stats = $derived(
    plan
      ? [
          ['issues', plan.issues?.length ?? 0],
          ['labels', plan.labels?.length ?? 0],
          ['projects', plan.projects?.length ?? 0],
          ['statuses', plan.statuses?.length ?? 0],
          ['cycles', plan.cycles?.length ?? 0],
          ['properties', plan.property_defs?.length ?? 0],
        ]
      : [],
  );
</script>

{#snippet tabsSlot()}
  <Tabs
    bind:value={source}
    label="Import source"
    tabs={[
      { id: 'csv', label: 'CSV', icon: 'download' },
      { id: 'linear', label: 'Linear', icon: 'link' },
    ]}
  />
{/snippet}

<section class="import">
  <PageHeader title="Import" icon="icon:download" tabs={tabsSlot} />

  {#if source === 'csv'}
    <div class="step">
      <h2><span class="num">1</span> Choose a file</h2>
      <label class="drop" data-has-file={fileName !== ''} ondragover={(e) => e.preventDefault()} ondrop={onDrop}>
        <input class="sr" type="file" accept=".csv,text/csv" aria-label="CSV file" onchange={onFile} />
        <span class="drop-title">{fileName || 'Choose a CSV file'}</span>
        <span class="drop-hint">The first row is the header. Each row becomes one issue.</span>
      </label>
    </div>

    {#if headers.length > 0}
      <div class="step">
        <h2><span class="num">2</span> Map the columns</h2>
        <table class="map">
          <thead>
            <tr><th scope="col">Column</th><th scope="col">Sample</th><th scope="col">Maps to</th></tr>
          </thead>
          <tbody>
            {#each headers as h, i (h + i)}
              <tr>
                <th scope="row">{h}</th>
                <td class="sample">{sample(i)}</td>
                <td>
                  <Select bind:value={() => mapping[h], (v) => (mapping[h] = v)} label={`Map ${h}`} items={FIELDS} />
                  {#if mapping[h] === 'property'}
                    <TextInput bind:value={() => propName[h] ?? '', (v) => (propName[h] = v)} label={`Property name for ${h}`} placeholder="Property name" invalid={!propName[h]?.trim()} />
                  {/if}
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
        {#if !hasTitle}
          <p class="note">Map one column to Title to continue.</p>
        {/if}
        <div class="actions">
          <Button variant="primary" disabled={!canDryRun} onclick={dryRun}>Dry run</Button>
        </div>
      </div>
    {/if}

    {#if plan && planValid && !done}
      <div class="step">
        <h2><span class="num">3</span> Review the plan</h2>
        <dl class="stats">
          {#each stats as [name, n] (name)}
            <div><dt>{name}</dt><dd>{n}</dd></div>
          {/each}
        </dl>
        {#if plan.warnings?.length}
          <ul class="warnings">
            {#each plan.warnings as w, i (i)}<li>{w}</li>{/each}
          </ul>
        {/if}
        {#if plan.skipped?.length}
          <table class="skipped">
            <thead><tr><th scope="col">Row</th><th scope="col">Skipped because</th></tr></thead>
            <tbody>
              {#each plan.skipped as s, i (i)}
                <tr><td>{s.row}</td><td>{s.reason}</td></tr>
              {/each}
            </tbody>
          </table>
        {/if}
        {#if plan.unmapped?.length}
          <p class="note">Not imported: {plan.unmapped.join(', ')}</p>
        {/if}
        <div class="actions">
          <Button variant="primary" disabled={count === 0} onclick={apply}>Import {count} issues</Button>
        </div>
      </div>
    {:else if plan && !planValid}
      <p class="note">The mapping changed. Dry run again to see the plan.</p>
    {/if}

    {#if done && !undone}
      <div class="step done">
        <h2>Imported {done.created} issues</h2>
        {#each done.warnings as w, i (i)}<p class="note">{w}</p>{/each}
        <div class="actions">
          <Button variant="quiet" onclick={undo}>Undo import</Button>
          <Button variant="quiet" onclick={() => navigate('/my')}>View issues</Button>
        </div>
      </div>
    {:else if undone}
      <p class="note">Import undone.</p>
    {/if}
  {:else}
    <div class="step">
      <h2>Linear</h2>
      <p>
        Cairn reads the Linear token from the <code>LINEAR_API_KEY</code> environment variable of the app or the CLI.
        This screen never asks for it.
      </p>
      <p>Run the first command in a terminal to see the plan. It writes nothing. The second writes the plan.</p>
      <ol class="cmds">
        <li><code>taskapp import linear</code></li>
        <li><code>taskapp import linear --apply</code></li>
      </ol>
    </div>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}
</section>

<style>
  .import { display: flex; flex-direction: column; gap: var(--s-7); padding: var(--s-7); max-width: 56rem; }
  .step { display: flex; flex-direction: column; gap: var(--s-5); background: var(--card); border: 1px solid var(--border); border-radius: var(--r-sheet); padding: var(--s-7); }
  h2 { font-size: var(--fs-lg); margin: 0; display: flex; align-items: center; gap: var(--s-4); color: var(--ink); }
  .num { display: inline-grid; place-items: center; width: var(--s-8); height: var(--s-8); border-radius: var(--r-round); background: var(--accent); color: var(--accent-ink); font-size: var(--fs-sm); }
  .sr { position: absolute; width: 1px; height: 1px; overflow: hidden; clip: rect(0 0 0 0); }
  .drop { display: flex; flex-direction: column; gap: var(--s-2); align-items: flex-start; padding: var(--s-9); border: 2px dashed var(--control); border-radius: var(--r-sheet); background: var(--raised); cursor: pointer; }
  .drop:focus-within { outline: 2px solid var(--focus); outline-offset: 2px; }
  .drop[data-has-file='true'] { border-style: solid; border-color: var(--accent); }
  .drop-title { font-size: var(--fs-title); font-weight: 600; color: var(--ink); }
  .drop-hint, .note { color: var(--muted); font-size: var(--fs-md); margin: 0; }
  table { width: 100%; border-collapse: collapse; font-size: var(--fs-md); }
  th, td { text-align: left; padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); vertical-align: top; }
  thead th { font-size: var(--fs-sm); color: var(--muted); font-weight: 600; background: var(--band-1); }
  .sample { color: var(--muted); }
  .actions { display: flex; gap: var(--s-4); justify-content: flex-end; }
  .stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(7rem, 1fr)); gap: var(--s-4); margin: 0; }
  .stats div { background: var(--band-1); border-radius: var(--r-ctl); padding: var(--s-5); }
  dt { font-size: var(--fs-sm); color: var(--muted); }
  dd { margin: 0; font-size: var(--fs-display); font-weight: 700; color: var(--stone); }
  .warnings { color: var(--warning); margin: 0; padding-left: var(--s-7); }
  .skipped td:first-child { width: 4rem; }
  .done { background: var(--band-2); }
  .cmds { margin: 0; padding-left: var(--s-7); display: flex; flex-direction: column; gap: var(--s-3); }
  code { font-family: var(--font); background: var(--raised); padding: var(--s-1) var(--s-3); border-radius: var(--r-sm); }
  .error { color: var(--signal); margin: 0; font-weight: 600; }
</style>
