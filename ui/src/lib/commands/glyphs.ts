// One formatter from registry key syntax (`mod+shift+z`, `g i`, `arrowdown`) to keycap text; Kbd is the only caller.
const MAC_MOD: Record<string, string> = { mod: '⌘', shift: '⇧', alt: '⌥' };
const OTHER_MOD: Record<string, string> = { mod: 'Ctrl+', shift: 'Shift+', alt: 'Alt+' };
/** Anything but Windows or Linux counts as a Mac (this is a Mac app; an unknown platform gets the Mac glyphs). */
export const isMac = (platform = globalThis.navigator?.platform ?? '') => !/Win|Linux/.test(platform);
const NAME: Record<string, string> = {
  enter: '↵', escape: 'Esc', esc: 'Esc', arrowup: '↑', arrowdown: '↓', arrowleft: '←', arrowright: '→',
  backspace: '⌫', tab: 'Tab', ' ': 'Space', space: 'Space',
};

/** `mod+shift+z` -> `⌘⇧Z`. Text that is already a glyph (`⌘K`, `Esc`, `↵`) passes through. */
function cap(part: string, MOD: Record<string, string>) {
  const bits = part.length > 1 ? part.split(/\+(?=.)/) : [part];
  const key = bits.pop()!;
  const k = NAME[key.toLowerCase()] ?? (key.length === 1 ? key.toUpperCase() : /^f\d+$/i.test(key) ? key.toUpperCase() : key);
  return bits.map((m) => MOD[m.toLowerCase()] ?? m).join('') + k;
}

/** One keycap per key press of each sequence: `g i` -> G, I. */
export const keycaps = (keys: string | string[], platform?: string) => {
  const MOD = isMac(platform) ? MAC_MOD : OTHER_MOD;
  return [keys].flat().flatMap((s) => s.split(/\s+/).filter(Boolean).map((p) => cap(p, MOD)));
};
