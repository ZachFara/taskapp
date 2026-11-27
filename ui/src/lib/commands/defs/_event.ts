import type { Ctx } from '../registry';

/** A command whose screen owns the action: it fires `command:<id>` and the screen that is mounted listens. */
export const emit = (id: string, detail?: unknown) => () => void dispatchEvent(new CustomEvent(`command:${id}`, { detail }));

export const onView = (ctx: Ctx) => ctx.route.startsWith('/view/');
/** Event actions: on the event page, and on the events list for the event last right-clicked. */
export const onEvent = (ctx: Ctx) => ctx.route.startsWith('/event/') || ctx.route === '/events';
/** Group actions need a layout that groups rows. */
export const onGrouped = (ctx: Ctx) => onView(ctx) && ctx.layout !== 'calendar' && ctx.layout !== 'timeline';
export const onIssue = (ctx: Ctx) => ctx.route.startsWith('/issue/') || ctx.openIssueId !== null;
export const onProject = (ctx: Ctx) => ctx.route.startsWith('/project/');
