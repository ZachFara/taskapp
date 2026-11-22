import { afterEach } from 'vitest';
import { mount, tick, unmount, type Component } from 'svelte';
import { setApi, type Api } from '../../lib/api';
import { createMockApi } from '../../lib/api.mock';

type Fns = Record<string, (...args: unknown[]) => unknown>;
type Overrides = Record<string, Record<string, (...args: unknown[]) => unknown>>;

const mounted: { app: object; target: Element }[] = [];

afterEach(() => {
  for (const { app, target } of mounted.splice(0)) {
    void unmount(app);
    target.remove();
  }
});

const flush = () => new Promise((r) => setTimeout(r, 0)).then(() => tick());

/** Mounts a settings component on a fresh mock api. Every call is recorded as "module.fn" in `calls`. */
export async function mountSettings(component: Component, overrides: Overrides = {}) {
  const mock = createMockApi() as unknown as Record<string, Fns>;
  for (const [module, fns] of Object.entries(overrides)) Object.assign(mock[module], fns);
  const calls: string[] = [];
  for (const [module, fns] of Object.entries(mock)) {
    for (const [name, fn] of Object.entries(fns)) {
      fns[name] = (...args) => {
        calls.push(`${module}.${name}`);
        return fn(...args);
      };
    }
  }
  setApi(mock as unknown as Api);
  const target = document.body.appendChild(document.createElement('main'));
  const app = mount(component, { target });
  mounted.push({ app, target });
  await flush();
  return { target, calls, api: mock as unknown as Api };
}

/** The form control with this aria-label. */
export function field(root: Element, label: string): HTMLInputElement & HTMLSelectElement {
  const el = root.querySelector(`[aria-label="${label}"]`);
  if (!el) throw new Error(`no control labelled "${label}"`);
  return el as HTMLInputElement & HTMLSelectElement;
}

/** Sets a value (or checked state) the way a user would and lets the component react. */
export async function setField(root: Element, label: string, value: string | boolean) {
  const el = field(root, label);
  if (typeof value === 'boolean') el.checked = value;
  else el.value = value;
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
  await flush();
}

/** Clicks the button whose text is exactly `text`, then lets async handlers finish. */
export async function press(root: Element, text: string) {
  const btn = [...root.querySelectorAll('button')].find((b) => b.textContent?.trim() === text);
  if (!btn) throw new Error(`no button "${text}"`);
  btn.click();
  await flush();
}

export const hasButton = (root: Element, text: string) =>
  [...root.querySelectorAll('button')].some((b) => b.textContent?.trim() === text);

// jsdom has no pointer capture; bits-ui calls these when a select opens.
const proto = Element.prototype as unknown as Record<string, unknown>;
proto.hasPointerCapture ??= () => false;
proto.releasePointerCapture ??= () => {};
proto.scrollIntoView ??= () => {};

/** Opens a bits-ui select by its label and picks the option with this text (the list is portalled to body). */
export async function pick(root: Element, label: string, option: string) {
  const trigger = field(root, label);
  trigger.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, pointerType: 'mouse' }));
  await flush();
  const item = [...document.querySelectorAll('[role="option"]')].find((o) => o.textContent?.trim() === option);
  if (!item) throw new Error(`no option "${option}" for "${label}"`);
  item.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, button: 0, pointerType: 'mouse' }));
  item.dispatchEvent(new MouseEvent('click', { bubbles: true }));
  await flush();
}

/** Clicks the switch with this aria-label, the way a user would. */
export async function toggle(root: Element, label: string) {
  field(root, label).click();
  await flush();
}

/** The text of the designed status line for a form, if any. */
export const statusText = (root: Element) => root.querySelector('[data-status]')?.textContent?.trim() ?? '';
