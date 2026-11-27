import { flushSync, mount, unmount, createRawSnippet } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import Button from './Button.svelte';
import Checkbox from './Checkbox.svelte';
import ContextMenu from './ContextMenu.svelte';
import Dialog from './Dialog.svelte';
import GroupHeader from './GroupHeader.svelte';
import IconButton from './IconButton.svelte';
import Kbd from './Kbd.svelte';
import ListRow from './ListRow.svelte';
import PageHeader from './PageHeader.svelte';
import TextInput from './TextInput.svelte';
import { setTooltips } from '../lib/tooltips.svelte';

let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
  document.documentElement.style.removeProperty('--delay-tooltip');
});
const html = (s: string) => createRawSnippet(() => ({ render: () => `<span>${s}</span>` }));
const render = (C: any, props: Record<string, unknown> = {}) => {
  app = mount(C, { target: document.body, props });
  flushSync();
};
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));
const key = (el: Element, k: string) => {
  el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }));
  flushSync();
};

describe('Kbd', () => {
  it('renders a repeated key twice', () => {
    render(Kbd, { keys: ['g', 'g'] });
    expect(document.querySelectorAll('kbd').length).toBe(2);
  });
});

describe('Dialog', () => {
  it('takes a size and may be bare (no title, no footer)', async () => {
    render(Dialog, { open: true, size: 'lg', children: html('body') });
    await wait(0);
    const d = document.querySelector<HTMLElement>('.cx-dialog')!;
    expect(d.dataset.size).toBe('lg');
    expect(document.querySelector('.cx-dialog-title')).toBeNull();
    expect(document.querySelector('.foot')).toBeNull();
  });
  it('defaults to md-less sm and keeps the Close footer when titled', async () => {
    render(Dialog, { open: true, title: 'T' });
    await wait(0);
    expect(document.querySelector<HTMLElement>('.cx-dialog')!.dataset.size).toBe('sm');
    expect(document.querySelector('.foot')).not.toBeNull();
  });
});

describe('Tooltip delay token', () => {
  it('reads --delay-tooltip from the root', async () => {
    setTooltips(true);
    document.documentElement.style.setProperty('--delay-tooltip', '10ms');
    render(IconButton, { icon: 'search', label: 'Search' });
    const b = document.querySelector('button')!;
    b.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, pointerType: 'mouse' }));
    await wait(150);
    expect(document.querySelector('.cx-tip')).not.toBeNull();
  });
});

describe('PageHeader', () => {
  const props = (extra = {}) => {
    const calls: string[] = [];
    render(PageHeader, { title: 'Old', editable: true, onrename: (v: string) => calls.push(v), ...extra });
    return calls;
  };
  const input = () => document.querySelector<HTMLInputElement>('h1 input')!;
  const edit = () => {
    document.querySelector<HTMLElement>('h1 [data-title]')!.click();
    flushSync();
  };
  it('Enter saves', () => {
    const calls = props();
    edit();
    input().value = 'New';
    input().dispatchEvent(new Event('input', { bubbles: true }));
    key(input(), 'Enter');
    expect(calls).toEqual(['New']);
  });
  it('Esc cancels', () => {
    const calls = props();
    edit();
    input().value = 'New';
    input().dispatchEvent(new Event('input', { bubbles: true }));
    key(input(), 'Escape');
    expect(calls).toEqual([]);
    expect(document.querySelector('h1')!.textContent).toContain('Old');
  });
  it('blur saves', () => {
    const calls = props();
    edit();
    input().value = 'New';
    input().dispatchEvent(new Event('input', { bubbles: true }));
    input().dispatchEvent(new FocusEvent('blur'));
    flushSync();
    expect(calls).toEqual(['New']);
  });
  it('title snippet replaces the text', () => {
    render(PageHeader, { title: 'Old', title: html('Custom') });
    expect(document.querySelector('h1')!.textContent).toContain('Custom');
  });
});

describe('trigger props', () => {
  it('Button and IconButton spread rest props', () => {
    render(Button, { 'data-x': '1', 'aria-haspopup': 'menu', children: html('a') });
    expect(document.querySelector('button')!.getAttribute('data-x')).toBe('1');
    document.body.innerHTML = '';
    render(IconButton, { icon: 'search', label: 'S', 'data-x': '2' });
    expect(document.querySelector('button')!.getAttribute('data-x')).toBe('2');
  });
});

describe('ContextMenu', () => {
  it('opens on contextmenu and lists items', async () => {
    const items = createRawSnippet(() => ({ render: () => `<span></span>` }));
    render(ContextMenu, { area: html('area'), children: items });
    const t = document.querySelector('span')!.parentElement!;
    t.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 5, clientY: 5 }));
    await wait(20);
    expect(document.querySelector('.cx-menu')).not.toBeNull();
  });
});

describe('ListRow', () => {
  it('marks focused and selected, and picks on click', () => {
    let picked = 0;
    render(ListRow, { focused: true, selected: true, onpick: () => picked++, children: html('Row') });
    const r = document.querySelector<HTMLElement>('.lrow')!;
    expect(r.dataset.focused).toBe('true');
    expect(r.dataset.selected).toBe('true');
    r.click();
    expect(picked).toBe(1);
  });
});

describe('fields callbacks', () => {
  it('TextInput passes type and fires onchange', () => {
    const got: string[] = [];
    render(TextInput, { label: 'At', type: 'time', value: '', onchange: (v: string) => got.push(v) });
    const i = document.querySelector('input')!;
    expect(i.type).toBe('time');
    i.value = '10:30';
    i.dispatchEvent(new Event('change', { bubbles: true }));
    expect(got).toEqual(['10:30']);
  });
  it('Checkbox fires onchange(checked)', () => {
    const got: boolean[] = [];
    render(Checkbox, { label: 'C', checked: false, onchange: (c: boolean) => got.push(c) });
    document.querySelector<HTMLElement>('[role=checkbox]')!.click();
    flushSync();
    expect(got).toEqual([true]);
  });
});

describe('GroupHeader tone', () => {
  it('takes a tone instead of a category and a leading snippet', () => {
    render(GroupHeader, { name: 'P', count: 1, tone: 3, leading: html('LEAD') });
    expect(document.querySelector<HTMLElement>('.band')!.dataset.step).toBe('3');
    expect(document.querySelector('.band')!.textContent).toContain('LEAD');
    expect(document.querySelector('.band svg[data-glyph], .band .glyph')).toBeNull();
  });
});
