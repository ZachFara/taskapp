// Drag and drop of sidebar rows with pointer events. HTML5 drag events do not fire reliably in the Mac app's web view
// (it takes file drops for itself), so this follows pointerdown, pointermove and pointerup on its own.
import type { FolderSection } from './bindings';

export type Src = { section: FolderSection; type: 'item' | 'folder'; id: number; label: string; container: number | null; pos: number };
/** Where a drop lands: inside `container` (null: the section's top level) at `index` among its items or folders (null: the end). */
export type Drop = { section: FolderSection; container: number | null; index: number | null; highlight: number | null; line: { type: 'item' | 'folder'; id: number; edge: 'before' | 'after' } | null };
export type Over = { section: FolderSection; type: 'item' | 'folder' | 'area'; id: number; container: number | null; pos: number; open: boolean };

/** The drag in progress; rows read it to draw the insertion line and the highlighted folder. */
export const dnd = $state<{ src: Src | null; drop: Drop | null }>({ src: null, drop: null });

const THRESHOLD = 4;
const OPEN_AFTER = 600;

/**
 * Turns a pointer position over a row into a drop. `frac` is how far down the row the pointer is (0 to 1).
 * A folder dragged onto a folder goes before it (top quarter), after it (bottom quarter) or inside it; anything else
 * dropped on a folder goes inside. Between items the half decides. `fits` says whether a folder may go inside a container.
 */
export function resolve(src: Src, over: Over, frac: number, fits: (folder: number, container: number | null) => boolean): Drop | null {
  if (over.section !== src.section || (over.type === src.type && over.id === src.id)) return null;
  // Index among the siblings once the dragged row is out of its list, which is how the backend counts.
  const at = (type: Src['type'], container: number | null, i: number) => i - (src.type === type && src.container === container && src.pos < i ? 1 : 0);
  const done = (d: Drop) => (src.type === 'folder' && !fits(src.id, d.container) ? null : d);
  const base = { section: src.section, highlight: null, line: null };
  if (over.type === 'area') return done({ ...base, container: null, index: null });
  if (over.type === 'item') {
    if (src.type === 'folder') return done({ ...base, container: over.container, index: null, highlight: over.container });
    const after = frac >= 0.5;
    return done({ ...base, container: over.container, index: at('item', over.container, over.pos + (after ? 1 : 0)), line: { type: 'item', id: over.id, edge: after ? 'after' : 'before' } });
  }
  const inside = done({ ...base, container: over.id, index: null, highlight: over.id });
  if (src.type === 'item') return inside;
  // An open folder shows its children right below it, so a line under it would read as "first child".
  if (frac < 0.25 || (frac > 0.75 && !over.open)) {
    const after = frac > 0.5;
    return done({ ...base, container: over.container, index: at('folder', over.container, over.pos + (after ? 1 : 0)), line: { type: 'folder', id: over.id, edge: after ? 'after' : 'before' } });
  }
  return inside;
}

type Options = { onDrop: (src: Src, drop: Drop) => void; open: (folder: number) => void; fits: (folder: number, container: number | null) => boolean };

const num = (v: string | undefined) => (v === undefined || v === '' ? null : Number(v));

/**
 * Svelte action for the sidebar. Rows say what they are with data attributes: `data-drag` (item or folder),
 * `data-section`, `data-id`, `data-container` (the folder they sit in, empty at the top), `data-pos` (place among
 * their siblings of that kind), `data-label`, `data-open` on a folder with its children showing. An element with
 * `data-area` and `data-section` is the section's empty space (the top level).
 */
export function dragRows(nav: HTMLElement, opts: Options) {
  let options = opts;
  let start: { x: number; y: number; row: HTMLElement } | null = null;
  let ghost: HTMLElement | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;
  let hovered: number | null = null;

  const srcOf = (row: HTMLElement): Src => ({
    section: row.dataset.section as FolderSection,
    type: row.dataset.drag as Src['type'],
    id: Number(row.dataset.id),
    label: row.dataset.label ?? '',
    container: num(row.dataset.container),
    pos: Number(row.dataset.pos ?? 0),
  });

  function hover(folder: number | null, shut: boolean) {
    if (folder === hovered) return;
    clearTimeout(timer);
    hovered = folder;
    if (folder !== null && shut) timer = setTimeout(() => options.open(folder), OPEN_AFTER);
  }

  function finish() {
    removeEventListener('pointermove', move, true);
    removeEventListener('pointerup', up, true);
    removeEventListener('pointercancel', cancel, true);
    removeEventListener('keydown', key, true);
    clearTimeout(timer);
    hovered = null;
    ghost?.remove();
    ghost = null;
    start = null;
    dnd.src = null;
    dnd.drop = null;
    document.body.classList.remove('dnd-active');
  }
  // The click that ends a drag must not open the row it started on.
  const swallowClick = () => {
    const stop = (e: Event) => (e.preventDefault(), e.stopPropagation());
    addEventListener('click', stop, { capture: true, once: true });
    setTimeout(() => removeEventListener('click', stop, true), 0);
  };

  function move(e: MouseEvent) {
    if (!start) return;
    if (!dnd.src) {
      if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < THRESHOLD) return;
      dnd.src = srcOf(start.row);
      ghost = document.createElement('div');
      ghost.className = 'dnd-ghost';
      ghost.textContent = dnd.src.label;
      document.body.append(ghost);
      document.body.classList.add('dnd-active');
    }
    if (ghost) ghost.style.transform = `translate(${e.clientX + 12}px, ${e.clientY + 8}px)`;
    const el = (e.target as Element | null)?.closest?.<HTMLElement>('[data-drag], [data-area]');
    if (!el || !nav.contains(el)) {
      dnd.drop = null;
      return hover(null, false);
    }
    const r = el.getBoundingClientRect();
    const over: Over = el.dataset.area !== undefined
      ? { section: el.dataset.section as FolderSection, type: 'area', id: 0, container: null, pos: 0, open: false }
      : { ...srcOf(el), open: el.dataset.open !== undefined };
    dnd.drop = resolve(dnd.src, over, r.height ? (e.clientY - r.top) / r.height : 0.5, options.fits);
    const into = dnd.drop?.highlight ?? null;
    hover(into, into !== null && over.type === 'folder' && !over.open);
  }
  function up() {
    const [src, drop] = [dnd.src, dnd.drop];
    finish();
    if (!src) return;
    swallowClick();
    if (drop) options.onDrop(src, drop);
  }
  function cancel() {
    const was = dnd.src;
    finish();
    if (was) swallowClick();
  }
  const key = (e: KeyboardEvent) => e.key === 'Escape' && (e.preventDefault(), cancel());

  function down(e: MouseEvent) {
    const row = (e.target as Element | null)?.closest?.<HTMLElement>('[data-drag]');
    if (e.button !== 0 || !row || !nav.contains(row)) return;
    start = { x: e.clientX, y: e.clientY, row };
    addEventListener('pointermove', move, true);
    addEventListener('pointerup', up, true);
    addEventListener('pointercancel', cancel, true);
    addEventListener('keydown', key, true);
  }
  nav.addEventListener('pointerdown', down);
  return {
    update: (next: Options) => (options = next),
    destroy() {
      nav.removeEventListener('pointerdown', down);
      finish();
    },
  };
}
