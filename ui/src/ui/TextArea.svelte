<script lang="ts">
  import { clearImageTarget, errorText, setImageTarget, uploadImage } from '../lib/images';
  import Button from './Button.svelte';

  let {
    value = $bindable(''),
    placeholder,
    label,
    rows = 4,
    disabled = false,
    invalid = false,
    images = false,
  }: { value?: string; placeholder?: string; label: string; rows?: number; disabled?: boolean; invalid?: boolean; images?: boolean } = $props();

  // `images`: a pasted or dropped image file, or one chosen with the button, is stored and its markdown inserted at the cursor.
  let ta: HTMLTextAreaElement | undefined = $state();
  let picker: HTMLInputElement | undefined = $state();
  let error = $state('');

  async function attach(files: File[]) {
    error = '';
    let at = ta?.selectionStart ?? value.length;
    const end = ta?.selectionEnd ?? at;
    try {
      const md = (await Promise.all(files.map(uploadImage))).join('\n');
      value = value.slice(0, at) + md + value.slice(end);
      at += md.length;
      queueMicrotask(() => { ta?.focus(); ta?.setSelectionRange(at, at); });
    } catch (e) {
      error = errorText(e);
    }
  }
  const paste = (e: ClipboardEvent) => {
    const files = [...(e.clipboardData?.files ?? [])].filter((f) => f.type.startsWith('image/'));
    if (!images || !files.length) return;
    e.preventDefault();
    void attach(files);
  };
  const over = (e: DragEvent) => { if (images && e.dataTransfer?.types.includes('Files')) e.preventDefault(); };
  const drop = (e: DragEvent) => {
    const files = [...(e.dataTransfer?.files ?? [])];
    if (!images || !files.length) return;
    e.preventDefault();
    void attach(files);
  };
  const chosen = () => {
    const files = [...(picker?.files ?? [])];
    if (picker) picker.value = '';
    if (files.length) void attach(files);
  };
  $effect(() => {
    const el = ta;
    if (!images || !el) return;
    const mine = { el, pick: () => picker?.click() };
    el.addEventListener('focus', () => setImageTarget(mine));
    return () => clearImageTarget(el);
  });
</script>

{#snippet field()}
  <textarea bind:this={ta} class="field" {rows} {placeholder} {disabled} aria-label={label} aria-invalid={invalid || undefined} bind:value onpaste={paste} ondragover={over} ondrop={drop}></textarea>
{/snippet}

{#if images}
  <div class="wrap">
    {@render field()}
    <div class="img">
      <Button icon="plus" variant="quiet" {disabled} onclick={() => picker?.click()}>Attach image</Button>
      <input bind:this={picker} type="file" accept="image/png,image/jpeg,image/gif,image/webp" multiple hidden aria-label="Choose image" onchange={chosen} />
      {#if error}<span class="err" role="alert">{error}</span>{/if}
    </div>
  </div>
{:else}
  {@render field()}
{/if}

<style>
  .wrap { display: grid; gap: var(--s-2); }
  .img { display: flex; align-items: center; gap: var(--s-4); }
  .err { color: var(--signal); font-size: var(--fs-xs); }
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
