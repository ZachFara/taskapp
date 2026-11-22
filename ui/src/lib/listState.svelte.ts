/** What the list screen shares with commands: its visible ids in order (for movement) and the row being renamed. */
export const listState = $state<{ ids: number[]; editing: number | null }>({ ids: [], editing: null });
