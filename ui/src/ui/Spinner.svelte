<script lang="ts">
  let { size = 16, label = 'Loading' }: { size?: number; label?: string } = $props();
</script>

<svg class="spin" width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" role="status" aria-label={label}>
  <circle cx="8" cy="8" r="6" opacity="0.25" />
  <path d="M14 8a6 6 0 0 0-6-6" />
</svg>

<style>
  .spin { display: block; color: var(--muted); animation: cx-spin var(--dur-spin) linear infinite; }
  @keyframes cx-spin { to { transform: rotate(360deg); } }
</style>
