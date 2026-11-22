<script lang="ts">
  import { ICONS } from './icons';

  let { name, size = 16, label }: { name: string; size?: number; label?: string } = $props();
  // An id the set does not know (hand-edited data) shows as a plain circle rather than nothing.
  const body = $derived(ICONS[name] ?? ICONS.circle);
</script>

<svg
  class="icon"
  width={size}
  height={size}
  viewBox="0 0 16 16"
  fill="none"
  stroke="currentColor"
  stroke-width="1.5"
  stroke-linecap="round"
  stroke-linejoin="round"
  role={label ? 'img' : undefined}
  aria-label={label}
  aria-hidden={label ? undefined : 'true'}
  data-icon={name}
>
  {@html body}
</svg>

<style>
  .icon { flex: none; display: block; }
</style>
