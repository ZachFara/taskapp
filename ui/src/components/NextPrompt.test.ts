import { mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { setApi } from '../lib/api';
import { createMockApi } from '../lib/api.mock';
import NextPrompt from './NextPrompt.svelte';

const settle = async () => {
  for (let i = 0; i < 40; i++) await Promise.resolve();
  await tick();
};
let app: ReturnType<typeof mount> | undefined;
afterEach(() => app && unmount(app));

async function setup() {
  const api = createMockApi();
  setApi(api);
  const p = await api.projects.create('fresh', 'Fresh', null, null);
  await api.projects.update({ ...p, status_id: 2 });
  const [a, b] = [await api.issues.create({ title: 'First', project_id: p.id } as never), await api.issues.create({ title: 'Second', project_id: p.id } as never)];
  await api.projects.setNext(p.id, a.id);
  app = mount(NextPrompt, { target: document.body });
  await settle();
  return { api, p, a, b };
}
const dialog = () => document.querySelector('[role="dialog"]');

describe('the finish prompt', () => {
  it('offers the open issues when the flagged one is finished, and flags the one picked', async () => {
    const { api, p, a, b } = await setup();
    expect(dialog()).toBeNull();
    await api.issues.update(a.id, { status_id: 4 } as never);
    await settle();
    expect(dialog()?.textContent).toContain(`${a.key} is finished`);
    const opt = [...document.querySelectorAll<HTMLElement>('[role="option"]')].find((o) => o.textContent?.includes(b.key))!;
    opt.click();
    await settle();
    expect((await api.next.list()).find((n) => n.project.id === p.id)?.issue?.id).toBe(b.id);
  });

  it('Skip leaves the project without a next action', async () => {
    const { api, p, a } = await setup();
    await api.issues.update(a.id, { status_id: 4 } as never);
    await settle();
    [...document.querySelectorAll<HTMLElement>('button')].find((x) => x.textContent?.trim() === 'Skip')!.click();
    await settle();
    expect((await api.next.list()).find((n) => n.project.id === p.id)?.stuck).toBe(true);
  });
});
