import type { FixedField } from './bindings';
import { api } from './api';
import { FIXED, normalize, type FieldName, type IssueField } from './issueFields';
import { pointsOn, readEstimation, type Estimation } from '../components/insights/estimation';

// A rune box, so every screen that asks `fieldOn` redraws when the setting changes.
const box = $state<{ list: IssueField[]; estimation: Estimation }>({ list: normalize(null), estimation: 'both' });

/** The fields in the order the details panel shows them, on or off. */
export const fieldList = () => box.list;
/** Whether a built-in field shows anywhere. Points is also off when the estimation setting has no points. */
export const fieldOn = (name: FieldName) => box.list.find((f) => f.field === name)?.on !== false && (name !== 'points' || pointsOn(box.estimation));
/** The same for a view field; fields that cannot be switched off (title, status, dates created...) are always on. */
export const fixedOn = (f: FixedField | undefined) => !f || !FIXED[f] || fieldOn(FIXED[f]!);

export async function loadFields() {
  try {
    const [raw, est] = await Promise.all([api.settings.get('issue_fields'), readEstimation()]);
    box.list = normalize(raw);
    box.estimation = est.estimation;
  } catch {
    /* no settings api yet: every field stays on */
  }
}

if (typeof window !== 'undefined') {
  void loadFields();
  // A settings write (here or from another window) reaches `history:wrote`.
  addEventListener('history:wrote', () => void loadFields());
}
