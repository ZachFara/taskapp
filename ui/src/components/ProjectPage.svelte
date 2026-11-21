<script lang="ts">
  import { api } from '../lib/api';
  import type { Milestone, Project, ResolvedPin } from '../lib/bindings';
  import ProjectHeader from './project/ProjectHeader.svelte';
  import ProjectIssues from './project/ProjectIssues.svelte';

  let { params }: { params: { slug?: string } } = $props();
  const slug = $derived(params.slug ?? '');

  // undefined while loading, null when no project has this slug.
  let project = $state<Project | null | undefined>(undefined);
  let progress = $state<[number, number]>([0, 0]);
  let milestones = $state<Milestone[]>([]);
  let pins = $state<ResolvedPin[]>([]);

  const pinned = $derived(pins.find((p) => p.pin.kind === 'Project' && p.pin.target === slug));

  async function reload(slug: string) {
    const found = (await api.projects.list()).find((p) => p.slug === slug) ?? null;
    project = found;
    if (!found) return;
    [progress, milestones, pins] = await Promise.all([
      api.projects.progress(found.id),
      api.projects.listMilestones(found.id),
      api.pins.listResolved(),
    ]);
  }

  async function togglePin() {
    if (pinned) await api.pins.unpin(pinned.pin.id);
    else await api.pins.pin('Project', slug, project?.name ?? slug);
    pins = await api.pins.listResolved();
  }

  // Reloads when the slug changes, since the router reuses this page for another project.
  $effect(() => {
    void reload(slug);
  });
</script>

{#if project === undefined}
  <p>Loading</p>
{:else if project === null}
  <h1>Project not found</h1>
  <p>No project has the slug <code>{slug}</code>.</p>
{:else}
  <ProjectHeader
    project={project}
    progress={progress}
    pinned={pinned !== undefined}
    ontogglepin={togglePin}
    onsaved={(p) => (project = p)}
  />

  {#if milestones.length}
    <section>
      <h2>Milestones</h2>
      <ul>
        {#each milestones as m (m.id)}
          <li>{m.name}{#if m.target_date} <time>{m.target_date}</time>{/if}</li>
        {/each}
      </ul>
    </section>
  {/if}

  <section>
    <h2>Issues</h2>
    <ProjectIssues projectId={project.id} />
  </section>
{/if}
