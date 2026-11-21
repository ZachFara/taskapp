import { mount, tick, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import QuickAdd from './QuickAdd.svelte';

let app: ReturnType<typeof mount> | null = null;
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
  await tick();
};
const input = () => document.querySelector<HTMLInputElement>('input')!;
async function type(text: string) {
  input().value = text;
  input().dispatchEvent(new Event('input', { bubbles: true }));
  await settle();
}
const enter = async () => {
  input().dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
  await settle();
};

const onclose = vi.fn();
let api: ReturnType<typeof createMockApi>;
beforeEach(async () => {
  api = createMockApi();
  setApi(api);
  onclose.mockClear();
  app = mount(QuickAdd, { target: document.body, props: { onclose } });
  await settle();
});
afterEach(() => {
  unmount(app!);
  document.body.innerHTML = '';
});

describe('QuickAdd', () => {
  it('quickadd_preview_shows_parsed_chips', async () => {
    await type('Buy milk !2 #home');
    expect([...document.querySelectorAll('.chip')].map((c) => c.textContent)).toEqual(['Priority 2', '#home']);
  });

  it('quickadd_enter_creates_and_closes', async () => {
    const create = vi.spyOn(api.issues, 'createFromQuickadd');
    await type('Buy milk');
    await enter();
    expect(create).toHaveBeenCalledWith('Buy milk');
    expect(onclose).toHaveBeenCalled();
  });

  it('whitespace_only_does_not_create', async () => {
    const create = vi.spyOn(api.issues, 'createFromQuickadd');
    await type('   ');
    await enter();
    expect(create).not.toHaveBeenCalled();
    expect(onclose).not.toHaveBeenCalled();
  });

  it('create_error_keeps_text_and_shows_message', async () => {
    api.issues.createFromQuickadd = async () => {
      throw { kind: 'Invalid', message: 'title too long' };
    };
    await type('Buy milk');
    await enter();
    expect(input().value).toBe('Buy milk');
    expect(document.body.textContent).toContain('title too long');
    expect(onclose).not.toHaveBeenCalled();
  });
});
