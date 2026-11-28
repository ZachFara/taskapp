<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { copyLink } from '../lib/deepLink';
  import type { Decision } from '../lib/bindings';
  import { decisions, openDecisions, refreshDecisions } from '../lib/decisions.svelte';
  import Button from '../ui/Button.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Pill from '../ui/Pill.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextInput from '../ui/TextInput.svelte';
  import { images, openLinks, renderMarkdown } from './detail/markdown';

  let tab = $state('open');
  let notes = $state<Record<number, string>>({});
  let texts = $state<Record<number, string>>({});
  let keys = $state<Record<number, string>>({});
  let error = $state('');
  // The decision last clicked or focused: the command menu acts on it. Before any click it is the first open one.
  let focused = $state<number | null>(null);

  const open = $derived(openDecisions());
  const closed = $derived(decisions.all.filter((d) => d.status !== 'open'));
  const shown = $derived(tab === 'open' ? open : closed);
  const target = $derived(open.find((d) => d.id === focused) ?? open[0]);
  // Copy link works on answered decisions too, so it looks in the shown list.
  const linkTarget = $derived(shown.find((d) => d.id === focused) ?? shown[0]);
  const optionLabel = (d: Decision, key: string | null) => d.options.find((o) => o.key === key)?.label ?? key ?? '';

  // Linked issues show as their keys; ids not seen yet are looked up once.
  $effect(() => {
    for (const id of decisions.all.flatMap((d) => d.issue_ids)) {
      if (!(id in keys)) {
        keys[id] = '';
        api.issues.get(id).then((i) => (keys[id] = i.key), () => delete keys[id]);
      }
    }
  });

  async function act(run: () => Promise<unknown>) {
    error = '';
    try {
      await run();
      await refreshDecisions();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }
  const answer = (d: Decision, option: string | null, text: string | null) =>
    act(() => api.decisions.answer(d.id, option, text, notes[d.id] ?? ''));
  const withdraw = (d: Decision) => act(() => api.decisions.withdraw(d.id));

  $effect(() => {
    const on: Record<string, () => void> = {
      'decision.answer': () => target?.recommendation && void answer(target, target.recommendation, null),
      'decision.withdraw': () => target && void withdraw(target),
      'decision.copyLink': () => linkTarget && void copyLink('decision', linkTarget.id),
      'view.refresh': () => void refreshDecisions(),
    };
    for (const [k, f] of Object.entries(on)) addEventListener(`command:${k}`, f);
    return () => Object.entries(on).forEach(([k, f]) => removeEventListener(`command:${k}`, f));
  });

  // `/decisions#<id>` (a decision link) shows that decision's tab, focuses it and scrolls to it.
  onMount(async () => {
    await refreshDecisions();
    const d = decisions.all.find((x) => `#${x.id}` === location.hash);
    if (!d) return;
    tab = d.status === 'open' ? 'open' : 'closed';
    focused = d.id;
    await tick();
    document.getElementById(`decision-${d.id}`)?.scrollIntoView?.({ block: 'center' });
  });
</script>

<PageHeader title="Decisions" icon="icon:help">
  {#snippet tabs()}
    <Tabs tabs={[{ id: 'open', label: `Open (${open.length})` }, { id: 'closed', label: `Answered (${closed.length})` }]} bind:value={tab} label="Decisions" />
  {/snippet}
</PageHeader>

<main class="body">
  {#if decisions.error}
    <p role="alert" class="error">{decisions.error}</p>
  {:else if decisions.loaded && shown.length === 0}
    <EmptyState title={tab === 'open' ? 'Nothing waits for you' : 'No answered decisions yet'} hint="A decision is a question with options that an agent or you raise with `taskapp decision create`. Answer it here with one click." />
  {/if}
  {#if error}<p role="alert" class="error">{error}</p>{/if}

  {#each shown as d (d.id)}
    <div class="slot" id="decision-{d.id}" role="presentation" onfocusin={() => (focused = d.id)} onclick={() => (focused = d.id)}>
    <article class="card" data-decision={d.id} data-blocking={d.mode === 'blocking' || undefined} data-target={target?.id === d.id || undefined} aria-label={d.question}>
      <div class="meta">
        {#if d.mode === 'blocking'}<Pill color="red">Blocking</Pill>{:else}<Pill>Assumed</Pill>{/if}
        {#if d.status !== 'open'}<Pill color={d.status === 'answered' ? 'green' : 'gray'}>{d.status === 'answered' ? 'Answered' : 'Withdrawn'}</Pill>{/if}
        <span class="src">{d.source} · {d.created_at.slice(0, 10)}</span>
        {#each d.issue_ids as id (id)}
          {#if keys[id]}<a class="issue" href={`/issue/${keys[id]}`} onclick={(e) => (e.preventDefault(), navigate(`/issue/${keys[id]}`))}>{keys[id]}</a>{/if}
        {/each}
      </div>
      <h2>{d.question}</h2>
      {#if d.context}<div class="md" use:openLinks use:images={d.context}>{@html renderMarkdown(d.context)}</div>{/if}

      {#if d.status === 'open'}
        <ul class="options">
          {#each d.options as o (o.key)}
            <li>
              <button type="button" class="opt" data-recommended={o.key === d.recommendation || undefined} onclick={() => answer(d, o.key, null)}>
                <span class="top"><strong>{o.label}</strong>{#if o.key === d.recommendation}<Pill color="green">Recommended</Pill>{/if}</span>
                {#if o.tradeoff}<span class="trade">{o.tradeoff}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
        <div class="extra">
          <TextInput label={`Note for "${d.question}"`} placeholder="Note with your answer (optional)" bind:value={() => notes[d.id] ?? '', (v) => (notes[d.id] = v)} />
          <TextInput label={`Own answer for "${d.question}"`} placeholder="Or answer in your own words" bind:value={() => texts[d.id] ?? '', (v) => (texts[d.id] = v)} />
          <Button icon="check" disabled={!texts[d.id]?.trim()} onclick={() => answer(d, null, texts[d.id])}>Send answer</Button>
          <Button onclick={() => withdraw(d)}>Withdraw</Button>
        </div>
      {:else}
        <p class="done">
          {#if d.status === 'answered'}<strong>{optionLabel(d, d.answer)}</strong>{:else}No longer needed.{/if}
          {#if d.note}<span class="trade"> {d.note}</span>{/if}
        </p>
      {/if}
    </article>
    </div>
  {/each}
</main>

<style>
  .body { padding: 0 var(--s-7) var(--s-7); display: grid; gap: var(--s-6); max-width: 760px; }
  .slot { display: contents; }
  .card { display: grid; gap: var(--s-4); padding: var(--s-6); border: 1px solid var(--border); border-radius: var(--r-sheet); background: var(--card); }
  .card[data-blocking] { border-color: var(--c-red); box-shadow: inset 3px 0 0 var(--c-red); }
  .card[data-target]:focus-within { border-color: var(--focus); }
  .meta { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-3); font-size: var(--fs-sm); color: var(--muted); }
  .issue { color: var(--ink); font-weight: 600; text-decoration: none; }
  .issue:hover { text-decoration: underline; }
  h2 { font-size: var(--fs-lg); font-weight: 700; letter-spacing: -0.01em; margin: 0; }
  .md { color: var(--ink); font-size: var(--fs-md); line-height: var(--lh-body); }
  .md :global(p) { margin: 0 0 var(--s-3); }
  .options { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-3); }
  .opt { display: grid; gap: var(--s-2); width: 100%; padding: var(--s-4) var(--s-5); border: 1px solid var(--border); border-radius: var(--r-ctl); background: var(--sheet); color: var(--ink); text-align: left; transition: background var(--dur-1) var(--ease); }
  .opt:hover { background: var(--active); }
  .opt[data-recommended] { border-color: var(--positive); }
  .top { display: flex; align-items: center; gap: var(--s-3); }
  .trade { color: var(--muted); font-size: var(--fs-sm); }
  .extra { display: flex; flex-wrap: wrap; gap: var(--s-3); align-items: center; }
  .extra :global(.field) { flex: 1 1 220px; width: auto; }
  .done { margin: 0; }
  .error { color: var(--warning); }
</style>
