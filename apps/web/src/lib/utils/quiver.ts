import type { QuiverChartKind } from '$lib/api/ontology';

export interface QuiverTimeSeriesRow {
  date: string;
  value: number;
  count: number;
}

export interface QuiverGroupedRow {
  group: string;
  value: number;
  count: number;
}

export interface QuiverVegaDraft {
  title: string;
  description: string;
  primaryTypeId: string;
  secondaryTypeId: string;
  joinField: string;
  secondaryJoinField: string;
  dateField: string;
  metricField: string;
  groupField: string;
  selectedGroup: string;
  chartKind: QuiverChartKind;
  shared: boolean;
}

export function buildQuiverVegaSpec(
  draft: QuiverVegaDraft,
  timeSeriesRows: QuiverTimeSeriesRow[],
  groupedRows: QuiverGroupedRow[],
) {
  return {
    $schema: 'https://vega.github.io/schema/vega-lite/v5.json',
    description: draft.description || `Quiver visual function '${draft.title}' generated from ontology analytics.`,
    title: {
      text: draft.title,
      subtitle: `${draft.dateField} • metric ${draft.metricField} • group ${draft.groupField}`,
      anchor: 'start',
    },
    spacing: 20,
    background: '#ffffff',
    datasets: {
      timeSeries: timeSeriesRows,
      grouped: groupedRows,
    },
    params: [
      {
        name: 'selectedGroup',
        value: draft.selectedGroup,
      },
    ],
    vconcat: [
      {
        data: { name: 'timeSeries' },
        mark:
          draft.chartKind === 'area'
            ? {
                type: 'area',
                line: true,
                point: true,
                interpolate: 'monotone',
                opacity: 0.22,
              }
            : {
                type: draft.chartKind,
                point: draft.chartKind !== 'bar',
                interpolate: 'monotone',
              },
        encoding: {
          x: {
            field: 'date',
            type: 'temporal',
            title: draft.dateField,
          },
          y: {
            field: 'value',
            type: 'quantitative',
            title: draft.metricField,
          },
          tooltip: [
            { field: 'date', type: 'temporal', title: draft.dateField },
            { field: 'value', type: 'quantitative', title: draft.metricField },
            { field: 'count', type: 'quantitative', title: 'count' },
          ],
        },
      },
      {
        data: { name: 'grouped' },
        mark: {
          type: 'bar',
          cornerRadiusTopLeft: 6,
          cornerRadiusTopRight: 6,
        },
        encoding: {
          x: {
            field: 'group',
            type: 'nominal',
            sort: '-y',
            title: draft.groupField,
          },
          y: {
            field: 'value',
            type: 'quantitative',
            title: draft.metricField,
          },
          color: {
            field: 'group',
            type: 'nominal',
            legend: null,
          },
          tooltip: [
            { field: 'group', type: 'nominal', title: draft.groupField },
            { field: 'value', type: 'quantitative', title: draft.metricField },
            { field: 'count', type: 'quantitative', title: 'count' },
          ],
        },
      },
    ],
    config: {
      view: { stroke: '#dbe4ea', cornerRadius: 18 },
      axis: {
        labelColor: '#334155',
        titleColor: '#0f172a',
        gridColor: '#e2e8f0',
        tickColor: '#cbd5e1',
      },
      header: {
        titleColor: '#0f172a',
        labelColor: '#475569',
      },
    },
    usermeta: {
      quiver: {
        primary_type_id: draft.primaryTypeId,
        secondary_type_id: draft.secondaryTypeId || null,
        join_field: draft.joinField,
        secondary_join_field: draft.secondaryJoinField,
        date_field: draft.dateField,
        metric_field: draft.metricField,
        group_field: draft.groupField,
        selected_group: draft.selectedGroup || null,
        chart_kind: draft.chartKind,
        shared: draft.shared,
      },
    },
  };
}

export function downloadJsonDocument(fileName: string, value: unknown) {
  const payload = JSON.stringify(value, null, 2);
  const blob = new Blob([payload], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement('a');
  anchor.href = url;
  anchor.download = fileName.endsWith('.json') ? fileName : `${fileName}.json`;
  document.body.append(anchor);
  anchor.click();
  anchor.remove();
  window.setTimeout(() => URL.revokeObjectURL(url), 1_000);
}
