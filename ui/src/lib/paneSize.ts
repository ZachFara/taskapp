/** Pane widths are remembered per device. Storage can be blocked, so every access is guarded. */
export function clampWidth(w: number, min: number, max: number) {
  return Math.round(Math.min(max, Math.max(min, w)));
}

export function readWidth(key: string): number | null {
  try {
    const n = Number(localStorage.getItem(key));
    return Number.isFinite(n) && n > 0 ? n : null;
  } catch {
    return null;
  }
}

export function writeWidth(key: string, w: number | null) {
  try {
    if (w === null) localStorage.removeItem(key);
    else localStorage.setItem(key, String(w));
  } catch {
    // Without storage the width simply is not remembered.
  }
}

/** A remembered on/off choice, such as a collapsed sidebar. */
export function readFlag(key: string) {
  try {
    return localStorage.getItem(key) === '1';
  } catch {
    return false;
  }
}

export function writeFlag(key: string, on: boolean) {
  try {
    localStorage.setItem(key, on ? '1' : '0');
  } catch {
    // Not remembered without storage.
  }
}
