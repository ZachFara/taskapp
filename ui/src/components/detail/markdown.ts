import DOMPurify from 'dompurify';
import { Marked } from 'marked';
import { api } from '../../lib/api';
import { isAllowedUrl, openExternal } from '../../lib/openExternal';
import { ICONS } from '../../ui/icons';

const esc = (s: string) => s.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

/** Label for a bare link: "Obsidian: <note>" for obsidian://, else host and a shortened path. No network call. */
export function linkLabel(href: string): string {
  try {
    const u = new URL(href);
    if (u.protocol === 'obsidian:') {
      const file = u.searchParams.get('file');
      return `Obsidian: ${file ? (file.split('/').pop() ?? file).replace(/\.md$/i, '') : (u.searchParams.get('vault') ?? 'link')}`;
    }
    const path = u.pathname === '/' ? '' : u.pathname;
    return u.hostname.replace(/^www\./, '') + (path.length > 40 ? `${path.slice(0, 39)}…` : path);
  } catch {
    return href;
  }
}

const chip = (href: string) =>
  `<a class="chip" href="${esc(href)}"><svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS.link}</svg><span>${esc(linkLabel(href))}</span></a>`;

// Marked's own autolinks (http, https, www) reach the link renderer; obsidian:// needs its own tokenizer.
const marked = new Marked({
  extensions: [
    {
      name: 'obsidianLink',
      level: 'inline',
      start: (src: string) => src.match(/obsidian:\/\//)?.index,
      tokenizer(src: string) {
        const m = /^obsidian:\/\/[^\s<]*[^\s<.,;:!?)"']/.exec(src);
        if (m && isAllowedUrl(m[0])) return { type: 'obsidianLink', raw: m[0], text: m[0] };
      },
      renderer: (t) => chip(t.text),
    },
  ],
  renderer: {
    // Only `attachment:<number>` is an image; its bytes load through the API (see `images`). Any other image source
    // would make the reader's machine fetch it, so it shows as its alt text.
    image({ href, text }) {
      const m = /^attachment:(\d+)$/.exec(href);
      return m ? `<img data-attachment="${m[1]}" alt="${esc(text)}" loading="lazy">` : esc(text);
    },
    link({ href, text, tokens }) {
      // A disallowed obsidian action (new, plugin commands) is plain text, never a link.
      if (/^obsidian:/i.test(href) && !isAllowedUrl(href)) return this.parser.parseInline(tokens);
      if (text === href) return chip(href);
      return `<a href="${esc(href)}">${this.parser.parseInline(tokens)}</a>`;
    },
  },
});

// DOMPurify's default allow-list has no obsidian: scheme, so its hrefs would be stripped. javascript: stays refused.
const URI = /^(?:(?:(?:f|ht)tps?|mailto|tel|callto|sms|cid|xmpp|obsidian):|[^a-z]|[a-z+.\-]+(?:[^a-z+.\-:]|$))/i;

// Raw <img> in the text is held to the same rule: never a source of its own, and only with a numeric attachment id.
DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName !== 'IMG') return;
  node.removeAttribute('src');
  node.removeAttribute('srcset');
  if (!/^\d+$/.test(node.getAttribute('data-attachment') ?? '')) node.remove();
});

/** Markdown to HTML, sanitized: raw HTML such as a script tag never survives. */
export const renderMarkdown = (src: string): string =>
  DOMPurify.sanitize(marked.parse(src, { async: false }), { ALLOWED_URI_REGEXP: URI });

/** Svelte action for a `.md` block: a link opens through openExternal, never in the webview. Capture phase, so a
 * link click does not also reach the block's own handler (the description opens its editor on click). */
export function openLinks(node: HTMLElement) {
  const onClick = (e: MouseEvent) => {
    const a = (e.target as Element | null)?.closest('a[href]');
    if (!a || !node.contains(a)) return;
    e.preventDefault();
    e.stopPropagation();
    const href = a.getAttribute('href') ?? '';
    if (isAllowedUrl(href)) void openExternal(href);
  };
  node.addEventListener('click', onClick, true);
  return { destroy: () => node.removeEventListener('click', onClick, true) };
}

// shortcut: object URLs are kept for the life of the page (one per image id), so a repeat render never refetches.
const urls = new Map<string, Promise<string>>();
const urlOf = (id: string) => {
  if (!urls.has(id)) {
    urls.set(id, api.attachments.get(Number(id)).then((a) => URL.createObjectURL(new Blob([new Uint8Array(a.bytes)], { type: a.mime }))));
    urls.get(id)!.catch(() => urls.delete(id));
  }
  return urls.get(id)!;
};

/** Svelte action for a rendered `.md` block: fills each attachment image from the API, and a click on one opens it larger
 * (`image:view`, shown by ImageViewer). Pass the markdown so it runs again when the text changes. */
export function images(node: HTMLElement, _src?: string) {
  const load = () => {
    for (const img of node.querySelectorAll<HTMLImageElement>('img[data-attachment]:not([src])')) {
      urlOf(img.dataset.attachment!).then((u) => (img.src = u), () => (img.alt = `${img.alt || 'image'} (not found)`));
    }
  };
  const onClick = (e: MouseEvent) => {
    const img = (e.target as Element | null)?.closest<HTMLImageElement>('img[data-attachment]');
    if (!img?.src) return;
    e.preventDefault();
    e.stopPropagation();
    dispatchEvent(new CustomEvent('image:view', { detail: { src: img.src, alt: img.alt } }));
  };
  node.addEventListener('click', onClick, true);
  queueMicrotask(load);
  return { update: () => queueMicrotask(load), destroy: () => node.removeEventListener('click', onClick, true) };
}
