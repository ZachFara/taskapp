import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { keysOf, listCommands, type Ctx } from './index';

// Keeps the keyboard tables in docs/GUIDE.md in step with the registry: a row names a command id in backticks,
// and its key cell lists the keys in backticks. Other tables (dialogs, command menu) are not checked.
const guide = readFileSync(resolve(import.meta.dirname, '../../../../docs/GUIDE.md'), 'utf8');
const rows = guide
  .split('\n')
  .map((line) => line.split('|').slice(1, -1).map((cell) => cell.trim()))
  .filter((cells) => /^`[a-z][\w.-]*`$/.test(cells[0] ?? '') && cells.length >= 3)
  .map(([id, , keys]) => ({ id: id.slice(1, -1), keys: [...keys.matchAll(/`([^`]+)`/g)].map((m) => m[1]).sort() }));

const all = listCommands({} as Ctx, { all: true });
const registered = new Map(all.map((c) => [c.id, keysOf(c).slice().sort()]));

describe('docs/GUIDE.md keyboard tables match the command registry', () => {
  it('finds the keyboard tables', () => {
    expect(rows.length).toBeGreaterThan(50);
  });

  it('every row names a registered command', () => {
    expect(rows.filter((r) => !registered.has(r.id)).map((r) => r.id)).toEqual([]);
  });

  it('every row lists exactly the keys the command has', () => {
    const wrong = rows
      .filter((r) => registered.has(r.id))
      .filter((r) => JSON.stringify(r.keys) !== JSON.stringify(registered.get(r.id)))
      .map((r) => `${r.id}: guide ${JSON.stringify(r.keys)} registry ${JSON.stringify(registered.get(r.id))}`);
    expect(wrong).toEqual([]);
  });

  it('every command with a key has a row', () => {
    const documented = new Set(rows.map((r) => r.id));
    const missing = all.filter((c) => keysOf(c).length && !documented.has(c.id)).map((c) => c.id);
    expect(missing).toEqual([]);
  });
});
