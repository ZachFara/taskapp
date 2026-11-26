import { onMount } from 'svelte';

export function onRowsChanged(load: () => void) {
  onMount(() => {
    addEventListener('history:wrote', load);
    addEventListener('rows:changed', load);
    return () => {
      removeEventListener('history:wrote', load);
      removeEventListener('rows:changed', load);
    };
  });
}
