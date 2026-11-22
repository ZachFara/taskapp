// Port of crates/core/src/import_csv.rs (plan) and the planning half of import_plan.rs.
import type { ImportPlan, PlanIssue, PropertyDef, Value } from './bindings';

const fail = (message: string): never => {
  throw { kind: 'Invalid', message: `invalid: ${message}` };
};

export type Known = {
  statuses: string[];
  labels: string[];
  projects: string[];
  propertyKeys: string[];
  cycleStarts: string[];
  imported: Set<string>;
};

const FIELDS = ['title', 'description', 'status', 'priority', 'project', 'labels', 'start_date', 'due_date', 'estimate'];
const isDate = (s: string) => /^\d{4}-\d{2}-\d{2}$/.test(s) && !Number.isNaN(Date.parse(s)) && new Date(s).toISOString().slice(0, 10) === s;

/** RFC 4180: quoted fields, `""` escapes, newlines inside quotes. Blank lines are dropped. */
function parseCsv(text: string): string[][] {
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = '';
  let quoted = false;
  let wasQuoted = false;
  const chars = [...text.replace(/^﻿+/, '')];
  for (let i = 0; i < chars.length; i++) {
    const c = chars[i];
    if (c === '"' && quoted && chars[i + 1] === '"') { cell += '"'; i++; }
    else if (c === '"' && quoted) quoted = false;
    else if (c === '"' && cell === '' && !wasQuoted) { quoted = true; wasQuoted = true; }
    else if (c === ',' && !quoted) { row.push(cell); cell = ''; wasQuoted = false; }
    else if (c === '\r' && !quoted) continue;
    else if (c === '\n' && !quoted) {
      row.push(cell); cell = ''; wasQuoted = false;
      if (row.some((x) => x !== '')) rows.push(row);
      row = [];
    } else cell += c;
  }
  if (quoted) fail('the file ends inside a quoted field');
  row.push(cell);
  if (row.some((x) => x !== '')) rows.push(row);
  return rows;
}

function guessMapping(headers: string[]): [string, string][] {
  const by: Record<string, string> = {
    title: 'title', name: 'title', summary: 'title', description: 'description', body: 'description', details: 'description',
    notes: 'description', status: 'status', state: 'status', priority: 'priority', project: 'project', labels: 'labels',
    label: 'labels', tags: 'labels', start: 'start_date', 'start date': 'start_date', due: 'due_date', 'due date': 'due_date',
    deadline: 'due_date', estimate: 'estimate', points: 'estimate',
  };
  return headers.flatMap((h): [string, string][] => {
    const f = by[h.trim().toLowerCase().replace(/[_-]/g, ' ')];
    return f ? [[h, f]] : [];
  });
}

const slugify = (name: string, sep: string) => {
  let out = '';
  for (const ch of [...name.toLowerCase()]) {
    if (/[a-z0-9]/.test(ch)) out += ch;
    else if (out && !out.endsWith(sep)) out += sep;
  }
  out = out.slice(0, 36);
  while (out.endsWith(sep)) out = out.slice(0, -1);
  return out;
};
export const projectSlug = (name: string) => slugify(name, '-') || 'project';
const propertyKey = (name: string) => (slugify(name, '_').match(/^[a-z]/) ? slugify(name, '_') : `p_${slugify(name, '_')}`);

const PRIORITY: Record<string, number> = { '': 0, none: 0, urgent: 1, high: 2, medium: 3, low: 4 };
function priority(s: string): number {
  const p = PRIORITY[s.toLowerCase()];
  if (p !== undefined) return p;
  if (/^\d+$/.test(s) && Number(s) <= 4) return Number(s);
  throw `\`${s}\` is not a priority (0 to 4, urgent, high, medium, low)`;
}

const num = (v: string) => v.trim() !== '' && Number.isFinite(Number(v));
function infer(name: string, values: string[]): PropertyDef {
  const filled = values.filter((v) => v !== '');
  const uniq = [...new Set(filled)];
  let kind: PropertyDef['kind'] = 'Text';
  let options: PropertyDef['options'] = [];
  if (filled.length && filled.every(num)) kind = 'Number';
  else if (filled.length && filled.every(isDate)) kind = 'Date';
  else if (filled.length && uniq.length <= 6 && filled.length >= 3 * uniq.length) {
    kind = 'Select';
    options = uniq.map((value) => ({ value, color: '#95a2b3' }));
  }
  return { key: propertyKey(name), name, kind, options, project_id: null, position: 0 };
}

