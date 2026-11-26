import { describe, expect, it } from 'vitest';
import { fixtures } from '../../lib/api.mock';
import type { Filter } from '../../lib/bindings';
import { defaultsFromFilter, mergeIssueDefaults } from './defaults';

const ctx = {
  statuses: fixtures.statuses,
  projects: fixtures.projects,
  propertyDefs: fixtures.propertyDefs,
  defaultStatusId: 1,
};

describe('defaultsFromFilter', () => {
  it('extracts_status_project_priority_and_labels_from_and_filter', () => {
    const filter: Filter = {
      And: [
        { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [3] } },
        { Cond: { field: { Fixed: 'Project' }, op: 'Eq', value: 2 } },
        { Cond: { field: { Fixed: 'Priority' }, op: 'Eq', value: 1 } },
        { Cond: { field: { Fixed: 'Label' }, op: 'In', value: [2, 5] } },
      ],
    };
    expect(defaultsFromFilter(filter, ctx)).toEqual({
      status_id: 3,
      project_id: 2,
      priority: 1,
      label_ids: [2],
    });
  });

  it('picks_first_matching_status_when_status_category_excludes_default', () => {
    const filter: Filter = {
      Cond: { field: { Fixed: 'StatusCategory' }, op: 'In', value: ['unstarted', 'started'] },
    };
    // defaultStatusId is 1 (Backlog), so Todo (2, unstarted) is chosen.
    expect(defaultsFromFilter(filter, ctx)).toEqual({ status_id: 2 });
    // When defaultStatusId already satisfies the filter, keep it.
    expect(defaultsFromFilter(filter, { ...ctx, defaultStatusId: 3 })).toEqual({ status_id: 3 });
  });

  it('keeps_default_status_when_in_multi_status_filter_otherwise_picks_first', () => {
    const filter: Filter = { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [3, 2] } };
    expect(defaultsFromFilter(filter, ctx)).toEqual({ status_id: 3 });
    expect(defaultsFromFilter(filter, { ...ctx, defaultStatusId: 2 })).toEqual({ status_id: 2 });
  });

  it('extracts_cycle_milestone_parent_dates_estimates_points_and_properties', () => {
    const filter: Filter = {
      And: [
        { Cond: { field: { Fixed: 'Cycle' }, op: 'Eq', value: 1 } },
        { Cond: { field: { Fixed: 'Milestone' }, op: 'Eq', value: 2 } },
        { Cond: { field: { Fixed: 'Parent' }, op: 'Eq', value: 7 } },
        { Cond: { field: { Fixed: 'StartDate' }, op: 'Eq', value: '2026-11-26' } },
        { Cond: { field: { Fixed: 'DueDate' }, op: 'Eq', value: '2026-11-30' } },
        { Cond: { field: { Fixed: 'Estimate' }, op: 'Eq', value: 60 } },
        { Cond: { field: { Fixed: 'Points' }, op: 'Eq', value: 5 } },
        { Cond: { field: { Property: 'area' }, op: 'In', value: ['work'] } },
      ],
    };
    expect(defaultsFromFilter(filter, ctx)).toEqual({
      cycle_id: 1,
      milestone_id: 2,
      parent_id: 7,
      start_date: '2026-11-26',
      due: { Date: '2026-11-30' },
      estimate_minutes: 60,
      points: 5,
      properties: { area: 'work' },
    });
  });

  it('handles_status_exclusion_filters_when_default_would_be_hidden', () => {
    const filter: Filter = {
      Cond: { field: { Fixed: 'StatusCategory' }, op: 'NotIn', value: ['completed', 'canceled'] },
    };
    expect(defaultsFromFilter(filter, ctx)).toEqual({});
    expect(defaultsFromFilter(filter, { ...ctx, defaultStatusId: 4 })).toEqual({ status_id: 1 });
  });

  it('combines_status_constraints_regardless_of_filter_order', () => {
    const positive: Filter = { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [1, 2, 3] } };
    const category: Filter = { Cond: { field: { Fixed: 'StatusCategory' }, op: 'In', value: ['started', 'unstarted'] } };
    const excluded: Filter = { Cond: { field: { Fixed: 'Status' }, op: 'NotIn', value: [2] } };
    expect(defaultsFromFilter({ And: [positive, category, excluded] }, ctx)).toEqual({ status_id: 3 });
    expect(defaultsFromFilter({ And: [excluded, category, positive] }, ctx)).toEqual({ status_id: 3 });
  });

  it('ignores_multi_branch_or_and_merges_overrides', () => {
    const orFilter: Filter = {
      Or: [
        { Cond: { field: { Fixed: 'Status' }, op: 'In', value: [1] } },
        { Cond: { field: { Fixed: 'Project' }, op: 'In', value: [2] } },
      ],
    };
    expect(defaultsFromFilter(orFilter, ctx)).toEqual({});
    expect(
      mergeIssueDefaults(
        { status_id: 2, project_id: 1, label_ids: [1], properties: { area: 'work' } },
        { status_id: 3, label_ids: [3], properties: { effort: 5 } },
      ),
    ).toEqual({
      status_id: 3,
      project_id: 1,
      label_ids: [1, 3],
      properties: { area: 'work', effort: 5 },
    });

  });

  it('empty_group_defaults_clear_inherited_labels_and_properties', () => {
    expect(mergeIssueDefaults(
      { label_ids: [1], properties: { area: 'work', score: 2 } },
      { label_ids: [], properties: { area: null } },
    )).toEqual({ label_ids: [], properties: { score: 2 } });
  });
});
