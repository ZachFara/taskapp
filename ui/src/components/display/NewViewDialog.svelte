<script lang="ts">
  import { api } from '../../lib/api';
  import type { SavedView, ViewDef, ViewEntity } from '../../lib/bindings';
  import { saveNew } from '../../lib/rowActions';
  import Tabs from '../../ui/Tabs.svelte';
  import { navigate } from '../../app/routes';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import ListRow from '../../ui/ListRow.svelte';
  import Pill from '../../ui/Pill.svelte';
  import { defaultTemplateId, templates, type ViewTemplate } from './templatesApi';

  // `current` is set when opened from a view, which adds "Copy of current view".
  // `entity` is what the new view lists; the dialog opens on it and the choice at the top changes it.
  let { open = $bindable(false), current, entity = $bindable('issue') }: { open?: boolean; current?: SavedView | null; entity?: ViewEntity } = $props();
  // The copy and the templates only fit views of the same kind.
  const copyable = $derived(current && current.entity === entity);

  const blank: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false };
  let choice = $state<'blank' | 'copy' | number>('blank');
  let list = $state<ViewTemplate[]>([]);
  let busy = $state(false);

  $effect(() => {
    if (!open) return;
    (async () => {
      list = await templates().list();
      const d = await defaultTemplateId();
      choice = entity === 'issue' && d !== null && list.some((x) => x.id === d) ? d : 'blank';
    })();
  });

  async function create() {
    busy = true;
    try {
      const v =
        choice === 'blank'
          ? await saveNew(entity)(null, 'Untitled view', blank, null, null)
          : choice === 'copy'
            ? await saveNew(entity)(null, `${current!.name} copy`, current!.def, current!.icon, current!.color)
            : await templates().createFromTemplate(choice, null);
      open = false;
      navigate(`/view/${v.id}`);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog bind:open title={entity === 'project' ? 'New project view' : 'New view'} description={entity === 'project' ? 'Start from a blank project view.' : 'Start from a blank view or from a template.'}>
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" icon="plus" disabled={busy} onclick={create}>Create view</Button>
  {/snippet}
  <div class="body">
  <div class="choices" role="group" aria-label="Start from">
    <ListRow selected={choice === 'blank'} onpick={() => (choice = 'blank')}>
      <span class="name">Blank</span>
      <Pill>List</Pill>
    </ListRow>
    {#if copyable}
      <ListRow selected={choice === 'copy'} onpick={() => (choice = 'copy')}>
        <span class="name">Copy of current view</span>
        <Pill>{current.def.layout}</Pill>
      </ListRow>
    {/if}
    {#each entity === 'issue' ? list : [] as t (t.id)}
      <ListRow selected={choice === t.id} onpick={() => (choice = t.id)}>
        <span class="name">{t.name}</span>
        <Pill>{t.def.layout}</Pill>
      </ListRow>
    {/each}
  </div>
  <div class="kind"><Tabs segmented label="View of" tabs={[{ id: 'issue', label: 'Issues' }, { id: 'project', label: 'Projects' }]} bind:value={() => entity, (v) => ((entity = v as ViewEntity), (choice = 'blank'))} /></div>
  </div>
</Dialog>

<style>
  /* After the list in the page, so the first thing to take focus is a row, not a tab whose tooltip would swallow Escape. */
  .body { display: flex; flex-direction: column; }
  .kind { order: -1; margin-bottom: var(--s-4); }
  .choices { display: grid; border: 1px solid var(--border); border-radius: var(--r-sm); overflow: hidden; }
  .name { flex: 1; font-weight: 600; }
</style>
