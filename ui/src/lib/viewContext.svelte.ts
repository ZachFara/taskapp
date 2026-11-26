import type { Filter, ViewDef } from './bindings';

export type ActiveViewContext = {
  inView: boolean;
  filter: Filter | null;
  viewDef: ViewDef | null;
};

/**
 * Tracks the currently mounted issue view (saved view, My issues, or project issues)
 * so QuickAdd and issue creation inherit its filters and open created issues inside it.
 */
export const viewContext = $state<ActiveViewContext>({
  inView: false,
  filter: null,
  viewDef: null,
});

export function setViewContext(next: ActiveViewContext): void {
  viewContext.inView = next.inView;
  viewContext.filter = next.filter;
  viewContext.viewDef = next.viewDef;
}

export function clearViewContext(): void {
  viewContext.inView = false;
  viewContext.filter = null;
  viewContext.viewDef = null;
}
