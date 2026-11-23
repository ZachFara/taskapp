import { api } from './api';
import { refreshAll } from './stores.svelte';
import type { PinKind } from './bindings';
import { templates } from '../components/display/templatesApi';

/** What a row action acts on. `id` is the entity's own id (a project's id, not its slug). */
export type Kind = 'view' | 'project' | 'cycle' | 'label' | 'template';
export type Target = { kind: Kind; id: number; name: string };
type Change = { name?: string; icon?: string | null; color?: string | null };

const PIN: Partial<Record<Kind, PinKind>> = { view: 'View', project: 'Project', cycle: 'Cycle' };

/** Every row action ends here: pages and the sidebar reload on `rows:changed`. */
const changed = () => {
  window.dispatchEvent(new CustomEvent('rows:changed'));
  void refreshAll();
};

/** Saves a name, icon or colour, keeping what the change does not name. */
export async function patch(t: Target, c: Change) {
  if (t.kind === 'view') {
    const v = (await api.views.list()).find((x) => x.id === t.id);
    if (v) await api.views.save(v.id, c.name ?? v.name, v.def, 'icon' in c ? c.icon! : v.icon, 'color' in c ? c.color! : v.color);
  } else if (t.kind === 'project') {
    const p = (await api.projects.list()).find((x) => x.id === t.id);
    if (p) await api.projects.update({ ...p, ...c });
  } else if (t.kind === 'cycle') {
    const x = (await api.cycles.list()).find((y) => y.id === t.id);
    if (x) await api.cycles.update({ ...x, ...c });
  } else if (t.kind === 'label') {
    const l = (await api.labels.list()).find((y) => y.id === t.id);
    if (l) await api.labels.update({ ...l, ...c });
  } else {
    const tp = (await api.templates.list()).find((y) => y.id === t.id);
    if (tp) await api.templates.update(tp.id, { name: c.name ?? tp.name, slug: tp.slug, icon: 'icon' in c ? c.icon! : tp.icon, color: 'color' in c ? c.color! : tp.color, body: tp.body });
  }
  changed();
}

export async function remove(t: Target) {
  if (t.kind === 'view') await api.views.delete(t.id);
  else if (t.kind === 'project') await api.projects.delete(t.id);
  else if (t.kind === 'cycle') await api.cycles.delete(t.id);
  else if (t.kind === 'label') await api.labels.delete(t.id);
  else await api.templates.delete(t.id);
  changed();
}

/** Copies a saved view under "<name> copy", as the view toolbar does. */
export async function duplicate(t: Target) {
  const v = (await api.views.list()).find((x) => x.id === t.id);
  if (v) await api.views.save(null, `${v.name} copy`, v.def, v.icon, v.color);
  changed();
}

/** Pins or unpins the entity on the sidebar; a project is pinned by its slug. */
export async function togglePin(t: Target) {
  const kind = PIN[t.kind];
  if (!kind) return;
  const target = t.kind === 'project' ? (await api.projects.list()).find((p) => p.id === t.id)?.slug : String(t.id);
  if (!target) return;
  const have = (await api.pins.listResolved()).find((p) => p.pin.kind === kind && p.pin.target === target);
  if (have) await api.pins.unpin(have.pin.id);
  else await api.pins.pin(kind, target, t.name);
  changed();
}
