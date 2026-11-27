<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { onMount } from 'svelte';
  import { api } from '../../lib/api';
  import { statuses, issues, refreshAll } from '../../lib/stores.svelte';
  import type { FlowKind, Status, StatusCategory } from '../../lib/bindings';
  import IconColorPicker from '../pickers/IconColorPicker.svelte';
  import Button from '../../ui/Button.svelte';
  import Dialog from '../../ui/Dialog.svelte';
  import GroupHeader from '../../ui/GroupHeader.svelte';
  import IconButton from '../../ui/IconButton.svelte';
  import Select from '../../ui/Select.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import TextInput from '../../ui/TextInput.svelte';
  import { STATUS_CATEGORIES as categories } from '../../lib/statusOrder';
  import './entities.css';

  const kinds: { value: FlowKind; label: string }[] = [
    { value: 'Active', label: 'Active: work is happening' },
    { value: 'Waiting', label: 'Waiting: blocked on someone or something' },
    { value: 'Idle', label: 'Idle: not being worked' },
  ];
  let adding = $state<StatusCategory | null>(null);
  let name = $state('');
  let color = $state<string | null>(null);
  let icon = $state<string | null>(null);
  let collapsed = $state<StatusCategory[]>([]);
  let error = $state('');
  let doomed = $state<Status | null>(null);
  let target = $state<number | null>(null);
  let dragId = $state<number | null>(null);
  let overId = $state<number | null>(null);

  const inCategory = (c: StatusCategory) => statuses.filter((s) => s.category === c);
  const users = (s: Status) => issues.filter((i) => i.status_id === s.id).length;
  const others = $derived(statuses.filter((s) => s.id !== doomed?.id));

  // The status a new issue gets when none is given (quick add, templates, imports); unset means the first status.
  let start = $state('');
  onMount(async () => {
    await refreshAll();
    const v = await api.settings.get('default_status');
    start = typeof v === 'number' && statuses.some((s) => s.id === v) ? String(v) : String(statuses[0]?.id ?? '');
  });
  const setStart = (v: string) => {
    start = v;
    void write(() => api.settings.set('default_status', Number(v)));
  };

  // Every write goes through here so a refused write shows its message and the store stays in step.
  async function write(action: () => Promise<unknown>) {
    error = '';
    try {
      await action();
      await refreshAll();
    } catch (e) {
      error = (e as { message?: string }).message ?? String(e);
    }
  }

  const add = () => write(async () => {
    await api.statuses.create(name.trim(), adding!, color ?? 'gray', icon);
    name = '';
    icon = color = null;
  });

  const rename = (s: Status, e: Event) => {
    const next = (e.target as HTMLInputElement).value.trim();
    if (next && next !== s.name) void write(() => api.statuses.update(s.id, next, null, null, null));
  };

  const flow = (s: Status, definition: string | null, kind: FlowKind | null, days: number | null) =>
    write(() => api.statuses.setFlow(s.id, definition, kind, days));

  // One reorder call hands the category the new order; the backend keeps it one undo step.
  function move(s: Status, to: number) {
    const order = inCategory(s.category);
    const from = order.findIndex((x) => x.id === s.id);
    if (to < 0 || to >= order.length || to === from) return;
    order.splice(to, 0, ...order.splice(from, 1));
    void write(() => api.statuses.reorder(order.map((x) => x.id)));
  }

  function drop(s: Status) {
    const from = statuses.find((x) => x.id === dragId);
    dragId = overId = null;
    if (from && from.category === s.category) move(from, inCategory(s.category).findIndex((x) => x.id === s.id));
  }

  const confirmDelete = () => write(async () => {
    await api.statuses.delete(doomed!.id, target!);
    doomed = null;
  });
</script>

