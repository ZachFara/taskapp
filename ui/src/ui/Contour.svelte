<script lang="ts">
  // Topographic rings: closed wobbling curves around one summit, drawn in currentColor.
  let { rings = 9, size = 220 }: { rings?: number; size?: number } = $props();

  function ring(i: number): string {
    const pts = 72;
    const r0 = 14 + i * 11.5;
    let d = '';
    for (let k = 0; k < pts; k++) {
      const a = (k / pts) * Math.PI * 2;
      // Fixed harmonics, phase shifted per ring: deterministic, so the art never jitters between renders.
      const wob = 1 + 0.09 * Math.sin(2 * a + i * 0.7) + 0.05 * Math.sin(3 * a + 1.3 + i * 0.4) + 0.03 * Math.sin(5 * a + i);
      const x = 110 + r0 * wob * Math.cos(a) * 1.12;
      const y = 110 + r0 * wob * Math.sin(a) * 0.9;
      d += `${k ? 'L' : 'M'}${x.toFixed(1)} ${y.toFixed(1)}`;
    }
    return d + 'Z';
  }
  const paths = $derived(Array.from({ length: rings }, (_, i) => ring(i)));
</script>

<svg class="contour" width={size} height={size} viewBox="0 0 220 220" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" aria-hidden="true">
  {#each paths as d, i (i)}<path {d} opacity={1 - i * 0.07} />{/each}
</svg>

<style>
  .contour { display: block; }
</style>
