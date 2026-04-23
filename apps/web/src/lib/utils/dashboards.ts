export type DashboardWidgetType = 'chart' | 'table' | 'kpi';
export type DashboardChartType = 'bar' | 'line' | 'area' | 'pie' | 'scatter';
export type DashboardNumberFormat = 'number' | 'currency' | 'percent';
export type DashboardDatePreset = 'last_7_days' | 'last_30_days' | 'last_90_days' | 'this_month' | 'quarter_to_date' | 'custom';

export interface DashboardDateRange {
  mode: 'relative' | 'absolute';
  preset: DashboardDatePreset;
  from: string;
  to: string;
}

export interface DashboardFilterState {
  search: string;
  dateRange: DashboardDateRange;
}

export interface DashboardWidgetLayout {
  colSpan: number;
  rowSpan: number;
}

export interface DashboardWidgetQuery {
  sql: string;
  limit: number;
}

interface DashboardWidgetBase {
  id: string;
  type: DashboardWidgetType;
  title: string;
  description: string;
  layout: DashboardWidgetLayout;
  query: DashboardWidgetQuery;
}

export interface DashboardChartWidget extends DashboardWidgetBase {
  type: 'chart';
  chartType: DashboardChartType;
  categoryColumn: string;
  seriesColumns: string[];
  stacked: boolean;
}

export interface DashboardTableWidget extends DashboardWidgetBase {
  type: 'table';
  pageSize: number;
  defaultSortColumn: string;
  defaultSortDirection: 'asc' | 'desc';
}

export interface DashboardKpiWidget extends DashboardWidgetBase {
  type: 'kpi';
  valueColumn: string;
  deltaColumn: string;
  sparklineColumn: string;
  valueFormat: DashboardNumberFormat;
}

export type DashboardWidget = DashboardChartWidget | DashboardTableWidget | DashboardKpiWidget;

export interface DashboardDefinition {
  id: string;
  name: string;
  description: string;
  widgets: DashboardWidget[];
  createdAt: string;
  updatedAt: string;
}

function createId() {
  return crypto.randomUUID();
}

function createDateLabel(offsetDays: number) {
  const date = new Date();
  date.setDate(date.getDate() + offsetDays);
  return date.toISOString().slice(0, 10);
}

function formatDate(date: Date) {
  return date.toISOString().slice(0, 10);
}

export function createDefaultDateRange(): DashboardDateRange {
  const today = new Date();
  const from = new Date(today);
  from.setDate(today.getDate() - 29);

  return {
    mode: 'relative',
    preset: 'last_30_days',
    from: formatDate(from),
    to: formatDate(today),
  };
}

export function createDefaultFilters(): DashboardFilterState {
  return {
    search: '',
    dateRange: createDefaultDateRange(),
  };
}

export function resolveDateRange(value: DashboardDateRange) {
  const today = new Date();

  if (value.mode === 'absolute' || value.preset === 'custom') {
    return {
      from: value.from,
      to: value.to,
      label: `${value.from} -> ${value.to}`,
    };
  }

  const end = formatDate(today);
  const start = new Date(today);

  switch (value.preset) {
    case 'last_7_days':
      start.setDate(today.getDate() - 6);
      break;
    case 'last_30_days':
      start.setDate(today.getDate() - 29);
      break;
    case 'last_90_days':
      start.setDate(today.getDate() - 89);
      break;
    case 'this_month':
      start.setDate(1);
      break;
    case 'quarter_to_date': {
      const quarterStartMonth = Math.floor(today.getMonth() / 3) * 3;
      start.setMonth(quarterStartMonth, 1);
      break;
    }
    default:
      start.setDate(today.getDate() - 29);
      break;
  }

  const from = formatDate(start);
  return {
    from,
    to: end,
    label: `${from} -> ${end}`,
  };
}

function escapeSqlLiteral(value: string) {
  return `'${value.replace(/'/g, "''")}'`;
}

