<script lang="ts">
  let {
    value = $bindable(''),
    placeholder,
    label,
    rows = 4,
    disabled = false,
    invalid = false,
  }: { value?: string; placeholder?: string; label: string; rows?: number; disabled?: boolean; invalid?: boolean } = $props();
</script>

<textarea class="field" {rows} {placeholder} {disabled} aria-label={label} aria-invalid={invalid || undefined} bind:value></textarea>

<style>
  .field {
    width: 100%;
    padding: var(--s-4) var(--s-5);
    border: 1px solid var(--control);
    border-radius: var(--r-ctl);
    background: var(--sheet);
    color: var(--ink);
    line-height: var(--lh-body);
    resize: none;
    transition: border-color var(--dur-1) var(--ease);
  }
  .field::placeholder { color: var(--muted); }
  .field:hover:not(:disabled) { border-color: var(--muted); }
  .field:focus-visible { outline-offset: 0; border-color: var(--focus); }
  .field:disabled { opacity: 0.55; background: var(--raised); }
  .field[aria-invalid='true'] { border-color: var(--signal); }
</style>
