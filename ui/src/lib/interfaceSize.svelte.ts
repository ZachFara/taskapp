// Interface size scales the app with CSS zoom on the app root, so the px tokens scale with it. Per device, like
// the pane widths: a screen's size is a property of the screen.
export const SIZES = [100, 110, 115, 125] as const;
export const DEFAULT = 110;
const KEY = 'cairn.interfaceSize';

export function readSize(): number {
  try {
    const n = Number(localStorage.getItem(KEY));
    return (SIZES as readonly number[]).includes(n) ? n : DEFAULT;
  } catch {
    return DEFAULT;
  }
}

export const interfaceSize = $state<{ size: number }>({ size: readSize() });

// The app root applies `zoom: var(--ui-zoom)`. Zooming the document itself would shift floating popovers and tooltips.
function apply() {
  document.documentElement.style.setProperty('--ui-zoom', String(interfaceSize.size / 100));
}
apply();

export function setInterfaceSize(size: number) {
  interfaceSize.size = size;
  try {
    localStorage.setItem(KEY, String(size));
  } catch {
    // Not remembered without storage; the size still applies.
  }
  apply();
}

/** One step bigger (1) or smaller (-1); the ends hold. */
export function stepInterfaceSize(dir: 1 | -1) {
  const i = SIZES.indexOf(interfaceSize.size as (typeof SIZES)[number]);
  const next = SIZES[Math.min(SIZES.length - 1, Math.max(0, i + dir))];
  setInterfaceSize(next);
}