export function applyDashboardQueryTemplate(sql: string, filters: DashboardFilterState) {
  const resolvedRange = resolveDateRange(filters.dateRange);
  const replacements: Record<string, string> = {
    search: escapeSqlLiteral(filters.search),
    date_from: escapeSqlLiteral(resolvedRange.from),
    date_to: escapeSqlLiteral(resolvedRange.to),
  };

  return sql.replace(/\{\{\s*([a-zA-Z0-9_]+)\s*\}\}/g, (match, key) => replacements[key] ?? match);
}

export function toNumber(value: unknown) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }

  if (typeof value === 'string') {
    const cleaned = value.replace(/,/g, '');
    const parsed = Number(cleaned);
    return Number.isFinite(parsed) ? parsed : null;
  }

  return null;
}

export function formatMetricValue(value: unknown, format: DashboardNumberFormat) {
  const numeric = toNumber(value);

  if (numeric === null) {
    return String(value ?? '--');
  }

  if (format === 'currency') {
    return new Intl.NumberFormat('en-US', {
      style: 'currency',
      currency: 'USD',
      maximumFractionDigits: 0,
    }).format(numeric);
  }

  if (format === 'percent') {
    return `${numeric.toFixed(1)}%`;
  }

  return new Intl.NumberFormat('en-US', { maximumFractionDigits: 2 }).format(numeric);
}

export function parseSparklineSeries(value: unknown) {
  if (Array.isArray(value)) {
    return value.map((entry) => toNumber(entry)).filter((entry): entry is number => entry !== null);
  }

  if (typeof value === 'string') {
    try {
      const parsed = JSON.parse(value);
      if (Array.isArray(parsed)) {
        return parsed
          .map((entry) => toNumber(entry))
          .filter((entry): entry is number => entry !== null);
      }
    } catch {
      return [];
    }
  }

  return [];
}

export function cloneDashboard<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}

export function createWidget(type: 'chart'): DashboardChartWidget;
export function createWidget(type: 'table'): DashboardTableWidget;
export function createWidget(type: 'kpi'): DashboardKpiWidget;
export function createWidget(type: DashboardWidgetType): DashboardWidget;
export function createWidget(type: DashboardWidgetType): DashboardWidget {
  if (type === 'table') {
    return {
      id: createId(),
      type,
      title: 'Account Coverage',
      description: 'Sortable operational table with local filtering and pagination.',
      layout: { colSpan: 8, rowSpan: 2 },
      query: {
        limit: 100,
        sql: [
          "SELECT 'Northwind' AS account, 'Enterprise' AS segment, 5820 AS arr, 'Healthy' AS status",
          "UNION ALL SELECT 'Lakehouse Co', 'Mid-market', 4380, 'Watch'",
          "UNION ALL SELECT 'Mercury Health', 'Enterprise', 9010, 'Healthy'",
          "UNION ALL SELECT 'Atlas Retail', 'SMB', 1920, 'Needs follow-up'",
          "UNION ALL SELECT 'Vertex Energy', 'Enterprise', 6640, 'Healthy'",
          "UNION ALL SELECT 'North Star', 'Mid-market', 3270, 'Expansion'",
        ].join(' '),
      },
      pageSize: 5,
      defaultSortColumn: 'arr',
      defaultSortDirection: 'desc',
    };
  }

  if (type === 'kpi') {
    return {
      id: createId(),
      type,
      title: 'Net Revenue',
      description: 'Single-number KPI with delta and sparkline.',
      layout: { colSpan: 4, rowSpan: 1 },
      query: {
        limit: 1,
        sql: "SELECT 18240 AS total_revenue, 12.8 AS delta_pct, '[14200,14880,15120,16050,16820,17640,18240]' AS sparkline",
      },
      valueColumn: 'total_revenue',
      deltaColumn: 'delta_pct',
      sparklineColumn: 'sparkline',
      valueFormat: 'currency',
    };
  }

  return {
    id: createId(),
    type: 'chart',
    title: 'Pipeline Throughput',
    description: 'ECharts-powered trend view sourced from SQL.',
    layout: { colSpan: 8, rowSpan: 2 },
    query: {
      limit: 50,
      sql: [
        "SELECT 'Mon' AS bucket, 124 AS ingested, 108 AS published",
        "UNION ALL SELECT 'Tue', 152, 131",
        "UNION ALL SELECT 'Wed', 148, 140",
        "UNION ALL SELECT 'Thu', 166, 150",
        "UNION ALL SELECT 'Fri', 190, 172",
        "UNION ALL SELECT 'Sat', 142, 134",
        "UNION ALL SELECT 'Sun', 118, 109",
      ].join(' '),
    },
    chartType: 'area',
    categoryColumn: 'bucket',
    seriesColumns: ['ingested', 'published'],
    stacked: false,
  };
}

