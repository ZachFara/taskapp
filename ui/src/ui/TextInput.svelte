<script lang="ts">
  let {
    value = $bindable(''),
    placeholder,
    label,
    type = 'text',
    disabled = false,
    invalid = false,
    oninput,
  }: {
    value?: string;
    placeholder?: string;
    label: string;
    type?: 'text' | 'search' | 'number' | 'date' | 'password';
    disabled?: boolean;
    invalid?: boolean;
    oninput?: (e: Event) => void;
  } = $props();
</script>

<input class="field" {type} {placeholder} {disabled} aria-label={label} aria-invalid={invalid || undefined} bind:value {oninput} />

<style>
  .field {
    height: var(--ctl-h);
    width: 100%;
    padding: 0 var(--s-5);
    border: 1px solid var(--control);
    border-radius: var(--r-ctl);
    background: var(--sheet);
    color: var(--ink);
    transition: border-color var(--dur-1) var(--ease);
  }
  .field::placeholder { color: var(--muted); }
  .field:hover:not(:disabled) { border-color: var(--muted); }
  .field:focus-visible { outline-offset: 0; border-color: var(--focus); }
  .field:disabled { opacity: 0.55; background: var(--raised); }
  .field[aria-invalid='true'] { border-color: var(--signal); }
</style>
