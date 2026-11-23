import { describe, expect, it, vi } from 'vitest';

const openExternal = vi.fn(async (_url: string) => true);
vi.mock('../../lib/openExternal', () => ({ openExternal: (url: string) => openExternal(url) }));

import { openLinks, renderMarkdown } from './markdown';

const dom = (html: string) => {
  const d = document.createElement('div');
  d.innerHTML = html;
  return d;
};

describe('bare URLs as link chips', () => {
  it('shows a bare https URL as a chip with host and short path, full URL in the title', () => {
    const d = dom(renderMarkdown('see https://example.com/docs/guide/intro for more'));
    const a = d.querySelector('a.chip')!;
    expect(a.getAttribute('href')).toBe('https://example.com/docs/guide/intro');
    expect(a.getAttribute('title')).toBe('https://example.com/docs/guide/intro');
    expect(a.textContent).toBe('example.com/docs/guide/intro');
    expect(a.querySelector('svg')).not.toBeNull();
  });

  it('shortens a long path', () => {
    const d = dom(renderMarkdown('https://example.com/' + 'a'.repeat(80)));
    expect(d.querySelector('a.chip')!.textContent!.length).toBeLessThan(60);
  });

  it('names an obsidian link after its note', () => {
    const d = dom(renderMarkdown('open obsidian://open?vault=Work&file=Projects%2FPlan%20A.md now'));
    const a = d.querySelector('a.chip')!;
    expect(a.textContent).toBe('Obsidian: Plan A');
    expect(a.getAttribute('href')).toBe('obsidian://open?vault=Work&file=Projects%2FPlan%20A.md');
  });

  it('keeps text of a markdown link as plain link text, not a chip', () => {
    const d = dom(renderMarkdown('[the spec](https://example.com/spec)'));
    expect(d.querySelector('a.chip')).toBeNull();
    expect(d.querySelector('a')!.textContent).toBe('the spec');
  });

  it('escapes markup in the label', () => {
    const d = dom(renderMarkdown('obsidian://open?vault=v&file=%3Cb%3Ex.md'));
    expect(d.querySelector('b')).toBeNull();
    expect(d.querySelector('a.chip')!.textContent).toBe('Obsidian: <b>x');
  });
});

describe('sanitising stays intact', () => {
  it.each([
    '[x](javascript:alert(1))',
    '<javascript:alert(1)>',
    '[x](JavaScript:alert(1))',
  ])('never makes a javascript: URL a link: %s', (src) => {
    const d = dom(renderMarkdown(src));
    for (const a of Array.from(d.querySelectorAll('a'))) {
      expect(a.getAttribute('href') ?? '').not.toMatch(/javascript:/i);
    }
  });

  it('drops script tags', () => {
    expect(dom(renderMarkdown('<script>window.pwned = 1</script>hi')).querySelector('script')).toBeNull();
  });
});

describe('openLinks', () => {
  it('opens a clicked chip through openExternal and stops the click going further', () => {
    openExternal.mockClear();
    const host = dom(renderMarkdown('go https://example.com/x'));
    document.body.append(host);
    const parentClick = vi.fn();
    host.addEventListener('click', parentClick);
    const action = openLinks(host);
    const a = host.querySelector('a.chip') as HTMLAnchorElement;
    a.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    expect(openExternal).toHaveBeenCalledWith('https://example.com/x');
    expect(parentClick).not.toHaveBeenCalled();
    action.destroy?.();
    host.remove();
  });
});
