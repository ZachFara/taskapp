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
