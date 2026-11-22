<script lang="ts">
  let { width = '100%', height = 'var(--s-7)' }: { width?: string; height?: string } = $props();
</script>

<span class="skel" style:width style:height aria-hidden="true"></span>

<style>
  .skel {
    display: block;
    border-radius: var(--r-sm);
    background: linear-gradient(90deg, var(--active), var(--raised), var(--active));
    background-size: 200% 100%;
    animation: cx-shimmer var(--dur-shimmer) var(--ease) infinite;
  }
  @keyframes cx-shimmer { from { background-position: 100% 0; } to { background-position: -100% 0; } }
</style>
