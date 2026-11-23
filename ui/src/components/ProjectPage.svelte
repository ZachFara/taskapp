<script lang="ts">
  import { api } from '../lib/api';
  import type { Issue, Layout, Milestone, Project, ResolvedPin, Status, ViewDef } from '../lib/bindings';
  import { navigate } from '../app/routes';
  import { openExternal } from '../lib/openExternal';
  import Button from '../ui/Button.svelte';
  import Dialog from '../ui/Dialog.svelte';
  import EmptyState from '../ui/EmptyState.svelte';
  import IconButton from '../ui/IconButton.svelte';
  import PageHeader from '../ui/PageHeader.svelte';
  import Skeleton from '../ui/Skeleton.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import ViewEditor from './ViewEditor.svelte';
  import ProjectHeader from './project/ProjectHeader.svelte';
  import ProjectIssues from './project/ProjectIssues.svelte';
  import { shortcutFor } from '../lib/commands/registry';
  import Milestones from './project/Milestones.svelte';
  import InsightsPanel from './insights/InsightsPanel.svelte';

  let { params }: { params: { slug?: string } } = $props();
  const slug = $derived(params.slug ?? '');

  // undefined while loading, null when no project has this slug.
  let project = $state<Project | null | undefined>(undefined);
  let progress = $state<[number, number]>([0, 0]);
  let milestones = $state<Milestone[]>([]);
  let rows = $state<Issue[]>([]);
  let statuses = $state<Status[]>([]);
  let pins = $state<ResolvedPin[]>([]);
  let layout = $state<Layout>('List');
  // raw: ViewEditor and the views clone it.
  let def = $state.raw<ViewDef>({ filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false });
  let renaming = $state(false);
  let confirming = $state(false);
  let insights = $state(false);

  const pinned = $derived(pins.find((p) => p.pin.kind === 'Project' && p.pin.target === slug));
  // Only Obsidian and https links get a button; anything else (javascript:, http:, file:) is dropped.
  const docHref = $derived(project?.doc_link && /^(obsidian|https):\/\//i.test(project.doc_link) ? project.doc_link : null);
  const layouts = [
    { id: 'List', label: 'List', icon: 'list' },
    { id: 'Board', label: 'Board', icon: 'board' },
    { id: 'Table', label: 'Table', icon: 'table' },
    { id: 'Gantt', label: 'Timeline', icon: 'timeline' },
    { id: 'Calendar', label: 'Calendar', icon: 'calendar' },
  ];

  async function reload(slug: string) {
    const found = (await api.projects.list()).find((p) => p.slug === slug) ?? null;
    project = found;
    if (!found) return;
    const all = api.views.query({
      filter: { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: found.id } },
      sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false,
    });
    [progress, milestones, pins, rows, statuses] = await Promise.all([
      api.projects.progress(found.id),
      api.projects.listMilestones(found.id),
      api.pins.listResolved(),
      all,
      api.statuses.list(),
    ]);
  }

  async function togglePin() {
    if (pinned) await api.pins.unpin(pinned.pin.id);
    else await api.pins.pin('Project', slug, project?.name ?? slug);
    pins = await api.pins.listResolved();
  }

  async function archive() {
    if (!project) return;
    confirming = false;
    await api.projects.update({ ...project, archived_at: new Date().toISOString() });
    navigate('/projects');
  }

  // Reloads when the slug changes, since the router reuses this page for another project.
  $effect(() => {
    void reload(slug);
  });

  // The command registry fires these; the page owns the state they act on.
  $effect(() => {
    const pin = () => void togglePin();
    const ask = () => (confirming = true);
    const panel = () => (insights = !insights);
    const rename = () => (renaming = true);
    addEventListener('command:project.rename', rename);
    addEventListener('command:project.pin', pin);
    addEventListener('command:project.archive', ask);
    addEventListener('command:insights.toggle', panel);
    return () => {
      removeEventListener('command:project.rename', rename);
      removeEventListener('command:project.pin', pin);
      removeEventListener('command:project.archive', ask);
      removeEventListener('command:insights.toggle', panel);
    };
  });
</script>

{#snippet crumb()}<span class="crumb">{project?.slug}</span>{/snippet}

{#if project === undefined}
  <div class="load" aria-busy="true"><Skeleton /><Skeleton /><Skeleton /></div>
{:else if project === null}
  <EmptyState title="Project not found" hint={`No project has the slug ${slug}. It may have been deleted or renamed.`}>
    {#snippet action()}<Button onclick={() => navigate('/projects')}>All projects</Button>{/snippet}
  </EmptyState>
{:else}
  <div class="page">
    <PageHeader title={crumb} icon={project.icon ?? 'icon:project'} color={project.color}>
      {#snippet actions()}
        {#if docHref}
          <Button icon="link" tooltip={{ label: 'Open the project doc' }} onclick={() => openExternal(docHref)}><span data-testid="open-obsidian">Open doc</span></Button>
        {/if}
        <IconButton icon="edit" label="Rename project" shortcut={shortcutFor('project.rename')} onclick={() => (renaming = true)} />
        <IconButton icon="pin" label={pinned ? 'Unpin project' : 'Pin project'} pressed={pinned !== undefined} onclick={togglePin} />
        <IconButton icon="archive" label="Archive project" onclick={() => (confirming = true)} />
      {/snippet}
    </PageHeader>

    <div class="cols">
    <div class="scroll">
      <ProjectHeader {project} {progress} bind:renaming onsaved={(p) => (project = p)} />
      <div class="ms">
        <Milestones projectId={project.id} {milestones} issues={rows} {statuses} onchange={() => reload(slug)} />
      </div>
      <div class="toolbar" data-testid="issues-toolbar">
        <Tabs tabs={layouts} bind:value={layout as string} label="Layout" />
        <ViewEditor {def} onchange={(p) => (def = { ...def, ...p })} />
      </div>
      <div class="issues"><ProjectIssues projectId={project.id} {layout} {def} /></div>
    </div>
    {#if insights}<InsightsPanel kind="project" id={project.id} title={project.name} onclose={() => (insights = false)} />{/if}
    </div>
  </div>

  <Dialog bind:open={confirming} title="Archive this project?" description="It leaves the project list. Its issues stay as they are, and you can restore it later.">
    {#snippet actions()}
      <Button onclick={() => (confirming = false)}>Cancel</Button>
      <Button variant="danger" onclick={archive}>Archive</Button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .page { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .cols { flex: 1; min-height: 0; display: flex; }
  .scroll { flex: 1; min-height: 0; overflow-y: auto; }
  .ms { padding: var(--s-7); }
  .toolbar { display: flex; align-items: center; gap: var(--s-7); flex-wrap: wrap; padding: 0 var(--s-7); border-top: 1px solid var(--border); border-bottom: 1px solid var(--border); }
  .crumb { font-size: var(--fs-lg); font-weight: 600; color: var(--muted); }
  .issues { height: 70vh; }
  .load { display: grid; gap: var(--s-5); padding: var(--s-8) var(--s-7); }
</style>
