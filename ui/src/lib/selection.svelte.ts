import type { Layout } from './commands/registry';

/** What the list screen shares with commands: its visible ids in order (for movement) and the row being renamed. */
export const listState = $state<{ ids: number[]; editing: number | null }>({ ids: [], editing: null });

/** Focused row and selected rows of the current view; views pass their ordered ids in. */
export function createSelection() {
  let focused = $state<number | null>(null);
  let selected = $state<number[]>([]);
  let anchor: number | null = null;
  let columns = $state<number[][]>([]);
  let peeked = $state<number | null>(null);
  let layout = $state<Layout | undefined>();
  return {
    /** The layout the open view shows; the view sets it so keys can be scoped to it (undefined off a view). */
    get layout() {
      return layout;
    },
    setLayout(l: Layout | undefined) {
      layout = l;
    },
    /** Ordered visible ids per column (a list has one column); the views publish them. */
    get columns() {
      return columns;
    },
    setColumns(cols: number[][]) {
      columns = cols;
    },
    /** Moves focus to the same row index in the column `delta` away, clamped to that column's length. */
    moveColumn(delta: number) {
      const cols = columns.filter((c) => c.length);
      if (!cols.length) return;
      if (focused === null) return void (focused = cols[0][0]);
      const at = Math.max(0, cols.findIndex((c) => c.includes(focused!)));
      const to = Math.min(cols.length - 1, Math.max(0, at + delta));
      focused = cols[to][Math.min(Math.max(0, cols[at].indexOf(focused)), cols[to].length - 1)];
    },
    get peeked() {
      return peeked;
    },
    /** Peeks the focused row, or closes the peek when it is already open. */
    togglePeek() {
      peeked = peeked === focused ? null : focused;
    },
    get focused() {
      return focused;
    },
    get selected() {
      return selected;
    },
    focus(id: number | null) {
      focused = id;
    },
    /** Moves focus by `delta` within `ids`, clamped; with `extend` the rows from the anchor to the new focus are selected. */
    move(ids: number[], delta: number, extend = false) {
      if (!ids.length) return;
      const at = focused === null ? -1 : ids.indexOf(focused);
      const to = Math.min(ids.length - 1, Math.max(0, at + delta));
      if (extend) {
        anchor ??= focused ?? ids[to];
        const a = ids.indexOf(anchor);
        selected = ids.slice(Math.min(a, to), Math.max(a, to) + 1);
      }
      focused = ids[to];
    },
    toggle(id = focused) {
      if (id === null) return;
      selected = selected.includes(id) ? selected.filter((x) => x !== id) : [...selected, id];
      anchor = null;
    },
    selectAll(ids: number[]) {
      selected = [...ids];
    },
    clear() {
      selected = [];
      anchor = null;
    },
  };
}

export const selection = createSelection();
