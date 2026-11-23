import { type ChildProcess, execFileSync, spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { resolve } from 'node:path';
import { chromium, expect as pw, type Browser, type Page } from '@playwright/test';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';

// The first-minute flows, driven as a user would in the mock app, asserting on what is on screen.
const uiDir = resolve(import.meta.dirname, '..');
let vite: ChildProcess;
let browser: Browser;
let base = '';
let page: Page;

const freePort = () =>
  new Promise<number>((ok, fail) => {
    const s = createServer().once('error', fail).listen(0, '127.0.0.1', () => {
      const { port } = s.address() as { port: number };
      s.close(() => ok(port));
    });
  });

beforeAll(async () => {
  execFileSync('pnpm', ['exec', 'playwright', 'install', 'chromium'], { cwd: uiDir });
  const port = await freePort();
  base = `http://localhost:${port}`;
  vite = spawn('pnpm', ['exec', 'vite', '--host', 'localhost', '--port', String(port), '--strictPort'], {
    cwd: uiDir, env: { ...process.env, VITE_API_MODE: 'mock' }, stdio: 'ignore', detached: true,
  });
  let up = false;
  for (let i = 0; i < 300 && !up; i++) {
    up = await fetch(base).then(() => true, () => false);
    if (!up) await new Promise((r) => setTimeout(r, 200));
  }
  if (!up) throw new Error(`vite did not answer on ${base}`);
  browser = await chromium.launch();
  // The first load compiles every module, which is slower than the 5s page timeout the tests use; pay that here.
  const warm = await browser.newPage();
  await warm.goto(base, { timeout: 90_000 });
  await warm.close();
}, 180_000);

afterAll(async () => {
  await browser?.close();
  if (vite?.pid) try { process.kill(-vite.pid); } catch { /* already gone */ }
});

async function open(path = '/view/1', viewport = { width: 1440, height: 900 }) {
  await page?.close();
  page = await browser.newPage({ viewport });
  page.setDefaultTimeout(5_000);
  await page.goto(base + path);
  await page.locator('[data-key], aside.sidebar').first().waitFor();
  if (path.startsWith('/view/')) await page.locator('li[data-key], li[data-issue], tr[data-key]').first().waitFor();
}
const press = async (...keys: string[]) => {
  for (const k of keys) await page.keyboard.press(k);
};
const rows = () => page.locator('li[data-key]');
const focusedKey = () => page.locator('li[data-key][aria-current=true], [data-focused]').first().getAttribute('data-key');
const palette = () => page.getByRole('dialog', { name: 'Command menu' });
const statusOf = (key: string) => rows().filter({ has: page.locator(`.key:text-is("${key}")`) }).getByRole('button', { name: /^Status/ }).getAttribute('aria-label');


describe('walk views', { timeout: 25_000 }, () => {
  it('R3-1 clicking Add a description focuses the textarea and ? types a ?', async () => {
    await open('/issue/ABC-2');
    await page.getByRole('button', { name: 'Edit description' }).click();
    await page.waitForTimeout(200);
    expect(await page.evaluate(() => document.activeElement?.tagName)).toBe('TEXTAREA');
    await page.keyboard.type('a?');
    expect(await page.getByLabel('Description text').inputValue()).toContain('a?');
    expect(await page.getByRole('dialog', { name: /shortcuts/i }).count()).toBe(0);
  });

  it('R3-9 cmd-click selects without opening the panel and sets aria-selected', async () => {
    await open();
    await rows().nth(0).click({ modifiers: ['Meta'] });
    await rows().nth(1).click({ modifiers: ['Meta'] });
    await page.waitForTimeout(200);
    expect(await page.locator('aside.panel').count()).toBe(0);
    expect(await page.locator('li[data-key][aria-selected=true]').count()).toBe(2);
    expect(await page.locator('li[data-key][aria-selected=false]').count()).toBeGreaterThan(0);
  });

  it('R4-09 quick add: p1 is Urgent and the due chip is a human date', async () => {
    await open();
    await press('c');
    await page.getByLabel('Title, with quick-add tokens').fill('Ship it p1 due 2026-11-27');
    await page.waitForTimeout(300);
    expect(await page.locator('[data-chip][data-kind=priority]').getAttribute('aria-label')).toContain('Urgent');
    expect(await page.locator('[data-chip][data-kind=due]').innerText()).toContain('Nov 27');
  });

  it('R4-05 side panel order: description before the properties block', async () => {
    await open();
    await rows().first().click();
    await press('Enter');
    await page.locator('aside.panel [aria-label=Description]').waitFor();
    const d = await page.locator('aside.panel section[aria-label=Description]').boundingBox();
    const p = await page.locator('aside.panel [aria-label=Details]').boundingBox();
    expect(d!.y).toBeLessThan(p!.y);
  });

  it('R4-01 1024 with the panel open keeps the title column and the header inside the sheet', async () => {
    await open('/view/1', { width: 1024, height: 768 });
    await rows().first().click();
    await press('Enter');
    await page.locator('aside.panel').waitFor();
    await page.waitForTimeout(300);
    const t = await rows().first().locator('.title .text').first().boundingBox();
    expect(t!.width).toBeGreaterThan(120);
    const close = await page.locator('aside.panel').getByRole('button', { name: 'Close issue' }).boundingBox();
    expect(close!.x + close!.width).toBeLessThanOrEqual(1024);
    const key = await page.locator('aside.panel .key').boundingBox();
    expect(key!.height).toBeLessThan(30);
  });

  it('R4-02 table at 1024 scrolls inside itself with the title column sticky', async () => {
    await open('/view/1/Table', { width: 1024, height: 768 });
    const sc = page.locator('main.body');
    expect(await sc.evaluate((e) => e.scrollWidth > e.clientWidth)).toBe(true);
    expect(await page.locator('th[data-sticky], td[data-sticky]').count()).toBeGreaterThan(0);
  });

  it('R3-4 Sort, Display, row status, filter remove, group add and view tabs show a tooltip with the shortcut', async () => {
    await open();
    const tip = async (loc: import('@playwright/test').Locator, text: RegExp) => {
      await page.mouse.move(0, 0);
      await page.waitForTimeout(150);
      await loc.hover();
      await page.locator('.cx-tip').first().waitFor({ timeout: 2500 });
      expect(await page.locator('.cx-tip').first().innerText()).toMatch(text);
    };
    await tip(page.getByRole('button', { name: /^Sort/ }), /Sort/);
    await tip(page.getByRole('button', { name: /^Display/ }), /Display/);
    await tip(page.getByRole('button', { name: /^Group/ }), /Group/);
    await tip(rows().first().getByRole('button', { name: /^Status/ }), /Status/);
    await tip(page.getByRole('button', { name: /^Remove filter/ }).first(), /Remove filter/);
  });

  it('events: description.edit focuses the editor; view.rename opens the rename form', async () => {
    await open('/issue/ABC-2');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:issue.description.edit')));
    await page.waitForTimeout(200);
    expect(await page.evaluate(() => document.activeElement?.tagName)).toBe('TEXTAREA');
    for (const [ev, sel] of [['issue.comment.focus', 'TEXTAREA'], ['issue.subissue.add', 'INPUT'], ['issue.relation.add', 'INPUT']]) {
      await page.evaluate((e) => dispatchEvent(new CustomEvent(`command:${e}`)), ev);
      await page.waitForTimeout(150);
      expect(await page.evaluate(() => document.activeElement?.tagName), ev).toBe(sel);
    }
    await open();
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:view.rename')));
    await page.getByLabel('View name').waitFor();
  });
});

describe('walk groups', { timeout: 25_000 }, () => {
  it('group.collapse hides the focused row group; group.collapse.all and group.add act on the list', async () => {
    await open();
    const before = await rows().count();
    await press('j');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBeLessThan(before);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse.all')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBe(0);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.collapse.all')));
    await page.waitForTimeout(150);
    expect(await rows().count()).toBe(before);
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:group.add')));
    await page.waitForTimeout(300);
    expect(await rows().count()).toBe(before + 1);
  });
});

