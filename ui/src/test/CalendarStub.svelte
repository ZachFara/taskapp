<script lang="ts">
  // Test double for the calendar component: the library measures the DOM, which jsdom lacks.
  // It records the options the view passes and forwards the two instance methods the view calls.
  import { vi } from 'vitest';

  let { options, plugins }: { options: any; plugins: any } = $props();
  const state = { options, plugins, setOption: vi.fn(), dateFromPoint: vi.fn((): null => null) };
  (globalThis as any).__calStub = state;

  export const setOption = (...args: unknown[]) => state.setOption(...args);
  export const dateFromPoint = (x: number, y: number) => state.dateFromPoint(x, y);
</script>
