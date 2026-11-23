import { afterEach, describe, expect, it, vi } from 'vitest';
import { dnd, dragRows, resolve, type Src } from './pointerDrag.svelte';

const fits = () => true;
const item = (id: number, container: number | null, pos: number): Src => ({ section: 'view', type: 'item', id, label: `v${id}`, container, pos });
const over = (type: 'item' | 'folder' | 'area', id: number, container: number | null, pos: number, open = false) => ({ section: 'view' as const, type, id, container, pos, open });

describe('resolve', () => {
  it('drops an item inside a folder, wherever on the folder row it is', () => {
    for (const y of [0.1, 0.5, 0.9]) expect(resolve(item(1, null, 0), over('folder', 7, null, 0), y, fits)).toMatchObject({ container: 7, index: null, highlight: 7, line: null });
  });
  it('puts an item before or after another by the half, counting the list without the dragged row', () => {
    expect(resolve(item(1, null, 0), over('item', 2, null, 2), 0.2, fits)).toMatchObject({ container: null, index: 1, line: { id: 2, edge: 'before' } });
    expect(resolve(item(1, null, 0), over('item', 2, null, 2), 0.8, fits)).toMatchObject({ index: 2, line: { id: 2, edge: 'after' } });
    expect(resolve(item(1, 5, 3), over('item', 2, null, 0), 0.2, fits)).toMatchObject({ container: null, index: 0 });
  });
  it('lets a folder go before, after or inside a folder, and an open folder has no after line', () => {
    const f = { ...item(1, null, 0), type: 'folder' as const };
    expect(resolve(f, over('folder', 2, null, 1), 0.1, fits)).toMatchObject({ index: 0, line: { type: 'folder', edge: 'before' } });
    expect(resolve(f, over('folder', 2, null, 1), 0.9, fits)).toMatchObject({ index: 1, line: { edge: 'after' } });
    expect(resolve(f, over('folder', 2, null, 1), 0.5, fits)).toMatchObject({ container: 2, highlight: 2 });
    expect(resolve(f, over('folder', 2, null, 1, true), 0.9, fits)).toMatchObject({ container: 2 });
  });
  it('refuses another section, itself, and a folder that does not fit', () => {
    expect(resolve(item(1, null, 0), { ...over('folder', 2, null, 0), section: 'project' }, 0.5, fits)).toBeNull();
    expect(resolve(item(1, null, 0), over('item', 1, null, 0), 0.5, fits)).toBeNull();
    const f = { ...item(1, null, 0), type: 'folder' as const };
    expect(resolve(f, over('folder', 2, null, 1), 0.5, () => false)).toBeNull();
  });
  it('takes the empty space of a section as its top level', () => {
    expect(resolve(item(1, 4, 0), over('area', 0, null, 0), 0.5, fits)).toMatchObject({ container: null, index: null });
  });
});

describe('dragRows', () => {
  let stop = () => {};
  afterEach(() => {
    stop();
    document.body.innerHTML = '';
  });
  function setup() {
    document.body.innerHTML = `<nav><ul>
      <li id="a" data-drag="item" data-section="view" data-id="1" data-label="A" data-pos="0">A</li>
      <li id="f" data-drag="folder" data-section="view" data-id="7" data-label="F" data-pos="0">F</li>
    </ul><div id="area" data-area data-section="view"></div></nav>`;
    const nav = document.querySelector('nav')!;
    const opts = { onDrop: vi.fn(), open: vi.fn(), fits };
    stop = dragRows(nav, opts).destroy;
    const el = (id: string) => document.getElementById(id)!;
    el('f').getBoundingClientRect = () => ({ top: 0, height: 20 }) as DOMRect;
    const fire = (type: string, target: Element, init: MouseEventInit = {}) => target.dispatchEvent(new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, ...init }));
    return { opts, el, fire };
  }

  it('drags an item onto a folder: a ghost follows, the folder is the target, the drop reports it', () => {
    const { opts, el, fire } = setup();
    fire('pointerdown', el('a'), { clientX: 0, clientY: 0 });
    fire('pointermove', el('f'), { clientX: 1, clientY: 1 }); // under the threshold: no drag yet
    expect(dnd.src).toBeNull();
    fire('pointermove', el('f'), { clientX: 10, clientY: 10 });
    expect(dnd.src).toMatchObject({ id: 1, type: 'item' });
    expect(document.querySelector('.dnd-ghost')!.textContent).toBe('A');
    expect(dnd.drop).toMatchObject({ container: 7, highlight: 7 });
    fire('pointerup', el('f'));
    expect(opts.onDrop).toHaveBeenCalledWith(expect.objectContaining({ id: 1 }), expect.objectContaining({ container: 7 }));
    expect(document.querySelector('.dnd-ghost')).toBeNull();
    expect(dnd.src).toBeNull();
  });

  it('a press and release without moving is a click and drops nothing', async () => {
    await new Promise((r) => setTimeout(r, 0)); // the previous drop's click guard has expired
    const { opts, el, fire } = setup();
    fire('pointerdown', el('a'), { clientX: 0, clientY: 0 });
    fire('pointerup', el('a'));
    expect(opts.onDrop).not.toHaveBeenCalled();
    const click = vi.fn();
    el('a').addEventListener('click', click);
    fire('click', el('a'));
    expect(click).toHaveBeenCalled();
  });

  it('Esc cancels the drag and the click that follows does not open the row', () => {
    const { opts, el, fire } = setup();
    const click = vi.fn();
    el('a').addEventListener('click', click);
    fire('pointerdown', el('a'), { clientX: 0, clientY: 0 });
    fire('pointermove', el('f'), { clientX: 10, clientY: 10 });
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(dnd.src).toBeNull();
    fire('pointerup', el('f'));
    fire('click', el('a'));
    expect(opts.onDrop).not.toHaveBeenCalled();
    expect(click).not.toHaveBeenCalled();
  });

  it('hovering a collapsed folder for 600 ms opens it', () => {
    vi.useFakeTimers();
    const { opts, el, fire } = setup();
    fire('pointerdown', el('a'), { clientX: 0, clientY: 0 });
    fire('pointermove', el('f'), { clientX: 10, clientY: 10 });
    vi.advanceTimersByTime(599);
    expect(opts.open).not.toHaveBeenCalled();
    vi.advanceTimersByTime(2);
    expect(opts.open).toHaveBeenCalledWith(7);
    vi.useRealTimers();
  });
});