describe('walk project', { timeout: 25_000 }, () => {
  it('project and milestone events open their editors', async () => {
    await open('/project/website');
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:project.rename')));
    await page.getByLabel('Project name').waitFor();
    await page.evaluate(() => dispatchEvent(new CustomEvent('command:milestone.new')));
    await page.getByLabel('Milestone name').waitFor();
  });

  // Priority means the same on every screen: whichever way it was set, the list, table, board and detail all say Urgent.
  // The mock has no db-changed event, so each tab switch mounts a view that reads the rows fresh.
  const urgentEverywhere = async (title: string) => {
    const mark = (loc: import('@playwright/test').Locator) => loc.locator('[role=img][aria-label=Urgent][data-level="4"]').first();
    const row = () => page.locator('li[data-key]').filter({ hasText: title });
    await pw(mark(row())).toBeVisible();
    await page.getByRole('tab', { name: 'Table' }).click();
    await pw(mark(page.locator('tr[data-key]').filter({ hasText: title }))).toBeVisible();
    await page.getByRole('tab', { name: 'Board' }).click();
    await pw(mark(page.locator('[data-issue]').filter({ hasText: title }))).toBeVisible();
    await page.getByRole('tab', { name: 'List' }).click();
    await row().first().click();
    await pw(page.locator('aside.panel').getByText('Urgent').first()).toBeVisible();
  };

  it('R4-09 priority Urgent from the detail panel shows Urgent in list, table, board and detail', async () => {
    await open();
    await rows().first().click();
    await press('Enter');
    const title = (await page.locator('aside.panel h2').first().innerText()).trim();
    await page.locator('aside.panel').getByRole('button', { name: /^Priority/ }).click();
    await page.getByRole('option', { name: 'Urgent' }).click();
    await urgentEverywhere(title);
  });

  it('R4-09 priority Urgent from the p command shows Urgent in list, table, board and detail', async () => {
    await open();
    await rows().first().click();
    const title = (await rows().first().locator('.title').first().innerText()).trim();
    await press('p');
    await palette().getByRole('option', { name: 'Urgent' }).click();
    await urgentEverywhere(title);
  });

  it('R4-09 priority Urgent from quick add p1 shows Urgent in list, table, board and detail', async () => {
    await open();
    await press('c');
    await page.getByLabel('Title, with quick-add tokens').fill('Walk urgent p1');
    await press('Enter');
    await pw(page.getByRole('status').getByRole('button', { name: /Undo/ })).toBeVisible();
    // A new issue starts in the first status, which the Active view hides; move it into view.
    await page.getByRole('button', { name: /^Remove filter/ }).first().click();
    await page.locator('li[data-key]').filter({ hasText: 'Walk urgent' }).first().click();
    await press('s');
    await palette().getByRole('option', { name: 'Todo' }).click();
    await urgentEverywhere('Walk urgent');
  });

  it('R4-01 below 1100px the issue panel overlays the list instead of squeezing it', async () => {
    await open('/view/1', { width: 1024, height: 768 });
    const before = (await page.locator('.main').boundingBox())!.width;
    await rows().first().click();
    const panel = page.locator('aside.panel');
    await panel.waitFor();
    expect(await panel.evaluate((e) => getComputedStyle(e).position)).toBe('absolute');
    expect((await page.locator('.main').boundingBox())!.width).toBe(before);
    await open('/view/1', { width: 1440, height: 900 });
    await rows().first().click();
    await page.locator('aside.panel').waitFor();
    expect(await page.locator('aside.panel').evaluate((e) => getComputedStyle(e).position)).toBe('relative');
  });

  it('R3-4 group header add and collapse, and the layout tabs, show a tooltip with the name', async () => {
    await open();
    const tip = async (loc: import('@playwright/test').Locator, text: RegExp) => {
      await page.mouse.move(0, 0);
      await page.waitForTimeout(150);
      await loc.hover();
      await page.locator('.cx-tip').first().waitFor({ timeout: 2500 });
      expect(await page.locator('.cx-tip').first().innerText()).toMatch(text);
    };
    await tip(page.getByRole('button', { name: /^Add to/ }).first(), /Add issue/);
    await tip(page.getByRole('button', { name: /^Collapse/ }).first(), /Collapse group/);
    await tip(page.getByRole('tab', { name: 'Board' }), /Board/);
    expect(await page.getByRole('tab', { name: 'Board' }).getAttribute('data-state')).toBe('inactive');
    expect(await page.getByRole('tab', { name: 'List' }).getAttribute('data-state')).toBe('active');
  });

  it('R4-12 calendar: month grid fills the sheet; week opens at working hours; Week and Month show their names at 1024', async () => {
    await open('/view/1/Calendar');
    const grid = (await page.locator('.calendar .body').boundingBox())!;
    const last = (await page.locator('.grid[data-calendar] .ec-day').last().boundingBox())!;
    expect(last.y + last.height).toBeGreaterThan(grid.y + grid.height - 80);
    await page.getByRole('tab', { name: 'Week' }).click();
    await page.waitForTimeout(300);
    expect(await page.locator('.grid[data-calendar] .ec-main').evaluate((e) => e.scrollTop)).toBeGreaterThan(0);
    await open('/view/1/Calendar', { width: 1024, height: 768 });
    expect(await page.getByRole('tab', { name: 'Week' }).innerText()).toContain('Week');
    const g = (await page.locator('.grid[data-calendar]').boundingBox())!;
    const side = (await page.locator('aside.side').boundingBox())!;
    expect(side.y).toBeGreaterThanOrEqual(g.y + g.height - 1);
  });

  it('R4-13 board: the toggle in the count row is labelled and the board ends inside the sheet', async () => {
    await open('/view/2');
    await page.locator('[data-column]').first().waitFor();
    expect((await page.locator('.count').filter({ hasText: /issues/ }).locator('..').locator('button').first().innerText()).trim()).not.toBe('');
    const body = (await page.locator('main.body').boundingBox())!;
    const board = (await page.locator('.board').boundingBox())!;
    expect(board.y + board.height).toBeLessThanOrEqual(body.y + body.height + 1);
  });

  it('R4-20 the Group popover is a list with the current choice marked; the Display panel keeps a gutter', async () => {
    await open();
    await page.getByRole('button', { name: /^Group/ }).click();
    const list = page.getByRole('listbox', { name: 'Group by' });
    await list.waitFor();
    expect(await list.getByRole('option', { selected: true }).innerText()).toContain('Status');
    await list.getByRole('option', { name: 'Project' }).click();
    await pw(page.locator('.band .name', { hasText: 'Website relaunch' }).first()).toBeVisible();
    await press('Escape');
    await page.getByRole('button', { name: /^Display/ }).click();
    const box = (await page.locator('.cx-popover').last().boundingBox())!;
    expect(1440 - (box.x + box.width)).toBeGreaterThanOrEqual(8);
  });

  it('the app fills the window at every interface size (no empty band under it)', async () => {
    await open();
    for (const zoom of ['1', '1.1', '1.25']) {
      await page.evaluate((z) => document.documentElement.style.setProperty('--ui-zoom', z), zoom);
      const gap = await page.evaluate(() => {
        const r = document.querySelector('.app')!.getBoundingClientRect();
        return [Math.round(r.top), Math.round(window.innerHeight - r.bottom), Math.round(window.innerWidth - r.right)];
      });
      expect(gap, `zoom ${zoom}`).toEqual([0, 0, 0]);
    }
    await page.evaluate(() => document.documentElement.style.removeProperty('--ui-zoom'));
  });

  it('R4-23 the comment box has no resize grip and a disabled primary button is plainly disabled', async () => {
    await open('/issue/ABC-2');
    const box = page.getByLabel('Leave a comment (markdown)').or(page.locator('textarea').last());
    expect(await box.evaluate((e) => getComputedStyle(e).resize)).toBe('none');
    const send = page.getByRole('button', { name: 'Comment', exact: true });
    expect(await send.isDisabled()).toBe(true);
    expect(await send.evaluate((e) => getComputedStyle(e).opacity)).toBe('1');
  });

  it('R5 filter and rename saves raise an undo toast; layout and display toggles stay quiet', async () => {
    await open();
    await page.getByRole('tab', { name: 'Table' }).click();
    await page.waitForTimeout(900);
    expect(await page.locator('.toast').allInnerTexts()).toEqual([]);
    await page.getByRole('button', { name: /^Remove filter/ }).first().click();
    await pw(page.locator('.toast')).toContainText('Changed filter of');
    await pw(page.locator('.toast').getByRole('button', { name: /Undo/ })).toBeVisible();
    await page.getByRole('button', { name: 'Rename view' }).click();
    await page.getByLabel('View name').fill('Renamed once');
    await page.getByRole('button', { name: 'Rename', exact: true }).click();
    await pw(page.locator('.toast')).toContainText('Renamed view to Renamed once');
  });
});
