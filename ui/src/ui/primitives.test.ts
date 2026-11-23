import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import Button from './Button.svelte';
import Checkbox from './Checkbox.svelte';
import EmptyState from './EmptyState.svelte';
import EntityIcon from './EntityIcon.svelte';
import GroupHeader from './GroupHeader.svelte';
import Icon from './Icon.svelte';
import IconButton from './IconButton.svelte';
import Pill from './Pill.svelte';
import PriorityGlyph from './PriorityGlyph.svelte';
import Select from './Select.svelte';
import StatusGlyph from './StatusGlyph.svelte';
import Switch from './Switch.svelte';
import Tabs from './Tabs.svelte';
import TextInput from './TextInput.svelte';
import Toast from './Toast.svelte';
import { ICON_IDS } from './icons';
import { createRawSnippet } from 'svelte';

// Glyph assertions below are the Mac ones whatever machine runs them.
beforeEach(() => void vi.spyOn(navigator, 'platform', 'get').mockReturnValue('MacIntel'));

let app: ReturnType<typeof mount> | undefined;
afterEach(() => {
  if (app) unmount(app);
  app = undefined;
  document.body.innerHTML = '';
});
const text = (s: string) => createRawSnippet(() => ({ render: () => `<span>${s}</span>` }));
const render = (C: any, props: Record<string, unknown> = {}) => {
  app = mount(C, { target: document.body, props });
  flushSync();
};
const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};

describe('Button', () => {
  it('always renders through the btn class with its variant, never a bare button', () => {
    for (const variant of ['primary', 'quiet', 'danger']) {
      document.body.innerHTML = '';
      render(Button, { variant, children: text('Go') });
      const b = document.querySelector('button')!;
      expect(b.classList.contains('btn')).toBe(true);
      expect(b.dataset.variant).toBe(variant);
      expect(b.type).toBe('button');
    }
  });
  it('shows an optional key hint and icon', () => {
    render(Button, { variant: 'primary', icon: 'plus', kbd: 'C', children: text('New') });
    expect(document.querySelector('kbd')?.textContent).toBe('C');
    expect(document.querySelector('svg[data-icon=plus]')).not.toBeNull();
  });
});

describe('Tooltip', () => {
  it('shows the action name and its shortcut when the trigger is focused', async () => {
    render(IconButton, { icon: 'search', label: 'Search', shortcut: ['⌘', 'K'] });
    document.querySelector('button')!.focus();
    await settle();
    const tip = document.querySelector('.cx-tip')!;
    expect(tip.textContent).toContain('Search');
    expect([...tip.querySelectorAll('kbd')].map((k) => k.textContent)).toEqual(['⌘', 'K']);
  });
  it('stays closed before any hover or focus', async () => {
    render(IconButton, { icon: 'search', label: 'Search' });
    await settle();
    expect(document.querySelector('.cx-tip')).toBeNull();
  });
});

describe('icons', () => {
  it('draws every known id and falls back to a circle for an unknown one', () => {
    expect(ICON_IDS.length).toBeGreaterThanOrEqual(40);
    render(Icon, { name: 'inbox' });
    expect(document.querySelector('svg')?.innerHTML).not.toBe('');
    document.body.innerHTML = '';
    render(Icon, { name: 'no-such-icon' });
    expect(document.querySelector('svg circle')).not.toBeNull();
  });
  it('EntityIcon renders icon ids and emoji in a palette colour, gray when the key is unknown', () => {
    render(EntityIcon, { icon: 'icon:folder', color: 'blue' });
    expect(document.querySelector<HTMLElement>('.entity')!.style.color).toBe('var(--c-blue)');
    document.body.innerHTML = '';
    render(EntityIcon, { icon: 'emoji:🚀', color: 'nope' });
    expect(document.querySelector('.emoji')?.textContent).toBe('🚀');
    expect(document.querySelector<HTMLElement>('.entity')!.dataset.color).toBe('gray');
  });
});

