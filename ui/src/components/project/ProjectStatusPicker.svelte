<script lang="ts">
  import { api } from '../../lib/api';
  import type { Project, ProjectStatus } from '../../lib/bindings';
  import { glyphCategory } from '../../lib/projectFields';
  import Popover from '../../ui/Popover.svelte';
  import StatusGlyph from '../../ui/StatusGlyph.svelte';
  import OptionList from '../detail/OptionList.svelte';

  let { project, statuses, iconOnly = false, onsaved }: {
    project: Project; statuses: ProjectStatus[]; iconOnly?: boolean; onsaved?: () => void;
  } = $props();
  let open = $state(false);
  let saving = $state(false);
  let error = $state('');
  const status = $derived(statuses.find((s) => s.id === project.status_id));

  async function pick(id: string | number | null) {
    open = false;
    if (id === null || Number(id) === project.status_id || saving) return;
    saving = true;
    error = '';
    try {
      const current = (await api.projects.list()).find((p) => p.id === project.id);
      if (!current) throw new Error('Project no longer exists');
      await api.projects.update({ ...current, status_id: Number(id) });
      onsaved?.();
    } catch (err) {
      error = `Could not change status: ${(err as { message?: string }).message ?? String(err)}`;
    } finally {
      saving = false;
    }
  }
</script>

<span class="control" role="presentation" onclick={(e) => e.stopPropagation()}>
  <Popover bind:open>
    {#snippet trigger(props)}
      <button {...props} type="button" class="trigger" disabled={saving} aria-label={`Change status of ${project.name}`}>
        {#if status}<StatusGlyph category={glyphCategory[status.category]} icon={status.icon} color={status.color} size={14} />{/if}
        {#if !iconOnly}{status?.name ?? 'Set status'}{/if}
      </button>
    {/snippet}
    <OptionList label="Project status" value={project.status_id} options={statuses} onpick={pick}>
      {#snippet lead(o)}
        {@const s = statuses.find((item) => item.id === o.id)}
        {#if s}<span aria-hidden="true"><StatusGlyph category={glyphCategory[s.category]} icon={s.icon} color={s.color} /></span>{/if}
      {/snippet}
    </OptionList>
  </Popover>
  {#if error}<span class="error" role="alert">{error}</span>{/if}
</span>

<style>
  .control { display: inline-flex; align-items: center; gap: var(--s-2); }
  .trigger { display: inline-flex; align-items: center; gap: var(--s-3); padding: var(--s-1); border: 0; border-radius: var(--r-sm); background: transparent; color: inherit; font: inherit; cursor: pointer; }
  .trigger:hover, .trigger:focus-visible { background: var(--active); }
  .trigger:disabled { cursor: wait; }
  .error { color: var(--signal); }
</style>
