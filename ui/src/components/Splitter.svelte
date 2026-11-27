<script lang="ts">
  // A pane edge the pointer drags. Keys and the reset live in the command registry, so this only reports widths.
  let { label, clamp, fromPointer, onchange, onreset }: {
    label: string; clamp: (w: number) => number; fromPointer: (x: number) => number;
    onchange: (w: number) => void; onreset: () => void;
  } = $props();

  function drag(e: PointerEvent) {
    e.preventDefault();
    const move = (m: PointerEvent) => onchange(clamp(fromPointer(m.clientX)));
    const stop = () => {
      removeEventListener('pointermove', move);
      removeEventListener('pointerup', stop);
    };
    addEventListener('pointermove', move);
    addEventListener('pointerup', stop);
  }
</script>

<div class="split" role="separator" aria-orientation="vertical" aria-label={label} onpointerdown={drag} ondblclick={onreset}></div>

<style>
  .split { position: absolute; inset: 0; cursor: col-resize; }
  .split:hover, .split:active { background: var(--control); opacity: 0.5; }
</style>
