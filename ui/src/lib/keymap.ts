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

import { createKeys, type Binding } from './commands/keys';

const entries: Binding[] = [];
let detach: (() => void) | null = null;

/** Calls `handler` when `combo` is pressed outside text fields; returns the function that unregisters it. */
export function registerShortcut(combo: string, _id: string, handler: () => void): () => void {
  detach ??= createKeys(() => entries);
  // Combos with a modifier (cmd+k) keep working from inside a text field, as they always did here.
  const entry = { seq: combo, global: combo.includes('+'), run: () => handler() };
  entries.push(entry);
  return () => {
    entries.splice(entries.indexOf(entry), 1);
    if (!entries.length) {
      detach?.();
      detach = null;
    }
  };
}
