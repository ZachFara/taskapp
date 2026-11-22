import { beforeAll, describe, expect, it } from 'vitest';
import { cases } from './cases';
import { haveDevserver, type Outcome, runMock, runReal } from './run';

// Locally a missing binary skips this with a loud message; in CI it is a failure.
if (!haveDevserver && process.env.CI) throw new Error('taskapp-devserver is not built: run `cargo build -p taskapp-devserver` before the UI tests');
if (!haveDevserver) console.error('\n*** CONTRACT TESTS SKIPPED: build the real side with `cargo build -p taskapp-devserver` ***\n');

const kind = (o: Outcome) => ('error' in o ? { error: o.error.kind, message: o.error.message } : o);

describe.skipIf(!haveDevserver)('mock matches the real backend', () => {
  let mock: Outcome[];
  let real: Outcome[];
  beforeAll(async () => {
    [mock, real] = [await runMock(cases), await runReal(cases)];
  }, 120_000);

  it('has at least 40 comparable cases', () => {
    expect(cases.filter((c) => !c.pending).length).toBeGreaterThanOrEqual(40);
  });

  cases.forEach((c, i) =>
    it(c.name, () => {
      if (!c.pending) return expect(kind(mock[i])).toEqual(kind(real[i]));
      // TODO(P3): drop `pending` once the backend has the command; until then this fails loudly the day it appears.
      expect(mock[i], `mock result for ${c.cmd}`).toEqual({ ok: c.pending.expectMock });
      expect('error' in real[i] && real[i].error.kind, `${c.cmd} exists on the real side now: remove pending from this case (${c.pending.by})`).toBe('NotFound');
    }),
  );
});
