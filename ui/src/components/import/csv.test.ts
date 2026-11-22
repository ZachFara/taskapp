import { describe, expect, it } from 'vitest';
import { parseCsv } from './csv';

describe('parseCsv (preview)', () => {
  const cases: [string, string, string[][]][] = [
    ['plain rows', 'a,b\n1,2', [['a', 'b'], ['1', '2']]],
    ['quoted comma stays in the field', 'a,"b, c",d', [['a', 'b, c', 'd']]],
    ['doubled quote is one quote', '"say ""hi""",x', [['say "hi"', 'x']]],
    ['newline inside quotes stays in the field', 'a,"line 1\nline 2"\nb,c', [['a', 'line 1\nline 2'], ['b', 'c']]],
    ['CRLF line endings', 'a,b\r\n1,2\r\n', [['a', 'b'], ['1', '2']]],
    ['empty fields are kept', 'a,,c', [['a', '', 'c']]],
    ['blank lines are dropped', 'a\n\n\nb\n', [['a'], ['b']]],
    ['a quote inside an unquoted field is literal', 'a"b,c', [['a"b', 'c']]],
    ['no trailing newline still yields the last row', 'x,y', [['x', 'y']]],
    ['empty input has no rows', '', []],
  ];
  it.each(cases)('%s', (_name, input, expected) => {
    expect(parseCsv(input)).toEqual(expected);
  });
});
