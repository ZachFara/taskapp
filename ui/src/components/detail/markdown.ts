import DOMPurify from 'dompurify';
import { Marked } from 'marked';
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
  `<a class="chip" href="${esc(href)}" title="${esc(href)}"><svg viewBox="0 0 16 16" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS.link}</svg><span>${esc(linkLabel(href))}</span></a>`;

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
