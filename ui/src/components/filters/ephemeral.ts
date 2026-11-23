import type { Filter } from '../../lib/bindings';

/** The filter a view shows: its saved filter and the temporary quick filter, both required. Only the saved one is stored. */
export function combineFilters(saved: Filter | null, quick: Filter | null): Filter | null {
  const parts = [saved, quick].filter((f): f is Filter => f !== null);
  return parts.length === 0 ? null : parts.length === 1 ? parts[0] : { And: parts };
}
