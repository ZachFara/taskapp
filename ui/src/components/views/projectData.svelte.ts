import { onMount, untrack } from 'svelte';
import { api } from '../../lib/api';
import type { ProjectRow, ViewDef } from '../../lib/bindings';
import { emptyPCtx, loadPCtx, type PCtx } from '../../lib/projectViews';

/** The projects a project view shows and what draws them, reloaded when the definition or the data changes. Call while a component initialises. */
export function projectData(getDef: () => ViewDef) {
  const data = $state<{ rows: ProjectRow[]; ctx: PCtx; loaded: boolean }>({ rows: [], ctx: emptyPCtx, loaded: false });
  let queryVersion = 0;
  async function load() {
    const mine = ++queryVersion;
    const def = $state.snapshot(getDef()) as ViewDef;
    const [rows, ctx] = await Promise.all([
      api.views.queryProjects(def),
      loadPCtx({ statuses: api.project.statusesList, labels: api.labels.list, defs: api.properties.listDefs }),
    ]);
    if (mine === queryVersion) [data.rows, data.ctx, data.loaded] = [rows, ctx, true];
  }
  $effect(() => {
    getDef();
    untrack(load);
  });
  onMount(() => {
    for (const e of ['rows:changed', 'history:wrote']) addEventListener(e, load);
    return () => ['rows:changed', 'history:wrote'].forEach((e) => removeEventListener(e, load));
  });
  return data;
}
