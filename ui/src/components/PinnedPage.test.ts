import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mount, unmount } from 'svelte';
import { setApi, type Api } from '../lib/api';
import { createMockApi, fixtures } from '../lib/api.mock';
import type { PinKind, ResolvedPin } from '../lib/bindings';
import PinnedPage from './PinnedPage.svelte';

let mounted: ReturnType<typeof mount>[] = [];
const open = async (api: Api) => {
  setApi(api);
  const target = document.createElement('div');
  document.body.append(target);
  mounted.push(mount(PinnedPage, { target }));
  // The page has no loading text; one macrotask lets its initial load resolve.
  await new Promise((r) => setTimeout(r, 0));
  return target;
};
const seedPins = async (pins: [PinKind, string][]) => {
  const mock = createMockApi(fixtures);
  for (const [kind, target] of pins) await mock.pins.pin(kind, target, null);
  return mock;
};
const cards = (target: HTMLElement) => [...target.querySelectorAll<HTMLElement>('[data-pin-id]')];

beforeEach(() => {
  history.replaceState(null, '', '/pinned');
});
afterEach(() => {
  mounted.forEach((m) => void unmount(m));
  mounted = [];
  document.body.innerHTML = '';
  vi.restoreAllMocks();
});

describe('PinnedPage', () => {
  it('pinned_page_groups_by_kind', async () => {
    const mock = await seedPins([['Issue', 'ABC-1'], ['Project', 'website'], ['Issue', 'ABC-2']]);
    const target = await open(mock);
    const issueGroup = target.querySelector<HTMLElement>('[data-kind="Issue"]');
    const projectGroup = target.querySelector<HTMLElement>('[data-kind="Project"]');
    expect(issueGroup?.querySelectorAll('[data-pin-id]').length).toBe(2);
    expect(projectGroup?.querySelectorAll('[data-pin-id]').length).toBe(1);
    expect(target.querySelector('[data-kind="Url"]')).toBeNull();
  });

  it('archived_pin_dimmed', async () => {
    const mock = await seedPins([['Issue', 'ABC-1'], ['Issue', 'ABC-2']]);
    const real = mock.pins.listResolved;
    mock.pins.listResolved = async () => {
      const rows = await real();
      return rows.map((r, i): ResolvedPin => (i === 0 ? { ...r, archived: true } : r));
    };
    const target = await open(mock);
    const [archived, live] = cards(target);
    expect(archived.classList.contains('archived')).toBe(true);
    expect(archived.textContent).toContain('Archived');
    expect(live.classList.contains('archived')).toBe(false);
    expect(live.textContent).not.toContain('Archived');
  });

  it('empty_pinned_page_shows_empty_state', async () => {
    const target = await open(createMockApi(fixtures));
    expect(cards(target)).toHaveLength(0);
    expect(target.textContent).toContain('Nothing pinned yet');
  });

  it('drag_reorders_pins', async () => {
    const mock = await seedPins([['Issue', 'ABC-1'], ['Issue', 'ABC-2'], ['Issue', 'ABC-3']]);
    const target = await open(mock);
    const [a, , c] = cards(target);
    c.dispatchEvent(new Event('dragstart', { bubbles: true }));
    a.dispatchEvent(new Event('drop', { bubbles: true, cancelable: true }));
    await vi.waitFor(async () => {
      const order = (await mock.pins.listResolved()).map((r) => r.pin.target);
      expect(order).toEqual(['ABC-3', 'ABC-1', 'ABC-2']);
    });
  });

  it('url_pin_with_non_http_target_is_not_a_link', async () => {
    // The backend refuses a javascript: pin, so it can only arrive as old data; seed it directly.
    const url = (id: number, target: string) => ({ id, kind: 'Url' as const, target, title: null, position: id });
    const mock = createMockApi({ ...fixtures, pins: [url(1, 'javascript:alert(1)'), url(2, 'https://example.com')] });
    const target = await open(mock);
    const links = [...target.querySelectorAll<HTMLAnchorElement>('a')].map((a) => a.getAttribute('href'));
    expect(links).not.toContain('javascript:alert(1)');
    expect(links).toContain('https://example.com');
  });
});

describe('PinnedPage unpin', () => {
  it('unpin_removes_the_pin', async () => {
    const mock = await seedPins([['Issue', 'ZF-1'], ['Issue', 'ZF-2']]);
    const target = await open(mock);
    const first = cards(target)[0];
    first.querySelector<HTMLButtonElement>('button[aria-label^="Unpin"]')!.click();
    await vi.waitFor(() => expect(cards(target)).toHaveLength(1));
    expect((await mock.pins.listResolved()).map((r) => r.pin.target)).toEqual(['ZF-2']);
  });
});
