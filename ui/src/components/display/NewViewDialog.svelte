<script lang="ts">
  import { api } from '../../lib/api';
  import type { ViewDef } from '../../lib/bindings';
  import { navigate } from '../../app/routes';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import ListRow from '../../ui/ListRow.svelte';
  import Pill from '../../ui/Pill.svelte';
  import { defaultTemplateId, templates, type ViewTemplate } from './templatesApi';

  let { open = $bindable(false) }: { open?: boolean } = $props();

  const blank: ViewDef = { filter: null, sorts: [], group_by: null, layout: 'List', columns: [], include_archived: false };
  // 'blank' or a template id. "Copy of current view" is the view menu's "Save as new view", so it is not repeated here.
  let choice = $state<'blank' | number>('blank');
  let list = $state<ViewTemplate[]>([]);
  let busy = $state(false);

  $effect(() => {
    if (!open) return;
    const t = templates();
    (async () => {
      list = t ? await t.list() : [];
      const d = await defaultTemplateId();
      choice = d !== null && list.some((x) => x.id === d) ? d : 'blank';
    })();
  });

  async function create() {
    busy = true;
    try {
      const t = templates();
      const v =
        choice === 'blank' || !t
          ? await api.views.save(null, 'Untitled view', blank, null, null)
          : await t.createFromTemplate(choice, null);
      open = false;
      navigate(`/view/${v.id}`);
    } finally {
      busy = false;
    }
  }
</script>

<Dialog bind:open title="New view" description="Start from a blank view or from a template.">
  {#snippet actions()}
    <Button variant="quiet" onclick={() => (open = false)}>Cancel</Button>
    <Button variant="primary" icon="plus" disabled={busy} onclick={create}>Create view</Button>
  {/snippet}
  <div class="choices" role="group" aria-label="Start from">
    <ListRow selected={choice === 'blank'} onpick={() => (choice = 'blank')}>
      <span class="name">Blank</span>
      <Pill>List</Pill>
    </ListRow>
    {#each list as t (t.id)}
      <ListRow selected={choice === t.id} onpick={() => (choice = t.id)}>
        <span class="name">{t.name}</span>
        <Pill>{t.def.layout}</Pill>
      </ListRow>
    {/each}
  </div>
</Dialog>

<style>
  .choices { display: grid; border: 1px solid var(--border); border-radius: var(--r-sm); overflow: hidden; }
  .name { flex: 1; font-weight: 600; }
</style>
