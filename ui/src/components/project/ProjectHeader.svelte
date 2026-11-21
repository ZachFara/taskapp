<script lang="ts">
  import { api } from '../../lib/api';
  import { openExternal } from '../../lib/openExternal';
  import type { Project } from '../../lib/bindings';

  let {
    project,
    progress,
    pinned,
    ontogglepin,
    onsaved,
  }: {
    project: Project;
    progress: [number, number];
    pinned: boolean;
    ontogglepin: () => void;
    onsaved: (p: Project) => void;
  } = $props();

  // Only Obsidian and https links get a button; anything else (javascript:, http:, file:) is dropped.
  const docHref = $derived(project.doc_link && /^(obsidian|https):\/\//i.test(project.doc_link) ? project.doc_link : null);

  let editing = $state(false);
  let draft = $state('');

  function openDoc() {
    if (docHref) void openExternal(docHref);
  }

  async function saveDescription() {
    const description = draft.trim() || null;
    onsaved(await api.projects.update({ ...project, description }));
    editing = false;
  }
</script>

<header>
  <h1>{project.name}</h1>
  {#if docHref}
    <button type="button" data-testid="open-obsidian" onclick={openDoc}>Open in Obsidian</button>
  {/if}
  <button type="button" data-testid="pin-toggle" aria-pressed={pinned} onclick={ontogglepin}>
    {pinned ? 'Unpin' : 'Pin'}
  </button>
</header>

{#if project.description}
  <p data-testid="description">{project.description}</p>
{:else if editing}
  <form onsubmit={(e) => { e.preventDefault(); saveDescription(); }}>
    <textarea bind:value={draft} aria-label="Description"></textarea>
    <button type="submit">Save</button>
  </form>
{:else}
  <button type="button" onclick={() => { draft = ''; editing = true; }}>Add description</button>
{/if}

<p>
  {progress[0]} / {progress[1]} done
  {#if project.start_date} · Start <time>{project.start_date}</time>{/if}
  {#if project.target_date} · Target <time>{project.target_date}</time>{/if}
</p>