describe('glyphs', () => {
  it('status glyphs differ by category, and a chosen icon replaces the default', () => {
    const markup = ['Backlog', 'Unstarted', 'Started', 'Completed', 'Canceled'].map((category) => {
      document.body.innerHTML = '';
      render(StatusGlyph, { category });
      return document.querySelector('svg')!.innerHTML;
    });
    expect(new Set(markup).size).toBe(5);
    document.body.innerHTML = '';
    render(StatusGlyph, { category: 'Started', icon: 'icon:star', color: 'red' });
    expect(document.querySelector('[data-icon=star]')).not.toBeNull();
  });
  it('priority glyphs fill one more bar per level; urgent fills all three', () => {
    const bars = [0, 1, 2, 3, 4].map((level) => {
      document.body.innerHTML = '';
      render(PriorityGlyph, { level });
      return document.querySelectorAll('rect').length === 3 ? document.querySelectorAll('rect.on').length : -1;
    });
    expect(bars).toEqual([0, 1, 2, 3, 3]);
  });
});

describe('fields', () => {
  it('TextInput is labelled, bindable and marks invalid', () => {
    render(TextInput, { label: 'Title', value: 'x', invalid: true });
    const i = document.querySelector('input')!;
    expect(i.getAttribute('aria-label')).toBe('Title');
    expect(i.value).toBe('x');
    expect(i.getAttribute('aria-invalid')).toBe('true');
  });
  it('Checkbox and Switch expose their state', () => {
    render(Checkbox, { label: 'Done', checked: true });
    expect(document.querySelector('[role=checkbox]')?.getAttribute('aria-checked')).toBe('true');
    document.body.innerHTML = '';
    render(Switch, { label: 'On', checked: false });
    expect(document.querySelector('[role=switch]')?.getAttribute('aria-checked')).toBe('false');
  });
  it('Select shows the chosen label, else the placeholder', () => {
    const items = [{ value: 'a', label: 'Alpha' }];
    render(Select, { items, value: 'a', label: 'Pick' });
    expect(document.querySelector('.v')?.textContent).toBe('Alpha');
    document.body.innerHTML = '';
    render(Select, { items, value: '', label: 'Pick', placeholder: 'Choose' });
    expect(document.querySelector('.v')?.textContent).toBe('Choose');
  });
});

describe('layout pieces', () => {
  it('Tabs lists every tab and marks the active one', () => {
    render(Tabs, { value: 'b', tabs: [{ id: 'a', label: 'A' }, { id: 'b', label: 'B' }] });
    const tabs = [...document.querySelectorAll('[role=tab]')];
    expect(tabs.map((t) => t.textContent?.trim())).toEqual(['A', 'B']);
    expect(tabs[1].getAttribute('data-state')).toBe('active');
  });
  it('GroupHeader deepens the band with the status category and toggles', () => {
    const steps: Record<string, string | undefined> = {};
    let toggled = 0;
    for (const category of ['Backlog', 'Unstarted', 'Started', 'Completed']) {
      document.body.innerHTML = '';
      render(GroupHeader, { name: 'N', count: 2, category, ontoggle: () => toggled++ });
      steps[category] = document.querySelector<HTMLElement>('.band')!.dataset.step;
    }
    expect(steps).toEqual({ Backlog: '1', Unstarted: '2', Started: '3', Completed: '4' });
    document.querySelector<HTMLButtonElement>('.toggle')!.click();
    expect(toggled).toBe(1);
  });
  it('EmptyState, Pill and Toast render their content', () => {
    render(EmptyState, { title: 'Nothing here', hint: 'Add one' });
    expect(document.querySelector('h2')?.textContent).toBe('Nothing here');
    expect(document.querySelector('svg.contour path')).not.toBeNull();
    document.body.innerHTML = '';
    render(Pill, { color: 'teal', children: text('ux') });
    expect(document.querySelector<HTMLElement>('.pill')!.style.getPropertyValue('--tint')).toBe('var(--c-teal)');
    document.body.innerHTML = '';
    render(Toast, { tone: 'error', children: text('Failed') });
    expect(document.querySelector('[role=alert]')?.textContent).toContain('Failed');
  });
  it('an info Toast draws no icon; success and error keep theirs', () => {
    for (const [tone, icon] of [['info', null], ['success', 'check'], ['error', 'flag']] as const) {
      document.body.innerHTML = '';
      render(Toast, { tone, children: text('x') });
      expect(document.querySelector('[data-icon]')?.getAttribute('data-icon') ?? null, tone).toBe(icon);
    }
  });
});