export function planCsv(text: string, mappingIn: [string, string][] | null, known: Known): Required<ImportPlan> {
  const rows = parseCsv(text);
  const headers = rows[0] ?? fail('the file is empty');
  const mapping = new Map(mappingIn ?? guessMapping(headers));
  for (const k of mapping.keys()) if (!headers.includes(k)) fail(`no column named \`${k}\``);
  const cols: [number, string][] = [];
  headers.forEach((h, idx) => {
    const f = mapping.get(h)?.trim();
    if (f === undefined) return;
    if (!FIELDS.includes(f) && !(f.startsWith('property:') && f.slice(9).trim())) {
      fail(`unknown field \`${f}\`; use title, description, status, priority, project, labels, start_date, due_date, estimate or property:<name>`);
    }
    cols.push([idx, f]);
  });
  if (!cols.some(([, f]) => f === 'title')) fail('no column is mapped to title');
  const defs = new Map<string, PropertyDef>();
  for (const [idx, f] of cols) {
    if (f.startsWith('property:')) {
      const name = f.slice(9).trim();
      defs.set(name, infer(name, rows.slice(1).map((r) => (r[idx] ?? '').trim())));
    }
  }
  const plan: Required<ImportPlan> = {
    source: 'csv', statuses: [], labels: [], projects: [], cycles: [], property_defs: [], issues: [], relations: [],
    warnings: [], skipped: [], unmapped: headers.filter((h) => !mapping.has(h)),
  };
  const seen = new Set<string>();
  rows.slice(1).forEach((cells, n) => {
    const row = n + 2;
    const warnings: string[] = [];
    try {
      const i: PlanIssue = {
        external_id: `csv:${cells.join('\u001f')}`, title: '', description: '', status: null, priority: 0, project: null,
        labels: [], start_date: null, due_date: null, estimate_minutes: null, properties: {}, parent: null, cycle: null,
      };
      for (const [idx, f] of cols) {
        const cell = (cells[idx] ?? '').trim();
        if (!cell) continue;
        if (f === 'title') i.title = cell;
        else if (f === 'description') i.description = cells[idx];
        else if (f === 'status') {
          const st = known.statuses.find((x) => x.toLowerCase() === cell.toLowerCase());
          if (st) i.status = st;
          else warnings.push(`unknown status \`${cell}\`; those issues get the default status`);
        } else if (f === 'priority') i.priority = priority(cell);
        else if (f === 'project') i.project = cell;
        else if (f === 'labels') i.labels = cell.split(/[,;]/).map((l) => l.trim()).filter(Boolean);
        else if (f === 'start_date' || f === 'due_date') {
          if (!isDate(cell)) throw `\`${cell}\` is not a YYYY-MM-DD date`;
          i[f] = cell;
        } else if (f === 'estimate') {
          if (!/^\d+$/.test(cell)) throw `\`${cell}\` is not a whole number of minutes`;
          i.estimate_minutes = Number(cell);
        } else {
          const d = defs.get(f.slice(9).trim())!;
          const v: Value = d.kind === 'Number' ? (num(cell) ? Number(cell) : fail(`\`${cell}\` is not a number for ${d.name}`)) : cell;
          if (d.kind === 'Select' && !d.options.some((o) => o.value === v)) throw `${JSON.stringify(v)} is not a valid Select for ${d.key}`;
          if (d.kind === 'Date' && !isDate(cell)) throw `${JSON.stringify(v)} is not a valid Date for ${d.key}`;
          i.properties[d.key] = v;
        }
      }
      if (!i.title) throw 'title is empty';
      if (seen.has(i.external_id)) plan.skipped.push({ row, reason: 'duplicate of an earlier row' });
      else {
        seen.add(i.external_id);
        plan.issues.push(i);
      }
    } catch (e) {
      if (typeof e !== 'string') {
        // A thrown {kind, message} comes from `fail`, which here means a bad cell, not a bad request.
        const m = (e as { message?: string }).message;
        plan.skipped.push({ row, reason: m ?? String(e) });
      } else plan.skipped.push({ row, reason: e });
    }
    for (const w of warnings) if (!plan.warnings.includes(w)) plan.warnings.push(w);
  });
  plan.property_defs = [...defs.values()];
  finish(plan, known);
  return plan;
}

const distinct = (names: string[]) => names.filter((n, i) => names.findIndex((o) => o.toLowerCase() === n.toLowerCase()) === i);
const lacks = (have: string[], name: string) => !have.some((h) => h.toLowerCase() === name.toLowerCase());

/** Narrows a raw plan to what is new and skips issues already imported. */
function finish(plan: Required<ImportPlan>, known: Known) {
  plan.labels = distinct(plan.issues.flatMap((i) => i.labels)).filter((l) => lacks(known.labels, l));
  plan.projects = distinct(plan.issues.flatMap((i) => (i.project ? [i.project] : []))).filter((p) => lacks(known.projects, p));
  plan.property_defs = plan.property_defs.filter((d) => !known.propertyKeys.includes(d.key));
  const fresh: PlanIssue[] = [];
  plan.issues.forEach((i, n) => {
    if (known.imported.has(i.external_id)) plan.skipped.push({ row: n + 1, reason: 'already imported' });
    else fresh.push(i);
  });
  plan.issues = fresh;
}
