import { describe, expect, it } from 'vitest';

import { buildQuiverVegaSpec } from './quiver';

describe('quiver utils', () => {
  it('builds a hydrated vega-lite spec for quiver', () => {
    const spec = buildQuiverVegaSpec(
      {
        title: 'Pipeline Throughput',
        description: 'Daily throughput by team.',
        primaryTypeId: 'primary',
        secondaryTypeId: 'secondary',
        joinField: 'order_id',
        secondaryJoinField: 'order_id',
        dateField: 'event_date',
        metricField: 'throughput',
        groupField: 'team',
        selectedGroup: 'EMEA',
        chartKind: 'area',
        shared: true,
      },
      [{ date: '2026-04-25', value: 12, count: 4 }],
      [{ group: 'EMEA', value: 12, count: 4 }],
    );

    expect(spec.$schema).toBe('https://vega.github.io/schema/vega-lite/v5.json');
    expect(spec.datasets.timeSeries).toHaveLength(1);
    expect(spec.vconcat[0].mark.type).toBe('area');
    expect(spec.usermeta.quiver.chart_kind).toBe('area');
    expect(spec.params[0].value).toBe('EMEA');
  });
});
