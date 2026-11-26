<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { projects, propertyDefs, refreshAll } from '../../lib/stores.svelte';
  import type { PropertyAppliesTo, PropertyDef, PropertyKind, SelectOption } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import EmptyState from '../../ui/EmptyState.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import './entities.css';

  const kinds: PropertyKind[] = ['Text', 'Number', 'Date', 'Select', 'MultiSelect', 'Checkbox', 'Url', 'NoteLink'];
  const keyPattern = /^[a-z][a-z0-9_]{0,39}$/;
  const hasOptions = (k: PropertyKind) => k === 'Select' || k === 'MultiSelect';

  let error = $state('');
  let doomed = $state<PropertyDef | null>(null);
  let newName = $state('');
  let newKey = $state('');
  let newKind = $state<PropertyKind>('Text');
  let newScope = $state<number | null>(null);
  let newApplies = $state<PropertyAppliesTo>('Issues');
  const appliesItems = (['Issues', 'Projects', 'Both'] as const).map((a) => ({ value: a, label: a }));
  let optionDraft = $state<Record<string, string>>({});

  const scopes = $derived([{ id: null as number | null, name: 'All projects' }, ...projects]);
  const kindItems = kinds.map((k) => ({ value: k, label: k }));
  const scopeItems = $derived(scopes.map((s) => ({ value: String(s.id ?? 'null'), label: s.name })));

  onMount(() => refreshAll());

  // Every write goes through here so a refused write shows its message and the lists stay in step.
  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  function add() {
    if (!keyPattern.test(newKey)) {
      error = 'Keys start with a letter and use lowercase letters, digits and underscores.';
      return;
    }
    void write(async () => {
      await api.properties.createDef({
        key: newKey, name: newName.trim() || newKey, kind: newKind, options: [], project_id: newApplies === 'Projects' ? null : newScope,
        position: propertyDefs.length, applies_to: newApplies,
      });
      newName = newKey = '';
      newKind = 'Text';
      newScope = null;
      newApplies = 'Issues';
    });
  }

  const rename = (d: PropertyDef, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== d.name) void write(() => api.properties.renameDef(d.key, next));
  };

  // Options belong to the kind, so editing them is a kind change to the same kind.
  const setOptions = (d: PropertyDef, options: SelectOption[]) => write(() => api.properties.changeKind(d.key, d.kind, options));
  const addOption = (d: PropertyDef) => {
    const value = (optionDraft[d.key] ?? '').trim();
    if (!value) return;
    optionDraft[d.key] = '';
    void setOptions(d, [...d.options, { value, color: 'gray' }]);
  };
  const patchOption = (d: PropertyDef, i: number, patch: Partial<SelectOption>) =>
    setOptions(d, d.options.map((o, j) => (j === i ? { ...o, ...patch } : o)));

  function move(d: PropertyDef, by: -1 | 1) {
    const keys = propertyDefs.map((x) => x.key);
    const at = keys.indexOf(d.key);
    if (at + by < 0 || at + by >= keys.length) return;
    [keys[at], keys[at + by]] = [keys[at + by], keys[at]];
    void write(() => api.properties.reorder(keys));
  }

  const remove = () => write(async () => {
    await api.properties.deleteDef(doomed!.key);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="Your own fields on issues, projects or both. On issues a property can cover all projects or just one." />

  {#if propertyDefs.length}
    <ul class="es-list">
      {#each propertyDefs as d, i (d.key)}
        <li class="item">
          <div class="es-row">
            <div class="es-name" onchange={(e) => rename(d, e)}><TextInput label={`Name of ${d.name}`} value={d.name} /></div>
            <span class="es-meta">{d.key}</span>
            <Select label={`Kind of ${d.name}`} items={kindItems} bind:value={() => d.kind, (k) => write(() => api.properties.changeKind(d.key, k as PropertyKind, hasOptions(k as PropertyKind) ? d.options : []))} />
            <Select label={`Applies to of ${d.name}`} items={appliesItems} bind:value={() => d.applies_to, (a) => write(() => api.properties.setAppliesTo(d.key, a as PropertyAppliesTo))} />
            {#if d.applies_to !== 'Projects'}<Select label={`Scope of ${d.name}`} items={scopeItems} bind:value={() => String(d.project_id ?? 'null'), (id) => write(() => api.properties.setScope(d.key, id === 'null' ? null : Number(id)))} />{/if}
            <div class="es-tools">
              <IconButton icon="arrow-up" label={`Move ${d.name} up`} disabled={i === 0} onclick={() => move(d, -1)} />
              <IconButton icon="arrow-down" label={`Move ${d.name} down`} disabled={i === propertyDefs.length - 1} onclick={() => move(d, 1)} />
              <IconButton icon="trash" label={`Delete ${d.name}`} onclick={() => (doomed = d)} />
            </div>
          </div>
          {#if hasOptions(d.kind)}
            <div class="opts">
              {#each d.options as o, j (j)}
                <div class="opt">
                  <IconColorPicker colorOnly label={`Colour of option ${o.value}`} value={{ icon: 'icon:circle', color: o.color }} onchange={(v) => patchOption(d, j, { color: v.color ?? o.color })} />
                  <div class="es-name" onchange={(e) => patchOption(d, j, { value: (e.target as HTMLInputElement).value.trim() || o.value })}><TextInput label={`Option ${o.value}`} value={o.value} /></div>
                  <IconButton icon="close" label={`Remove option ${o.value}`} onclick={() => setOptions(d, d.options.filter((_, k) => k !== j))} />
                </div>
              {/each}
              <form class="opt opt-add" onsubmit={(e) => { e.preventDefault(); addOption(d); }}>
                <div class="es-name"><TextInput label={`New option of ${d.name}`} placeholder="New option" bind:value={() => optionDraft[d.key] ?? '', (v) => (optionDraft[d.key] = v)} /></div>
                <Button type="submit">Add option</Button>
              </form>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {:else}
    <EmptyState title="No properties yet" hint="Add a field such as effort or area below." />
  {/if}

  <form class="es-add" onsubmit={(e) => { e.preventDefault(); add(); }}>
    <div class="es-name"><TextInput label="New property name" placeholder="Name" bind:value={newName} /></div>
    <div class="es-name"><TextInput label="New property key" placeholder="key_name" bind:value={newKey} /></div>
    <Select label="New property kind" items={kindItems} bind:value={() => newKind, (k) => (newKind = k as PropertyKind)} />
    <Select label="New property applies to" items={appliesItems} bind:value={() => newApplies, (a) => (newApplies = a as PropertyAppliesTo)} />
    {#if newApplies !== 'Projects'}<Select label="New property scope" items={scopeItems} bind:value={() => String(newScope ?? 'null'), (id) => (newScope = id === 'null' ? null : Number(id))} />{/if}
    <Button variant="primary" type="submit">Add property</Button>
  </form>
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }} title={`Delete ${doomed?.name ?? 'property'}?`} description="Its value is removed from every issue. This cannot be undone.">
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" onclick={remove}>Delete property</Button>
  {/snippet}
</Dialog>

<style>
  .item { border-bottom: 1px solid var(--border); }
  .item .es-row { border-bottom: 0; }
  /* Name, key, kind, applies-to and scope no longer fit one line: wrap instead of squeezing the names to nothing. */
  .es-row, .es-add { flex-wrap: wrap; }
  .es-name { min-width: 8rem; }
  .opts { display: grid; gap: var(--s-2); padding: var(--s-2) var(--s-4) var(--s-5) var(--s-10); border-left: 3px solid var(--band-2); margin-left: var(--s-5); }
  .opt { display: flex; align-items: center; gap: var(--s-4); max-width: 420px; }
  .opt-add :global(.field) { border-color: var(--control); background: var(--sheet); }
</style>