export function createDashboard(name = 'New Dashboard'): DashboardDefinition {
  const now = new Date().toISOString();
  return {
    id: createId(),
    name,
    description: 'Compose charts, tables, and KPI cards on a responsive grid.',
    widgets: [createWidget('kpi'), createWidget('chart'), createWidget('table')],
    createdAt: now,
    updatedAt: now,
  };
}

export function createStarterDashboards() {
  const executive = createDashboard('Executive Control Room');
  executive.description = 'A shareable baseline dashboard for pipeline health, revenue, and account coverage.';

  const operations = createDashboard('Operations Review');
  operations.widgets = [
    {
      ...createWidget('chart'),
      title: 'Run Volume by Day',
      chartType: 'bar',
      seriesColumns: ['ingested'],
      description: 'Daily run volume for the current reporting window.',
    },
    {
      ...createWidget('kpi'),
      title: 'Successful Runs',
      valueColumn: 'total_revenue',
      valueFormat: 'number',
      description: 'Swap the query to your own run metrics and reuse the same KPI card.',
      query: {
        limit: 1,
        sql: "SELECT 842 AS total_revenue, 4.2 AS delta_pct, '[790,802,815,821,829,836,842]' AS sparkline",
      },
    },
    {
      ...createWidget('table'),
      title: 'Escalations Queue',
      defaultSortColumn: 'priority',
      query: {
        limit: 100,
        sql: [
          "SELECT 'PIPE-1042' AS incident, 'High' AS priority, 'SLA breach risk' AS summary",
          "UNION ALL SELECT 'AUTH-288', 'Medium', 'SSO callback drift'",
          "UNION ALL SELECT 'DATA-931', 'Low', 'Late-arriving file partition'",
          "UNION ALL SELECT 'OPS-512', 'High', 'Backfill validation pending'",
        ].join(' '),
      },
    },
  ];

  return [executive, operations];
}

export function duplicateDashboardDefinition(dashboard: DashboardDefinition) {
  const copy = cloneDashboard(dashboard);
  const now = new Date().toISOString();

  return {
    ...copy,
    id: createId(),
    name: `${dashboard.name} Copy`,
    createdAt: now,
    updatedAt: now,
    widgets: copy.widgets.map((widget) => ({ ...widget, id: createId() })),
  };
}

function base64Encode(value: string) {
  if (typeof btoa === 'function') {
    return btoa(value);
  }

  return Buffer.from(value, 'utf-8').toString('base64');
}

function base64Decode(value: string) {
  if (typeof atob === 'function') {
    return atob(value);
  }

  return Buffer.from(value, 'base64').toString('utf-8');
}

export function serializeDashboardSnapshot(dashboard: DashboardDefinition) {
  return encodeURIComponent(base64Encode(JSON.stringify(dashboard)));
}

export function deserializeDashboardSnapshot(snapshot: string) {
  const decoded = base64Decode(decodeURIComponent(snapshot));
  return JSON.parse(decoded) as DashboardDefinition;
}

export function formatDashboardTimestamp(value: string) {
  return new Intl.DateTimeFormat('en-US', {
    month: 'short',
    day: 'numeric',
    hour: 'numeric',
    minute: '2-digit',
  }).format(new Date(value));
}

export function createWidgetPreviewSql(type: DashboardWidgetType) {
  return createWidget(type).query.sql;
}

export function defaultAbsoluteRange() {
  return {
    from: createDateLabel(-29),
    to: createDateLabel(0),
  };
}