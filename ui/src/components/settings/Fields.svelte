<script lang="ts">
  import SectionHead from './SectionHead.svelte';
  import { api } from '../../lib/api';
  import { fieldList, loadFields } from '../../lib/fields.svelte';
  import { FIELD_LABELS, type IssueField } from '../../lib/issueFields';
  import IconButton from '../../ui/IconButton.svelte';
  import Switch from '../../ui/Switch.svelte';
  import './entities.css';

  let dragFrom = $state<number | null>(null);
  let overAt = $state<number | null>(null);

  const list = $derived(fieldList());
  // Points also follows the estimation setting (General), so a switch here cannot show it then.
  const save = async (next: IssueField[]) => {
    await api.settings.set('issue_fields', $state.snapshot(next));
    await loadFields();
  };
  const set = (i: number, on: boolean) => save(list.map((f, j) => (j === i ? { ...f, on } : f)));
  function move(from: number, to: number) {
    if (to < 0 || to >= list.length || to === from) return;
    const next = [...list];
    next.splice(to, 0, ...next.splice(from, 1));
    void save(next);
  }
</script>

<div class="es">
  <SectionHead lede="Choose which built-in fields issues have, and in what order the issue panel lists them. A field that is off is hidden everywhere, and the issues keep their values. Status and title are always on." />
  <ul class="es-list" aria-label="Issue fields">
    {#each list as f, i (f.field)}
      <li
        class="es-row"
        draggable="true"
        data-over={overAt === i || undefined}
        ondragstart={() => (dragFrom = i)}
        ondragover={(e) => { if (dragFrom !== null) { e.preventDefault(); overAt = i; } }}
        ondrop={(e) => { e.preventDefault(); if (dragFrom !== null) move(dragFrom, i); dragFrom = overAt = null; }}
        ondragend={() => (dragFrom = overAt = null)}
      >
        <span class="es-name">{FIELD_LABELS[f.field]}</span>
        <div class="es-tools">
          <IconButton icon="arrow-up" label={`Move ${FIELD_LABELS[f.field]} up`} disabled={i === 0} onclick={() => move(i, i - 1)} />
          <IconButton icon="arrow-down" label={`Move ${FIELD_LABELS[f.field]} down`} disabled={i === list.length - 1} onclick={() => move(i, i + 1)} />
        </div>
        <Switch label={`Show ${FIELD_LABELS[f.field]}`} bind:checked={() => f.on, (v) => set(i, v)} />
      </li>
    {/each}
  </ul>
</div>
