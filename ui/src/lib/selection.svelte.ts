/** Focused row and selected rows of the current view; views pass their ordered ids in. */
export function createSelection() {
  let focused = $state<number | null>(null);
  let selected = $state<number[]>([]);
  let anchor: number | null = null;
  return {
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
