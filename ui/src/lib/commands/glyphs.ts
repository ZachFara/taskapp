// One formatter from registry key syntax (`mod+shift+z`, `g i`, `arrowdown`) to keycap text; Kbd is the only caller.
const MAC = !/Win|Linux/.test(globalThis.navigator?.platform ?? '');
const MOD: Record<string, string> = MAC
  ? { mod: '⌘', shift: '⇧', alt: '⌥' }
  : { mod: 'Ctrl+', shift: 'Shift+', alt: 'Alt+' };
const NAME: Record<string, string> = {
  enter: '↵', escape: 'Esc', esc: 'Esc', arrowup: '↑', arrowdown: '↓', arrowleft: '←', arrowright: '→',
  backspace: '⌫', tab: 'Tab', ' ': 'Space', space: 'Space',
};

/** `mod+shift+z` -> `⌘⇧Z`. Text that is already a glyph (`⌘K`, `Esc`, `↵`) passes through. */
function cap(part: string) {
  const bits = part.length > 1 ? part.split(/\+(?=.)/) : [part];
  const key = bits.pop()!;
  const k = NAME[key.toLowerCase()] ?? (key.length === 1 ? key.toUpperCase() : /^f\d+$/i.test(key) ? key.toUpperCase() : key);
  return bits.map((m) => MOD[m.toLowerCase()] ?? m).join('') + k;
}

/** One keycap per key press of each sequence: `g i` -> G, I. */
export const keycaps = (keys: string | string[]) => [keys].flat().flatMap((s) => s.split(/\s+/).filter(Boolean).map(cap));
