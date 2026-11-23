import { openUrl } from '@tauri-apps/plugin-opener';

/** Only http, https, mailto, and obsidian://open with vault and file parameters, leave the app. Everything else is
 * refused: javascript:, file:, other obsidian actions (new, plugin commands) and any extra parameter such as content. */
export function isAllowedUrl(url: string): boolean {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.protocol === 'http:' || u.protocol === 'https:' || u.protocol === 'mailto:') return true;
  return u.protocol === 'obsidian:' && u.hostname === 'open' && [...u.searchParams.keys()].every((k) => k === 'vault' || k === 'file');
}

/** Opens an external link in the system handler (desktop) or a new tab (browser). Returns false when refused. */
export async function openExternal(url: string): Promise<boolean> {
  if (!isAllowedUrl(url)) return false;
  if ('__TAURI_INTERNALS__' in window) await openUrl(url);
  else window.open(url, '_blank', 'noopener');
  return true;
}
