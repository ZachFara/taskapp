import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushSync, mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import { demoDecisions } from '../lib/api.mock.decisions';
import { decisions } from '../lib/decisions.svelte';
import DecisionsPage from './DecisionsPage.svelte';
import Sidebar from './Sidebar.svelte';
import IssueDecisions from './detail/IssueDecisions.svelte';

let live: ReturnType<typeof mount> | null = null;
beforeEach(() => {
  history.replaceState(null, '', '/decisions');
  decisions.all = [];
  decisions.loaded = false;
  decisions.error = '';
});
afterEach(() => {
  if (live) unmount(live);
  live = null;
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

const settle = async () => {
  for (let i = 0; i < 10; i++) await new Promise((r) => setTimeout(r, 0));
  flushSync();
};
async function open(api: Api, component: typeof DecisionsPage | typeof Sidebar = DecisionsPage) {
  setApi(api);
  live = mount(component as typeof DecisionsPage, { target: document.body });
  await settle();
}
const mockApi = () => createMockApi({ ...fixtures, decisions: structuredClone(demoDecisions) });
const card = (id: number) => document.querySelector<HTMLElement>(`[data-decision="${id}"]`)!;
const text = (el: Element) => el.textContent?.replace(/\s+/g, ' ');

describe('DecisionsPage', () => {
  it('a decision link opens the tab of that decision and focuses it', async () => {
    history.replaceState(null, '', '/decisions#3');
    await open(mockApi());
    expect(card(3)).not.toBeNull();
    const copied: string[] = [];
    vi.stubGlobal('navigator', { clipboard: { writeText: async (t: string) => void copied.push(t) } });
    dispatchEvent(new CustomEvent('command:decision.copyLink'));
    await settle();
    expect(copied).toEqual(['taskapp://decision/3']);
    vi.unstubAllGlobals();
  });

  it('a link in a decision context never navigates the app window', async () => {
    const seed = structuredClone(demoDecisions);
    seed[0].context = 'See [this page](https://example.com/evil)';
    await open(createMockApi({ ...fixtures, decisions: seed }));
    const link = card(seed[0].id).querySelector('a')!;
    const click = new MouseEvent('click', { bubbles: true, cancelable: true });
    link.dispatchEvent(click);
    expect(click.defaultPrevented).toBe(true);
  });

  it('shows open decisions as cards, blocking first, with trade-offs and the recommendation marked', async () => {
    await open(mockApi());
    const cards = [...document.querySelectorAll<HTMLElement>('[data-decision]')];
    expect(cards.map((c) => c.dataset.decision)).toEqual(['2', '1']);
    expect(card(2).dataset.blocking).toBe('true');
    expect(text(card(2))).toContain('Cheap; a phone change waits');
    expect(text(card(2).querySelector('[data-recommended]')!)).toContain('Recommended');
    expect(text(card(2))).toContain('ABC-1');
    expect(card(2).querySelector('.md')!.textContent).toContain('two seconds');
  });

  it('answers with one click, sends the note, and the card leaves the open list', async () => {
    const api = mockApi();
    await open(api);
    const note = card(2).querySelector<HTMLInputElement>('input')!;
    note.value = 'phone changes are rare';
    note.dispatchEvent(new Event('input', { bubbles: true }));
    card(2).querySelector<HTMLButtonElement>('.opt[data-recommended]')!.click();
    await settle();
    expect(card(2)).toBeNull();
    const done = await api.decisions.get(2);
    expect([done.status, done.answer, done.note]).toEqual(['answered', 'b', 'phone changes are rare']);
  });

  it('answered and withdrawn ones are on the second tab', async () => {
    await open(mockApi());
    [...document.querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent?.includes('Answered'))!.click();
    await settle();
    expect(text(card(3))).toContain('Keep');
    expect(text(card(3))).toContain('Needed for undo.');
    expect(card(3).querySelector('.opt')).toBeNull();
  });

  it('the registry commands act on the focused decision', async () => {
    const api = mockApi();
    await open(api);
    card(1).click();
    dispatchEvent(new CustomEvent('command:decision.answer'));
    await settle();
    expect((await api.decisions.get(1)).answer).toBe('no');
    // Nothing clicked since: the first open decision (now 2) is the target.
    dispatchEvent(new CustomEvent('command:decision.withdraw'));
    await settle();
    expect((await api.decisions.get(2)).status).toBe('withdrawn');
  });

  it('says plainly when a call fails and keeps the card', async () => {
    const api = mockApi();
    api.decisions.answer = async () => {
      throw { kind: 'Invalid', message: 'invalid: nope' };
    };
    await open(api);
    card(1).querySelector<HTMLButtonElement>('.opt')!.click();
    await settle();
    expect(document.querySelector('[role="alert"]')!.textContent).toContain('nope');
    expect(card(1)).not.toBeNull();
  });

  it('has an empty state', async () => {
    await open(createMockApi());
    expect(document.body.textContent).toContain('Nothing waits for you');
  });
});

describe('Sidebar decisions entry', () => {
  it('shows the open count, standing out when one is blocking', async () => {
    await open(mockApi(), Sidebar);
    const n = document.querySelector<HTMLElement>('a[href="/decisions"] .n')!;
    expect(n.textContent).toBe('2');
    expect(n.dataset.urgent).toBe('true');
  });

  it('shows no count when nothing is open or the list cannot be read', async () => {
    await open(createMockApi(), Sidebar);
    expect(document.querySelector('a[href="/decisions"] .n')).toBeNull();
  });
});

describe('IssueDecisions', () => {
  it('lists the open decisions that name the issue, and is absent when there are none', async () => {
    const api = mockApi();
    setApi(api);
    live = mount(IssueDecisions, { target: document.body, props: { issue: await api.issues.get(1), rev: 0 } });
    await settle();
    expect(document.body.textContent).toContain('Should the sync run every minute');
    expect(document.body.textContent).toContain('Blocking');
    unmount(live);
    live = mount(IssueDecisions, { target: document.body, props: { issue: await api.issues.get(2), rev: 0 } });
    await settle();
    expect(document.querySelector('section[aria-label="Decisions"]')).toBeNull();
  });
});
