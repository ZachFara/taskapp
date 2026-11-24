import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { createMockApi } from '../lib/api.mock';
import { setApi } from '../lib/api';
import { hasImageTarget } from '../lib/images';
import TextArea from './TextArea.svelte';

const PNG = new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10, 1, 2, 3]);
let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});

const paste = (ta: HTMLTextAreaElement, files: File[], text = '') => {
  const e = new Event('paste', { bubbles: true, cancelable: true }) as ClipboardEvent;
  Object.defineProperty(e, 'clipboardData', { value: { files, getData: () => text } });
  ta.dispatchEvent(e);
  return e;
};
const settle = () => new Promise((r) => setTimeout(r, 20));

function setup() {
  setApi(createMockApi());
  app = mount(TextArea, { target: document.body, props: { label: 'Body', images: true, value: 'before  after' } });
  const ta = document.querySelector('textarea')!;
  const props = { get value() { return ta.value; } };
  flushSync();
  ta.focus();
  ta.setSelectionRange(7, 7);
  return { ta, props };
}

describe('TextArea images', () => {
  it('uploads a pasted image and inserts its markdown at the cursor', async () => {
    const { ta, props } = setup();
    const e = paste(ta, [new File([PNG], 'shot.png', { type: 'image/png' })]);
    expect(e.defaultPrevented).toBe(true);
    await settle();
    flushSync();
    expect(props.value).toBe('before ![shot](attachment:1) after');
    expect(hasImageTarget()).toBe(true);
  });

  it('leaves pasted text alone', async () => {
    const { ta, props } = setup();
    const e = paste(ta, [], 'plain');
    expect(e.defaultPrevented).toBe(false);
    await settle();
    expect(props.value).toBe('before  after');
  });

  it('shows an inline error for a wrong type and inserts nothing', async () => {
    const { ta, props } = setup();
    paste(ta, [new File(['<svg/>'], 'a.svg', { type: 'image/svg+xml' })]);
    await settle();
    flushSync();
    expect(props.value).toBe('before  after');
    expect(document.querySelector('[role=alert]')!.textContent).toMatch(/PNG, JPEG, GIF and WebP/);
  });

  it('refuses a file whose bytes are not the type it claims', async () => {
    const { ta, props } = setup();
    paste(ta, [new File(['not an image'], 'a.png', { type: 'image/png' })]);
    await settle();
    flushSync();
    expect(props.value).toBe('before  after');
    expect(document.querySelector('[role=alert]')).not.toBeNull();
  });
});
