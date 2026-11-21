import { svelte } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig(({ mode }) => ({
  plugins: [svelte()],
  // Svelte resolves to its server build under vitest unless told otherwise.
  resolve: mode === 'test' ? { conditions: ['browser'] } : {},
  test: { environment: 'jsdom', include: ['src/**/*.test.ts', 'harness/**/*.test.ts'] },
}));
