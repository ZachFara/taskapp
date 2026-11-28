import { api } from './api';
import type { ViewDef } from './bindings';

export const archivedDef: ViewDef = {
  filter: { Cond: { field: { Fixed: 'Archived' }, op: 'Eq', value: true } },
  sorts: [], group_by: null, layout: 'List', columns: [], include_archived: true,
};

/** What the confirmation dialog is asking about; `n` is 0 while it is closed. `empty`: every archived issue. */
export const deleteAsk = $state<{ ids: number[]; empty: boolean; n: number }>({ ids: [], empty: false, n: 0 });

export const askDelete = (ids: number[]) => void Object.assign(deleteAsk, { ids, empty: false, n: ids.length });

export async function askEmpty() {
  const rows = await api.views.query(archivedDef);
  Object.assign(deleteAsk, { ids: [], empty: true, n: rows.length });
}
