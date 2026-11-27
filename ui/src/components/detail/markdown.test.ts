import { describe, expect, it, vi } from 'vitest';

const openExternal = vi.fn(async (_url: string) => true);
vi.mock('../../lib/openExternal', async (importOriginal) => ({
  ...(await importOriginal<typeof import('../../lib/openExternal')>()),
  openExternal: (url: string) => openExternal(url),
}));

import { openLinks, renderMarkdown } from './markdown';

const dom = (html: string) => {
  const d = document.createElement('div');
  d.innerHTML = html;
  return d;
};

describe('bare URLs as link chips', () => {
  it('shows a bare https URL as a chip with host and short path, with no hover title', () => {
    const d = dom(renderMarkdown('see https://example.com/docs/guide/intro for more'));
    const a = d.querySelector('a.chip')!;
    expect(a.getAttribute('href')).toBe('https://example.com/docs/guide/intro');
    expect(a.hasAttribute('title')).toBe(false);
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

describe('obsidian links are limited to open with vault and file', () => {
  it('a plain open link with vault and file is a chip', () => {
    const d = dom(renderMarkdown('obsidian://open?vault=v&file=n'));
    expect(d.querySelector('a.chip')!.textContent).toBe('Obsidian: n');
  });

  it.each([
    'obsidian://new?vault=v&name=n&content=x',
    'obsidian://open?vault=v&file=n&content=x',
    'obsidian://open?vault=v&file=n&path=/etc/passwd',
    'obsidian://unknown?vault=v&file=n',
  ])('%s is never a link, only text', (src) => {
    const d = dom(renderMarkdown(`see ${src} now`));
    expect(d.querySelector('a')).toBeNull();
    expect(d.textContent).toContain(src);
  });

  it('a markdown link to a disallowed obsidian URI is plain text', () => {
    const d = dom(renderMarkdown('[x](obsidian://new?vault=v&content=x)'));
    expect(d.querySelector('a')).toBeNull();
    expect(d.textContent!.trim()).toBe('x');
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
  it('a click on a disallowed obsidian link never reaches openExternal', () => {
    openExternal.mockClear();
    const host = document.createElement('div');
    host.innerHTML = '<a href="obsidian://new?vault=v&content=x">x</a>';
    document.body.append(host);
    const action = openLinks(host);
    host.querySelector('a')!.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    expect(openExternal).not.toHaveBeenCalled();
    action.destroy?.();
    host.remove();
  });

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

describe('images', () => {
  it('shows only attachment:<number> as an image, with no source of its own', () => {
    const img = dom(renderMarkdown('![shot](attachment:12)')).querySelector('img')!;
    expect(img.dataset.attachment).toBe('12');
    expect(img.getAttribute('alt')).toBe('shot');
    expect(img.hasAttribute('src')).toBe(false);
    expect(img.getAttribute('loading')).toBe('lazy');
  });

  it.each([
    '![x](javascript:alert(1))',
    '![x](data:image/png;base64,AAAA)',
    '![x](https://example.com/a.png)',
    '![x](http://example.com/a.png)',
    '![x](attachment:abc)',
    '![x](attachment:1x)',
    '![x](attachment:)',
    '<img src="https://example.com/a.png">',
    '<img src="data:image/png;base64,AAAA" onerror="alert(1)">',
    '<img src="attachment:3">',
  ])('never renders %s as an image', (md) => {
    const d = dom(renderMarkdown(md));
    expect(d.querySelector('img')).toBeNull();
    expect(d.innerHTML).not.toMatch(/onerror/);
  });

  it('strips a source typed by hand off an attachment image', () => {
    const img = dom(renderMarkdown('<img data-attachment="4" src="https://example.com/a.png">')).querySelector('img')!;
    expect(img.hasAttribute('src')).toBe(false);
  });
});

describe('checklist items render as checkboxes', () => {
  it('numbers each task box in document order and keeps its state', () => {
    const d = dom(renderMarkdown('- [ ] first\n- [x] second\n\n```\n- [ ] code\n```\n\n- [ ] third'));
    const boxes = [...d.querySelectorAll<HTMLInputElement>('input[type=checkbox]')];
    expect(boxes.map((b) => b.dataset.check)).toEqual(['0', '1', '2']);
    expect(boxes.map((b) => b.checked)).toEqual([false, true, false]);
  });

  it('starts numbering again for each render', () => {
    renderMarkdown('- [ ] a\n- [ ] b');
    const d = dom(renderMarkdown('- [ ] c'));
    expect(d.querySelector('input')!.dataset.check).toBe('0');
  });
});
