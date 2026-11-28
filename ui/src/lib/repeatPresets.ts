import { parseRule, ruleWords } from './recur';

const DAYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'];

/** The repeat choices for an issue due on `due` (YYYY-MM-DD, or none): the label and the rule text the backend takes. */
export function presets(due: string | null): { label: string; rule: string }[] {
  const at = due ? new Date(`${due}T00:00:00Z`) : new Date();
  const weekday = DAYS[at.getUTCDay()];
  return [
    { label: 'Every day', rule: 'daily' },
    { label: 'Every weekday', rule: 'weekdays' },
    { label: `Every week on ${weekday[0].toUpperCase()}${weekday.slice(1)}`, rule: `every ${weekday}` },
    { label: `Every month on day ${at.getUTCDate()}`, rule: `monthly on ${at.getUTCDate()}` },
    { label: 'Every 2 weeks', rule: 'every 2 weeks' },
    { label: 'Every day, from completion', rule: 'daily from completion' },
    { label: 'Every week, from completion', rule: 'weekly from completion' },
    { label: 'Every month, from completion', rule: 'monthly from completion' },
  ];
}

/** An issue's stored rule in plain words, or null when it does not repeat. */
export const repeatWords = (rule: string | null) => {
  const r = rule === null ? null : parseRule(rule);
  return r && ruleWords(r);
};
