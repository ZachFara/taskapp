<script lang="ts">
  import { navigate } from '../app/routes';
  import { api } from '../lib/api';
  import { setTheme } from '../lib/theme.svelte';
  import Button from '../ui/Button.svelte';
  import Contour from '../ui/Contour.svelte';
  import Select from '../ui/Select.svelte';
  import Tabs from '../ui/Tabs.svelte';
  import TextInput from '../ui/TextInput.svelte';

  let { ondone }: { ondone: () => void } = $props();

  // Intl gives the system zone; the list keeps it selectable even if the runtime lacks supportedValuesOf.
  const detected = Intl.DateTimeFormat().resolvedOptions().timeZone;
  const zones = [...new Set([detected, ...(Intl.supportedValuesOf?.('timeZone') ?? [])])];

  let name = $state('');
  let key = $state('ABC');
  let timezone = $state(detected);
  let theme = $state<'system' | 'light' | 'dark'>('system');
  let tried = $state(false);
  let busy = $state(false);
  let failed = $state(false);
  let created = $state(false);

  const keyOk = $derived(/^[A-Z]{2,5}$/.test(key));
  const nameOk = $derived(name.trim() !== '');
  const error = $derived(
    !keyOk ? 'Use 2 to 5 letters for the key.' : tried && !nameOk ? 'Name the workspace.' : failed ? 'Could not create the workspace. Try again.' : '',
  );

  // The attribute previews the choice; it is saved only once the workspace exists.
  $effect(() => {
    const el = document.documentElement;
    if (theme === 'system') el.removeAttribute('data-theme');
    else el.dataset.theme = theme;
  });

  async function create(e: SubmitEvent) {
    e.preventDefault();
    tried = true;
    if (!keyOk || !nameOk || busy) return;
    busy = true;
    failed = false;
    try {
      await api.workspace.setup(name.trim(), key, timezone);
      // The workspace exists already; a theme that fails to save falls back to the local cache.
      await api.settings.set('theme', theme).catch(() => {});
      setTheme(theme, false);
      created = true;
    } catch {
      failed = true;
    } finally {
      busy = false;
    }
  }
</script>

<div class="setup cairn-root" data-on-frame>
  <div class="art" aria-hidden="true"><Contour rings={13} size={640} /></div>

  <section class="intro">
    <div class="mark-row">
      <svg class="mark" width="36" height="36" viewBox="0 0 16 16" aria-hidden="true">
        <ellipse cx="8" cy="12.8" rx="6.4" ry="2.6" fill="currentColor" />
        <ellipse cx="8.5" cy="7.8" rx="4.4" ry="2.3" fill="currentColor" />
        <ellipse cx="7.7" cy="3.6" rx="2.6" ry="1.8" fill="currentColor" />
      </svg>
      <span class="wordmark">Cairn</span>
    </div>
    <h1>Stack your work, one stone at a time.</h1>
    <p class="lede">Name the workspace and pick how its issues are numbered. You can change all of this later in settings.</p>
    <p class="preview" aria-label="Issue numbering preview">{key || 'ABC'}-1</p>
  </section>

  {#if created}
    <section class="card next" aria-label="Next step">
      <h2>Workspace ready</h2>
      <p class="note">Start with an empty workspace, or bring your tasks in from a CSV file.</p>
      <div class="actions">
        <Button variant="quiet" onclick={() => ondone()}>Start empty</Button>
        <Button variant="primary" onclick={() => { navigate('/settings/import'); ondone(); }}>Import from CSV</Button>
      </div>
    </section>
  {:else}
  <form class="card" onsubmit={create} novalidate>
    <h2>Create your workspace</h2>

    <label class="row">
      <span>Workspace name</span>
      <TextInput label="Workspace name" bind:value={name} placeholder="Home" invalid={tried && !nameOk} autofocus />
    </label>

    <label class="row">
      <span>Workspace key</span>
      <TextInput
        label="Workspace key"
        bind:value={() => key, (v) => (key = v.toUpperCase())}
        placeholder="ABC"
        invalid={!keyOk}
      />
    </label>

    <div class="row">
      <span>Time zone</span>
      <Select label="Time zone" bind:value={timezone} items={zones.map((z) => ({ value: z, label: z }))} />
    </div>

    <div class="row">
      <span>Theme</span>
      <Tabs
        label="Theme"
        bind:value={theme}
        tabs={[
          { id: 'system', label: 'System' },
          { id: 'light', label: 'Light' },
          { id: 'dark', label: 'Dark' },
        ]}
      />
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="actions">
      <Button variant="primary" type="submit" disabled={busy}>Create workspace</Button>
    </div>
  </form>
  {/if}
</div>

<style>
  .setup {
    position: fixed;
    inset: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(320px, 440px);
    align-items: center;
    gap: var(--s-10);
    padding: var(--s-10);
    overflow: auto;
  }
  .art {
    position: absolute;
    right: -160px;
    bottom: -200px;
    color: var(--frame-muted);
    opacity: 0.35;
    pointer-events: none;
  }
  .intro { position: relative; display: grid; gap: var(--s-7); max-width: 560px; }
  .mark-row { display: flex; align-items: center; gap: var(--s-5); color: var(--frame-ink); }
  .mark { color: var(--frame-ink); }
  .wordmark { font-size: var(--fs-wordmark); font-weight: 700; letter-spacing: 0.01em; }
  h1 { font-size: var(--fs-display); line-height: var(--lh-tight); font-weight: 700; color: var(--frame-ink); }
  .lede { color: var(--frame-muted); font-size: var(--fs-lg); }
  .preview {
    justify-self: start;
    padding: var(--s-3) var(--s-6);
    border: 1px solid var(--frame-muted);
    border-radius: var(--r-ctl);
    color: var(--frame-ink);
    font-size: var(--fs-title);
    font-weight: 600;
  }
  .card {
    position: relative;
    display: grid;
    gap: var(--s-7);
    padding: var(--s-9);
    border-radius: var(--r-sheet);
    background: var(--sheet);
    color: var(--ink);
  }
  h2 { font-size: var(--fs-title); font-weight: 600; }
  .row { display: grid; gap: var(--s-3); }
  .row > span { color: var(--muted); font-size: var(--fs-sm); font-weight: 500; }
  .error { color: var(--signal); font-size: var(--fs-sm); }
  .actions { display: flex; justify-content: flex-end; gap: var(--s-4); flex-wrap: wrap; }
  .next { align-content: start; }
  .note { color: var(--muted); line-height: var(--lh-body); }

  @media (max-width: 760px) {
    .setup { grid-template-columns: 1fr; padding: var(--s-7); }
    .art { display: none; }
  }
</style>
