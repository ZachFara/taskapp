// Port of crates/core/src/palette.rs: colours are stored as palette keys, shown with a reference hex.
export const COLORS: [string, string][] = [
  ['gray', '#6b7280'], ['red', '#ef4444'], ['orange', '#f97316'], ['amber', '#f59e0b'], ['yellow', '#eab308'],
  ['green', '#22c55e'], ['teal', '#14b8a6'], ['cyan', '#06b6d4'], ['blue', '#3b82f6'], ['indigo', '#6366f1'],
  ['purple', '#a855f7'], ['pink', '#ec4899'],
];

export const hexOf = (key: string) => COLORS.find(([k]) => k === key)?.[1];
/** A stored key outside the palette shows as gray. */
export const resolve = (key: string) => hexOf(key) ?? COLORS[0][1];

const rgb = (hex: string) => (/^#[0-9a-fA-F]{6}$/.test(hex) ? [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)) : null);

/** The palette key for a key or hex (nearest by squared RGB distance); `null` when it is neither. */
export function keyFor(color: string): string | null {
  if (hexOf(color)) return color;
  const c = rgb(color);
  if (!c) return null;
  const dist = ([, ref]: [string, string]) => rgb(ref)!.reduce((sum, v, i) => sum + (v - c[i]) ** 2, 0);
  return COLORS.reduce((best, x) => (dist(x) < dist(best) ? x : best))[0];
}

export const iconOk = (icon: string) => {
  if (icon.startsWith('icon:')) return /^[a-z0-9-]{1,40}$/.test(icon.slice(5));
  if (icon.startsWith('emoji:')) {
    const chars = [...icon.slice(6)];
    return chars.length >= 1 && chars.length <= 8 && chars.every((c) => c.codePointAt(0)! > 127 && !/\p{Cc}/u.test(c));
  }
  return false;
};
