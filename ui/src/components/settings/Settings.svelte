<script lang="ts">
  import { navigate } from '../../app/routes';
  import Backups from './Backups.svelte';
  import Calendar from './Calendar.svelte';
  import Cycles from './Cycles.svelte';
  import Export from './Export.svelte';
  import General from './General.svelte';
  import Labels from './Labels.svelte';
  import Projects from './Projects.svelte';
  import Properties from './Properties.svelte';
  import Reminders from './Reminders.svelte';
  import Statuses from './Statuses.svelte';

  const sections = {
    general: General, statuses: Statuses, properties: Properties, projects: Projects, cycles: Cycles,
    labels: Labels, reminders: Reminders, calendar: Calendar, export: Export, backups: Backups,
  };

  let path = $state(location.pathname);
  const section = $derived(path.split('/')[2] ?? 'general');
  const Section = $derived(sections[section as keyof typeof sections]);
</script>

<svelte:window onpopstate={() => (path = location.pathname)} />

<div class="settings">
  <nav aria-label="Settings sections">
    {#each Object.keys(sections) as name (name)}
      <a
        href={`/settings/${name}`}
        aria-current={name === section ? 'page' : undefined}
        onclick={(e) => { e.preventDefault(); navigate(`/settings/${name}`); }}
      >{name}</a>
    {/each}
  </nav>
  <section>
    {#if Section}
      <Section />
    {:else}
      <p>Unknown settings section: {section}</p>
    {/if}
  </section>
</div>

<style>
  .settings { display: grid; grid-template-columns: 12rem 1fr; gap: 1.5rem; padding: 1rem; }
  nav { display: flex; flex-direction: column; gap: 0.25rem; }
  nav a { padding: 0.25rem 0.5rem; border-radius: 4px; color: inherit; text-decoration: none; text-transform: capitalize; }
  nav a[aria-current='page'] { background: rgba(127, 127, 127, 0.18); font-weight: 600; }
  section { display: flex; flex-direction: column; gap: 0.75rem; max-width: 40rem; }
  @media (max-width: 40rem) { .settings { grid-template-columns: 1fr; } }
</style>
