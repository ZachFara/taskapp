<script lang="ts">
  import type { Component } from 'svelte';
  import { navigate } from '../../app/routes';
  import Backups from './Backups.svelte';
  import Calendar from './Calendar.svelte';
  import Cycles from './Cycles.svelte';
  import Export from './Export.svelte';
  import Fields from './Fields.svelte';
  import General from './General.svelte';
  import ImportPage from '../import/ImportPage.svelte';
  import Labels from './Labels.svelte';
  import Projects from './Projects.svelte';
  import Properties from './Properties.svelte';
  import Reminders from './Reminders.svelte';
  import RelationKinds from './RelationKinds.svelte';
  import Statuses from './Statuses.svelte';
  import Templates from './Templates.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import Icon from '../../ui/Icon.svelte';
  import PageHeader from '../../ui/PageHeader.svelte';

  // Keyboard shortcuts is not a page: its entry opens the cheat sheet command instead.
  const sections: { id: string; label: string; icon: string; screen?: Component; action?: string }[] = [
    { id: 'general', label: 'General', icon: 'settings', screen: General },
    { id: 'statuses', label: 'Statuses', icon: 'circle', screen: Statuses },
    { id: 'labels', label: 'Labels', icon: 'tag', screen: Labels },
    { id: 'projects', label: 'Projects', icon: 'project', screen: Projects },
    { id: 'cycles', label: 'Cycles', icon: 'cycle', screen: Cycles },
    { id: 'fields', label: 'Fields', icon: 'list', screen: Fields },
    { id: 'properties', label: 'Properties', icon: 'layers', screen: Properties },
    { id: 'relations', label: 'Relations', icon: 'link', screen: RelationKinds },
    { id: 'templates', label: 'Templates', icon: 'bookmark', screen: Templates },
    { id: 'import', label: 'Import', icon: 'arrow-down', screen: ImportPage },
    { id: 'reminders', label: 'Reminders', icon: 'bell', screen: Reminders },
    { id: 'calendar', label: 'Calendar', icon: 'calendar', screen: Calendar },
    { id: 'export', label: 'Export', icon: 'download', screen: Export },
    { id: 'backups', label: 'Backups', icon: 'archive', screen: Backups },
    { id: 'shortcuts', label: 'Keyboard shortcuts', icon: 'help', action: 'command:help.shortcuts' },
  ];

  let path = $state(location.pathname);
  const id = $derived(path.split('/')[2] ?? 'general');
  const current = $derived(sections.find((s) => s.id === id));
</script>

<svelte:window onpopstate={() => (path = location.pathname)} />

<div class="page">
<PageHeader icon="icon:settings">
  {#snippet title()}<span class="crumb">Settings /</span> {current?.label ?? 'Settings'}{/snippet}
</PageHeader>
<div class="settings">
  <nav aria-label="Settings sections">
    <ul>
      {#each sections as s (s.id)}
        <li>
          <a
            href={s.action ? '#shortcuts' : `/settings/${s.id}`}
            aria-current={s.id === id ? 'page' : undefined}
            onclick={(e) => {
              e.preventDefault();
              if (s.action) dispatchEvent(new CustomEvent(s.action));
              else navigate(`/settings/${s.id}`);
            }}
          ><Icon name={s.icon} />{s.label}</a>
        </li>
      {/each}
    </ul>
  </nav>
  <section class="pane">
    {#if current?.screen}
      {@const Screen = current.screen}
      <Screen />
    {:else}
      <EmptyState title="No settings page for this section" hint="Pick another section on the left." />
    {/if}
  </section>
</div>
</div>

<style>
  /* The page fills the shell's main area: the header and rail stay put, only the content pane scrolls. */
  .page { height: 100%; display: flex; flex-direction: column; min-height: 0; }
  .crumb { color: var(--muted); font-weight: 500; }
  .settings { display: grid; grid-template-columns: var(--sidebar-w) minmax(0, 1fr); flex: 1; min-height: 0; }
  nav {
    background: var(--raised);
    color: var(--ink);
    padding: var(--s-7) var(--s-5);
    display: flex;
    flex-direction: column;
    gap: var(--s-6);
    border-right: 1px solid var(--border);
  }
  nav ul { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-1); }
  nav a {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--s-5);
    height: var(--row-h);
    padding: 0 var(--s-6);
    border-radius: var(--r-ctl);
    color: var(--muted);
    font-weight: 500;
    text-decoration: none;
    transition: background var(--dur-1) var(--ease), color var(--dur-1) var(--ease);
  }
  nav a:hover { color: var(--ink); background: var(--selection); }
  nav a[aria-current='page'] { color: var(--ink); background: var(--active); font-weight: 700; }
  nav a[aria-current='page']::before {
    content: '';
    position: absolute;
    left: 0;
    top: var(--s-3);
    bottom: var(--s-3);
    width: 3px;
    border-radius: var(--r-sm);
    background: var(--signal);
  }
  .pane {
    display: flex;
    flex-direction: column;
    gap: var(--s-8);
    padding: var(--s-9) var(--s-10);
    max-width: 58rem;
    overflow: auto;
  }

  /* Shared by every settings screen: a display heading, a status chip, a card with a form grid. */
  :global(.st-head) { display: grid; gap: var(--s-3); }
  :global(.st-head h2) { font-size: var(--fs-display); font-weight: 700; letter-spacing: -0.03em; line-height: var(--lh-tight); }
  :global(.st-lede) { color: var(--muted); max-width: 52ch; line-height: var(--lh-body); }
  :global(.st-card) {
    display: grid;
    gap: var(--s-7);
    padding: var(--s-8);
    background: var(--card);
    border: 1px solid var(--border);
    border-radius: var(--r-sheet);
  }
  :global(.st-status) {
    display: inline-flex;
    align-items: center;
    gap: var(--s-4);
    width: fit-content;
    height: var(--ctl-h-sm);
    padding: 0 var(--s-6) 0 var(--s-5);
    border-radius: var(--r-round);
    background: var(--raised);
    color: var(--ink);
    font-weight: 600;
    font-size: var(--fs-md);
  }
  :global(.st-status::before) { content: ''; width: 8px; height: 8px; border-radius: var(--r-round); background: var(--muted); }
  :global(.st-status[data-state='ok']::before) { background: var(--positive); }
  :global(.st-status[data-state='warn']::before) { background: var(--warning); }
  :global(.st-form) { display: grid; gap: var(--s-7); grid-template-columns: repeat(auto-fit, minmax(16rem, 1fr)); }
  :global(.st-field) { display: grid; gap: var(--s-3); align-content: start; }
  :global(.st-field > span:first-child) { font-weight: 600; font-size: var(--fs-md); }
  :global(.st-hint) { color: var(--muted); font-size: var(--fs-sm); line-height: var(--lh-body); }
  :global(.st-actions) { display: flex; flex-wrap: wrap; align-items: center; gap: var(--s-5); }
  :global(.st-error) { color: var(--signal); font-weight: 600; font-size: var(--fs-md); }
  :global(.st-path) { font-family: var(--font); font-size: var(--fs-sm); color: var(--muted); overflow-wrap: anywhere; }
  :global(.st-list) { list-style: none; margin: 0; padding: 0; display: grid; gap: var(--s-2); }
  :global(.st-list li) { padding: var(--s-3) 0; border-top: 1px solid var(--border); }

  @media (max-width: 40rem) {
    .settings { grid-template-columns: 1fr; }
    .pane { padding: var(--s-7); }
  }
</style>
