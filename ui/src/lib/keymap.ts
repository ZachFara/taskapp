/** Action id to combo. Combos are `cmd+k`, a single key, or a sequence such as `g i`. */
export const defaultKeymap = {
  palette: 'cmd+k',
  create: 'c',
  search: '/',
  next: 'j',
  previous: 'k',
  open: 'enter',
  close: 'esc',
  status: 's',
  priority: 'p',
  labels: 'l',
  inbox: 'g i',
  views: 'g v',
} as const;

type Entry = { combo: string; id: string; handler: () => void };
const entries: Entry[] = [];
const SEQUENCE_MS = 1000;
let pending: { key: string; at: number } | null = null;

const KEY_NAMES: Record<string, string> = { Escape: 'esc', Enter: 'enter' };
const keyOf = (e: KeyboardEvent) =>
  (e.metaKey ? 'cmd+' : '') + (e.ctrlKey ? 'ctrl+' : '') + (e.altKey ? 'alt+' : '') + (KEY_NAMES[e.key] ?? e.key.toLowerCase());

function onKeydown(e: KeyboardEvent) {
  const key = keyOf(e);
  const typing =
    e.target instanceof Element && e.target.closest('input, textarea, select, [contenteditable]:not([contenteditable="false"])');
  // Palette and close must still work from inside a text field.
  if (typing && !key.includes('+') && key !== 'esc') return;
  if (key === 'enter' && e.target instanceof Element && e.target.closest('button, a, [role=button], [role=menuitem]')) return;
  const sequence = pending && e.timeStamp - pending.at < SEQUENCE_MS ? `${pending.key} ${key}` : null;
  const hits = entries.filter((x) => x.combo === sequence);
  pending = hits.length ? null : { key, at: e.timeStamp };
  for (const x of hits.length ? hits : entries.filter((x) => x.combo === key)) x.handler();
}

/** Calls `handler` when `combo` is pressed outside text fields; returns the function that unregisters it. */
export function registerShortcut(combo: string, id: string, handler: () => void): () => void {
  if (!entries.length) addEventListener('keydown', onKeydown);
  const entry = { combo, id, handler };
  entries.push(entry);
  return () => {
    entries.splice(entries.indexOf(entry), 1);
    if (!entries.length) removeEventListener('keydown', onKeydown);
  };
}
