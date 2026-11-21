import { openUrl } from '@tauri-apps/plugin-opener';

// Only these schemes leave the app; javascript:, file: and anything else are refused, not opened.
const ALLOWED = /^(obsidian|https|http):\/\//i;

/** Opens an external link in the system handler (desktop) or a new tab (browser). Returns false when refused. */
export async function openExternal(url: string): Promise<boolean> {
  if (!ALLOWED.test(url)) return false;
  if ('__TAURI_INTERNALS__' in window) await openUrl(url);
  else window.open(url, '_blank', 'noopener');
  return true;
}