<div class="es">
  <SectionHead lede="The stages work moves through. A category says where a status sits in the lifecycle. Its kind says how time there counts for flow efficiency: active, waiting or idle. A nudge flags issues that sit in a status too long." />
  <label class="start">
    <span>New issues start in</span>
    <Select label="New issues start in" items={statuses.map((s) => ({ value: String(s.id), label: s.name, glyph: { category: s.category, icon: s.icon, color: s.color } }))} bind:value={() => start, setStart} />
  </label>

  {#each categories as c (c)}
    <section aria-label={c}>
      <GroupHeader
        name={c}
        category={c}
        count={inCategory(c).length}
        collapsed={collapsed.includes(c)}
        ontoggle={() => (collapsed = collapsed.includes(c) ? collapsed.filter((x) => x !== c) : [...collapsed, c])}
        onadd={() => (adding = c)}
      />
      {#if !collapsed.includes(c)}
        <ul class="es-list">
          {#each inCategory(c) as s, i (s.id)}
            <li
              class="es-row"
              draggable="true"
              data-over={overId === s.id || undefined}
              ondragstart={() => (dragId = s.id)}
              ondragover={(e) => { if (dragId !== null) { e.preventDefault(); overId = s.id; } }}
              ondrop={(e) => { e.preventDefault(); drop(s); }}
              ondragend={() => (dragId = overId = null)}
            >
              <StatusGlyph category={s.category} icon={s.icon} color={s.color} />
              <IconColorPicker
                label={`Icon and colour of ${s.name}`}
                value={{ icon: s.icon, color: s.color }}
                onchange={(v) => write(() => api.statuses.update(s.id, null, v.color, null, v.icon))}
              />
              <div class="es-name" onchange={(e) => rename(s, e)}><TextInput label={`Name of ${s.name}`} value={s.name} /></div>
              <span class="es-meta">{users(s)} {users(s) === 1 ? 'issue' : 'issues'}</span>
              <div class="es-tools">
                <IconButton icon="arrow-up" label={`Move ${s.name} up`} disabled={i === 0} onclick={() => move(s, i - 1)} />
                <IconButton icon="arrow-down" label={`Move ${s.name} down`} disabled={i === inCategory(c).length - 1} onclick={() => move(s, i + 1)} />
                <IconButton icon="trash" label={`Delete ${s.name}`} onclick={() => { doomed = s; target = null; }} />
              </div>
              <div class="es-flow">
                <div class="es-def"><TextInput label={`Definition of ${s.name}`} placeholder="What this status means" value={s.definition} onchange={(v) => v.trim() !== s.definition && flow(s, v, null, null)} /></div>
                <div class="es-kind">
                  <Select label={`Flow kind of ${s.name}`} items={kinds} bind:value={() => s.flow_kind, (v) => flow(s, null, v as FlowKind, null)} />
                </div>
                <span class="es-hint">Nudge after</span>
                <div class="es-days"><TextInput label={`Nudge days of ${s.name}`} type="number" min="0" placeholder="Never" value={s.nudge_days === null ? '' : String(s.nudge_days)} onchange={(v) => flow(s, null, null, Math.max(0, Math.trunc(Number(v)) || 0))} /></div>
                <span class="es-hint">days</span>
              </div>
            </li>
          {/each}
        </ul>
        {#if adding === c}
          <form class="es-add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
            <IconColorPicker label="New status icon and colour" value={{ icon, color }} onchange={(v) => ({ icon, color } = v)} />
            <div class="es-name"><TextInput label="New status name" placeholder={`New ${c.toLowerCase()} status`} bind:value={name} /></div>
            <Button variant="primary" type="submit">Add status</Button>
          </form>
        {/if}
      {/if}
    </section>
  {/each}
  {#if error}<p class="es-err" role="alert">{error}</p>{/if}
</div>

<Dialog
  bind:open={() => doomed !== null, (v) => { if (!v) doomed = null; }}
  title={`Delete ${doomed?.name ?? 'status'}?`}
  description={`Its ${doomed ? users(doomed) : 0} issues need a new status first. Choose where they go.`}
>
  {#snippet children()}
    <div class="es-pick" role="group" aria-label="Move issues to">
      {#each others as o (o.id)}
        <button type="button" aria-label={`Move issues to ${o.name}`} aria-pressed={target === o.id} onclick={() => (target = o.id)}>{o.name}</button>
      {/each}
    </div>
  {/snippet}
  {#snippet actions()}
    <Button onclick={() => (doomed = null)}>Cancel</Button>
    <Button variant="danger" disabled={target === null} onclick={confirmDelete}>Delete status</Button>
  {/snippet}
</Dialog>

<style>
  .start { display: flex; align-items: center; gap: var(--s-5); max-width: 28rem; margin-bottom: var(--s-6); }
  .es-row { flex-wrap: wrap; }
  .es-flow { flex-basis: 100%; display: grid; grid-template-columns: minmax(0, 1fr) 18rem auto 6rem auto; align-items: center; gap: var(--s-4); padding-bottom: var(--s-2); }
  .es-hint { color: var(--muted); font-size: var(--fs-sm); white-space: nowrap; }
  .es-kind :global(.cx-select) { width: 100%; }
  .start span { flex: none; font-weight: 600; }
</style>
