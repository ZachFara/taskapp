import { readdirSync, readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { listCommands, type Ctx } from './index';

// Every shortcutFor('<id>') in the UI must name a registered command, or its tooltip shows nothing.
const srcDir = resolve(import.meta.dirname, '../..');
const files = readdirSync(srcDir, { recursive: true, encoding: 'utf8' })
  .filter((f) => /\.(svelte|ts)$/.test(f) && !/\.test\.ts$/.test(f))
  .map((f) => join(srcDir, f));
const refs = files.flatMap((f) =>
  [...readFileSync(f, 'utf8').matchAll(/shortcutFor\(\s*'([^']+)'\s*\)/g)].map((m) => ({ id: m[1], file: f })),
);
const ids = new Set(listCommands({} as Ctx, { all: true }).map((c) => c.id));

describe('shortcutFor references', () => {
  it('finds the references', () => {
    expect(refs.length).toBeGreaterThan(5);
  });

  it('every id names a registered command', () => {
    expect(refs.filter((r) => !ids.has(r.id)).map((r) => `${r.id} in ${r.file}`)).toEqual([]);
  });
});
