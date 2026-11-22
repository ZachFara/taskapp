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

  it('has at least 40 cases', () => {
    expect(cases.length).toBeGreaterThanOrEqual(40);
  });

  cases.forEach((c, i) =>
    it(c.name, () => {
      expect(kind(mock[i])).toEqual(kind(real[i]));
    }),
  );
});
