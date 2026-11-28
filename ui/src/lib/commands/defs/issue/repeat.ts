import { defineCommand } from '../../registry';
import { parseRule, ruleWords } from '../../../recur';
import { presets } from '../../../repeatPresets';
import { apply, choices, pageOf, patchOf, targets } from './_issue';

// Presets are built after the issues loaded, so "every week on <day>" follows the due date of the target.
defineCommand({
  id: 'issue.repeat', label: 'Repeat', icon: 'cycle', group: 'Issue',
  noShortcutReason: 'Reached from the Repeat row of the issue details and the command menu.',
  page: (ctx) => pageOf(ctx, 'Repeat', () => {
    const issue = choices.issues.find((i) => i.id === targets(ctx)[0]);
    const due = issue?.due ? (issue.due.Date ?? issue.due.DateTime?.utc.slice(0, 10) ?? null) : null;
    return [
      ...presets(due).map((p): [string, ReturnType<typeof patchOf>] => [p.label, patchOf({ repeat: p.rule })]),
      ['Stop repeating', patchOf({ repeat: null })],
    ];
  }, {
    // Typed text such as "every 2 weeks on mon" or "monthly on 15 from completion".
    parse: (text) => {
      const r = parseRule(text);
      return r && { label: `Repeat: ${ruleWords(r)}`, run: () => apply(ctx, patchOf({ repeat: text })) };
    },
  }),
});
