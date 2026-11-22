/** `active` is asked on every key press; of the active bindings with the same keys the highest `rank` fires. */
export type Binding = { seq: string; global?: boolean; rank?: () => number; active?: () => boolean; run: (count: number) => void };

const TYPING = 'input, textarea, select, [contenteditable]:not([contenteditable="false"])';
const MODIFIERS = ['Shift', 'Control', 'Alt', 'Meta'];
const ALIASES: Record<string, string> = { esc: 'escape', cmd: 'mod', ctrl: 'mod', meta: 'mod', space: ' ' };

/** One key press in canonical form: `j`, `J` (shift+j), `?`, `mod+k`, `escape`, `shift+enter`. */
function token(key: string, mod: boolean, alt: boolean, shift: boolean) {
  const letter = key.length === 1 && /[a-z]/i.test(key);
  const k = letter ? (shift ? key.toUpperCase() : mod || alt ? key.toLowerCase() : key) : key.length > 1 ? key.toLowerCase() : key;
  return (mod ? 'mod+' : '') + (alt ? 'alt+' : '') + (shift && key.length > 1 ? 'shift+' : '') + k;
}

/** `g i` -> ['g', 'i']; `mod+k` -> ['mod+k']; `shift+j` -> ['J']. */
export function parseSeq(seq: string) {
  return seq.split(/\s+/).filter(Boolean).map((part) => {
    const bits = part.length > 1 ? part.split(/\+(?=.)/) : [part];
    const mods = bits.slice(0, -1).map((m) => ALIASES[m.toLowerCase()] ?? m.toLowerCase());
    const key = bits[bits.length - 1];
    return token(ALIASES[key.toLowerCase()] ?? key, mods.includes('mod'), mods.includes('alt'), mods.includes('shift'));
  });
}

/**
 * Attaches the one keydown listener; `bindings` is read on every key so the list can change.
 * Returns the function that detaches it.
 */
export function createKeys(bindings: () => Binding[]) {
  let buf: string[] = [];
  let count = '';
  let timer: ReturnType<typeof setTimeout> | undefined;
  const reset = () => {
    buf = [];
    count = '';
    clearTimeout(timer);
  };
  const wait = () => {
    clearTimeout(timer);
    timer = setTimeout(reset, 1000);
  };

  function onKeydown(e: KeyboardEvent) {
    if (e.isComposing || e.keyCode === 229 || MODIFIERS.includes(e.key)) return;
    const t = token(e.key, e.metaKey || e.ctrlKey, e.altKey, e.shiftKey);
    const target = e.target instanceof Element ? e.target : null;
    const typing = !!target?.closest(TYPING);
    if (t === 'enter' && target?.closest('button, a, [role=button], [role=menuitem]')) return;
    if (t === 'escape' && (buf.length || count)) return reset();
    const all = bindings().map((b) => ({ b, seq: parseSeq(b.seq) }));
    const live = all.filter((x) => x.b.active?.() !== false);
    const usable = typing ? live.filter((x) => x.seq.length === 1 && (x.seq[0] === 'escape' || (x.b.global && x.seq[0].startsWith('mod+')))) : live;
    // A failed sequence retries the key on its own, so `g x` then `c` still works.
    for (const prefix of buf.length && !typing ? [buf, []] : [[]]) {
      const next = [...prefix, t];
      const starts = (s: string[]) => next.every((k, i) => s[i] === k);
      const found = usable.filter((x) => x.seq.length === next.length && starts(x.seq));
      const rank = (x: (typeof found)[number]) => x.b.rank?.() ?? 0;
      const top = Math.max(...found.map(rank));
      const hits = found.filter((x) => rank(x) === top);
      if (hits.length) {
        e.preventDefault();
        const n = Number(count) || 1;
        reset();
        hits.forEach((x) => x.b.run(n));
        return;
      }
      if (usable.some((x) => x.seq.length > next.length && starts(x.seq))) {
        e.preventDefault();
        buf = next;
        return wait();
      }
    }
    const [wasPending, before] = [buf.length > 0, count];
    reset();
    if (!typing && !wasPending && /^\d$/.test(t) && (before || t !== '0')) {
      count = before + t;
      wait();
    }
  }
  addEventListener('keydown', onKeydown);
  return () => {
    removeEventListener('keydown', onKeydown);
    reset();
  };
}
