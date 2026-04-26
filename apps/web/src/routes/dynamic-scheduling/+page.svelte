<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    getMachineryInsights,
    getMachineryQueue,
    listObjectTypes,
    listRules,
    updateMachineryQueueItem,
    type MachineryInsight,
    type MachineryQueueItem,
    type MachineryQueueResponse,
    type ObjectType,
    type OntologyRule
  } from '$lib/api/ontology';
  import { notifications } from '$stores/notifications';

  type ViewMode = 'week' | 'day' | 'agenda';

  type ScenarioPlacement = {
    scheduled_for: string;
    required_capability: string;
  };

  type TimelineSegment = {
    key: string;
    start: Date;
    end: Date;
    label: string;
    secondaryLabel: string;
  };

  type SchedulingSuggestion = {
    itemId: string;
    capability: string;
    start: Date;
    end: Date;
    score: number;
    reason: string;
  };

  let objectTypes = $state<ObjectType[]>([]);
  let rules = $state<OntologyRule[]>([]);
  let insights = $state<MachineryInsight[]>([]);
  let queue = $state<MachineryQueueResponse | null>(null);
  let selectedObjectTypeId = $state('');
  let viewMode = $state<ViewMode>('week');
  let horizonStartInput = $state(toDateInput(new Date()));
  let selectedItemId = $state('');
  let draggingItemId = $state('');
  let loading = $state(true);
  let busy = $state(false);
  let error = $state('');
  let scenarioEdits = $state<Record<string, ScenarioPlacement>>({});

  function startOfLocalDay(value: Date) {
    return new Date(value.getFullYear(), value.getMonth(), value.getDate(), 0, 0, 0, 0);
  }

  function toDateInput(value: Date) {
    const year = value.getFullYear();
    const month = String(value.getMonth() + 1).padStart(2, '0');
    const day = String(value.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  function parseDateInput(value: string) {
    if (!value) return startOfLocalDay(new Date());
    const parsed = new Date(`${value}T00:00:00`);
    return Number.isNaN(parsed.getTime()) ? startOfLocalDay(new Date()) : parsed;
  }

  function addMinutes(value: Date, minutes: number) {
    return new Date(value.getTime() + minutes * 60_000);
  }

  function addHours(value: Date, hours: number) {
    return addMinutes(value, hours * 60);
  }

  function addDays(value: Date, days: number) {
    return addHours(value, days * 24);
  }

  function formatTimestamp(value: string | Date | null) {
    if (!value) return 'n/a';
    const date = value instanceof Date ? value : new Date(value);
    return new Intl.DateTimeFormat('en', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    }).format(date);
  }

  function formatDuration(minutes: number) {
    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      const remainder = minutes % 60;
      return remainder === 0 ? `${hours}h` : `${hours}h ${remainder}m`;
    }
    return `${minutes}m`;
  }

  function capabilityLabel(capability: string | null | undefined) {
    const normalized = (capability || 'general').replaceAll('_', ' ');
    return normalized.charAt(0).toUpperCase() + normalized.slice(1);
  }

  function horizonStart() {
    return parseDateInput(horizonStartInput);
  }

  function horizonEnd() {
    return viewMode === 'day' ? addHours(horizonStart(), 24) : addDays(horizonStart(), 7);
  }

  function buildSegments() {
    if (viewMode === 'agenda') return [] as TimelineSegment[];

    const start = horizonStart();
    const stepHours = viewMode === 'day' ? 2 : 6;
    const segmentCount = viewMode === 'day' ? 12 : 28;

    return Array.from({ length: segmentCount }, (_, index) => {
      const segmentStart = addHours(start, stepHours * index);
      const segmentEnd = addHours(segmentStart, stepHours);
      const major = index === 0 || segmentStart.getHours() === 0;
      return {
        key: `${segmentStart.toISOString()}-${index}`,
        start: segmentStart,
        end: segmentEnd,
        label: major
          ? new Intl.DateTimeFormat('en', {
              weekday: 'short',
              month: 'short',
              day: 'numeric'
            }).format(segmentStart)
          : new Intl.DateTimeFormat('en', {
              hour: 'numeric'
            }).format(segmentStart),
        secondaryLabel: new Intl.DateTimeFormat('en', {
          hour: 'numeric'
        }).format(segmentStart)
      };
    });
  }

  function effectiveCapability(item: MachineryQueueItem) {
    return scenarioEdits[item.id]?.required_capability || item.required_capability || 'general';
  }

  function effectiveStart(item: MachineryQueueItem) {
    return new Date(scenarioEdits[item.id]?.scheduled_for || item.scheduled_for);
  }

  function effectiveEnd(item: MachineryQueueItem) {
    return addMinutes(effectiveStart(item), Math.max(item.estimated_duration_minutes, 30));
  }

  function isPendingLike(item: MachineryQueueItem) {
    return item.status === 'pending' || item.status === 'in_progress';
  }

  function timelineItems() {
    return (queue?.data ?? [])
      .filter(isPendingLike)
      .sort((left, right) => effectiveStart(left).getTime() - effectiveStart(right).getTime());
  }

  function agendaItems() {
    return [...(queue?.data ?? [])].sort(
      (left, right) => effectiveStart(left).getTime() - effectiveStart(right).getTime()
    );
  }

  function capabilities() {
    const values = new Set<string>();
    for (const item of timelineItems()) {
      values.add(effectiveCapability(item));
    }
    for (const capability of queue?.recommendation.capability_load ?? []) {
      values.add(capability.capability);
    }
    if (values.size === 0) values.add('general');
    return Array.from(values).sort((left, right) => left.localeCompare(right));
  }

  function rowItems(capability: string) {
    return timelineItems().filter((item) => effectiveCapability(item) === capability);
  }

  function intervalsOverlap(
    leftStart: Date,
    leftEnd: Date,
    rightStart: Date,
    rightEnd: Date
  ) {
    return leftStart < rightEnd && rightStart < leftEnd;
  }

  function itemConflicts(item: MachineryQueueItem) {
    const itemStart = effectiveStart(item);
    const itemEnd = effectiveEnd(item);
    return timelineItems().filter((candidate) => {
      if (candidate.id === item.id) return false;
      if (effectiveCapability(candidate) !== effectiveCapability(item)) return false;
      return intervalsOverlap(itemStart, itemEnd, effectiveStart(candidate), effectiveEnd(candidate));
    });
  }

  function recommendationRank(itemId: string) {
    const rank = queue?.recommendation.recommended_order.indexOf(itemId) ?? -1;
    return rank >= 0 ? rank + 1 : null;
  }

  function itemStyle(item: MachineryQueueItem) {
    const start = effectiveStart(item);
    const end = effectiveEnd(item);
    const horizonStartDate = horizonStart();
    const horizonEndDate = horizonEnd();
    const horizonMs = horizonEndDate.getTime() - horizonStartDate.getTime();
    const clampedStart = Math.max(start.getTime(), horizonStartDate.getTime());
    const clampedEnd = Math.min(end.getTime(), horizonEndDate.getTime());
    const leftPct = ((clampedStart - horizonStartDate.getTime()) / horizonMs) * 100;
    const widthPct = Math.max(((clampedEnd - clampedStart) / horizonMs) * 100, 4);
    return `left:${leftPct}%;width:${widthPct}%;`;
  }

  function rowUtilization(capability: string) {
    const totalMinutes = viewMode === 'day' ? 24 * 60 : 7 * 24 * 60;
    const usedMinutes = rowItems(capability).reduce(
      (sum, item) => sum + Math.max(item.estimated_duration_minutes, 30),
      0
    );
    return Math.min(100, Math.round((usedMinutes / totalMinutes) * 100));
  }

  function stageItemPlacement(itemId: string, capability: string, start: Date) {
    scenarioEdits = {
      ...scenarioEdits,
      [itemId]: {
        required_capability: capability,
        scheduled_for: start.toISOString()
      }
    };
    selectedItemId = itemId;
  }

  function clearScenario() {
    scenarioEdits = {};
  }

  function clearItemScenario(itemId: string) {
    const next = { ...scenarioEdits };
    delete next[itemId];
    scenarioEdits = next;
  }

  function selectedItem() {
    return agendaItems().find((item) => item.id === selectedItemId) ?? null;
  }

  function selectedSuggestions() {
    const item = selectedItem();
    if (!item || viewMode === 'agenda') return [] as SchedulingSuggestion[];

    const durationMinutes = Math.max(item.estimated_duration_minutes, 30);
    const items = timelineItems();
    const horizonEndDate = horizonEnd();
    const suggestions: SchedulingSuggestion[] = [];

    for (const capability of capabilities()) {
      for (const segment of buildSegments()) {
        const candidateStart = segment.start;
        const candidateEnd = addMinutes(candidateStart, durationMinutes);
        if (candidateEnd > horizonEndDate) continue;

        const overlapping = items.some((candidate) => {
          if (candidate.id === item.id) return false;
          if (effectiveCapability(candidate) !== capability) return false;
          return intervalsOverlap(
            candidateStart,
            candidateEnd,
            effectiveStart(candidate),
            effectiveEnd(candidate)
          );
        });

        if (overlapping) continue;

        const sameCapabilityBonus = capability === (item.required_capability || 'general') ? 40 : 0;
        const overduePenalty = item.status === 'pending' && candidateStart < new Date() ? -15 : 0;
        const score = sameCapabilityBonus - overduePenalty - suggestions.length;

        suggestions.push({
          itemId: item.id,
          capability,
          start: candidateStart,
          end: candidateEnd,
          score,
          reason:
            capability === (item.required_capability || 'general')
              ? 'Matches required capability and avoids overlaps.'
              : 'Alternative row with no detected overlap in the visible horizon.'
        });
      }
    }

    return suggestions
      .sort((left, right) => right.score - left.score || left.start.getTime() - right.start.getTime())
      .slice(0, 3);
  }

  function validationSummary(item: MachineryQueueItem) {
    const conflicts = itemConflicts(item);
    const validations = [];

    if (conflicts.length > 0) {
      validations.push({
        label: 'Overlap conflict',
        tone: 'of-status-danger',
        detail: `${conflicts.length} conflicting puck(s) on ${capabilityLabel(effectiveCapability(item))}.`
      });
    } else {
      validations.push({
        label: 'No overlap',
        tone: 'of-status-success',
        detail: 'No conflicting assignment detected for the selected resource lane.'
      });
    }

    if (item.status === 'pending' && effectiveStart(item) < new Date()) {
      validations.push({
        label: 'Overdue',
        tone: 'of-status-warning',
        detail: 'The item is still pending even though its scheduled window has already started.'
      });
    }

    if (Object.keys(item.constraint_snapshot ?? {}).length > 0) {
      validations.push({
        label: 'Constraint snapshot',
        tone: 'of-status-info',
        detail: 'Constraint metadata is attached and visible to the scheduling surface.'
      });
    }

    return validations;
  }

  async function refreshSurface(preferredItemId = selectedItemId) {
    if (!selectedObjectTypeId) return;
    loading = true;
    error = '';

    try {
      const [queueResponse, insightResponse, ruleResponse] = await Promise.all([
        getMachineryQueue({ object_type_id: selectedObjectTypeId }),
        getMachineryInsights({ object_type_id: selectedObjectTypeId }),
        listRules({ object_type_id: selectedObjectTypeId, page: 1, per_page: 100 })
      ]);

      queue = queueResponse;
      insights = insightResponse.data;
      rules = ruleResponse.data;

      if (preferredItemId && queueResponse.data.some((item) => item.id === preferredItemId)) {
        selectedItemId = preferredItemId;
      } else {
        selectedItemId = queueResponse.data[0]?.id ?? '';
      }
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load dynamic scheduling surface';
    } finally {
      loading = false;
    }
  }

  async function load() {
    loading = true;
    error = '';

    try {
      const response = await listObjectTypes({ page: 1, per_page: 100 });
      objectTypes = response.data;
      selectedObjectTypeId = selectedObjectTypeId || response.data[0]?.id || '';
      await refreshSurface();
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load ontology object types';
      loading = false;
    }
  }

  async function transitionItemStatus(itemId: string, status: string) {
    busy = true;
    try {
      await updateMachineryQueueItem(itemId, { status });
      notifications.success(`Queue item moved to ${status.replaceAll('_', ' ')}.`);
      await refreshSurface(itemId);
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : 'Failed to update queue item';
      notifications.error(message);
    } finally {
      busy = false;
    }
  }

  function moveHorizon(step: number) {
    const next = addDays(horizonStart(), step);
    horizonStartInput = toDateInput(next);
  }

  function onDropPlacement(capability: string, segment: TimelineSegment) {
    if (!draggingItemId) return;
    stageItemPlacement(draggingItemId, capability, segment.start);
    draggingItemId = '';
  }

  function timelineHighlight(
    capability: string,
    segment: TimelineSegment,
    suggestions: SchedulingSuggestion[]
  ) {
    return suggestions.some(
      (suggestion) =>
        suggestion.capability === capability &&
        suggestion.start.getTime() === segment.start.getTime()
    );
  }

  const segments = $derived(buildSegments());
  const scenarioCount = $derived(Object.keys(scenarioEdits).length);

  $effect(() => {
    load();
  });
</script>

<svelte:head>
  <title>OpenFoundry - Dynamic Scheduling</title>
</svelte:head>

<div class="space-y-6">
  <section class="overflow-hidden rounded-[30px] border border-[var(--border-default)] bg-[linear-gradient(135deg,#fffdf8_0%,#f6f7ff_45%,#f0fbf7_100%)] shadow-[var(--shadow-panel)]">
    <div class="grid gap-7 px-6 py-7 lg:grid-cols-[minmax(0,1.2fr)_360px] lg:px-8">
      <div>
        <div class="of-eyebrow">Dynamic scheduling</div>
        <h1 class="mt-3 max-w-4xl text-[34px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">
          Schedule ontology work across resource lanes, scenario staging, and constraint-aware queue operations.
        </h1>
        <p class="mt-4 max-w-3xl text-[15px] leading-7 text-[var(--text-muted)]">
          This surface turns the machinery queue into a dedicated scheduling application. Teams can
          visualize schedules and constraints, stage drag-and-drop changes locally, inspect validation
          pressure, and dispatch operational queue transitions from one place.
        </p>

        <div class="mt-6 flex flex-wrap gap-3">
          <a href="/ontology" class="of-btn">
            <Glyph name="ontology" size={16} />
            <span>Back to ontology</span>
          </a>
          <a href="/ontology/graph" class="of-btn">
            <Glyph name="graph" size={16} />
            <span>Open graph</span>
          </a>
        </div>
      </div>

      <div class="rounded-[26px] border border-white/70 bg-white/80 p-5 backdrop-blur">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Core concepts</div>
        <div class="mt-4 grid gap-3">
          <article class="rounded-[18px] border border-[#e3e7ef] bg-[#fbfcfe] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Schedule objects</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
              Queue items behave like schedule pucks with start time, duration, priority, and resource affinity.
            </div>
          </article>
          <article class="rounded-[18px] border border-[#e3e7ef] bg-[#fbfcfe] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Resource rows</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
              Required capabilities become the operational resource lanes used for load and conflict analysis.
            </div>
          </article>
          <article class="rounded-[18px] border border-[#e3e7ef] bg-[#fbfcfe] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Scenario staging</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
              Dragging a puck stages a local what-if plan without mutating the backend queue until you promote an action.
            </div>
          </article>
        </div>
      </div>
    </div>
  </section>

  <section class="of-panel p-5">
    <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_auto] xl:items-end">
      <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
        <label>
          <div class="mb-1 text-sm font-medium text-[var(--text-default)]">Schedule object type</div>
          <select bind:value={selectedObjectTypeId} class="of-select" onchange={() => void refreshSurface()}>
            {#each objectTypes as objectType (objectType.id)}
              <option value={objectType.id}>{objectType.display_name}</option>
            {/each}
          </select>
        </label>

        <label>
          <div class="mb-1 text-sm font-medium text-[var(--text-default)]">View</div>
          <div class="of-pill-toggle">
            <button type="button" data-active={viewMode === 'week'} onclick={() => (viewMode = 'week')}>Week</button>
            <button type="button" data-active={viewMode === 'day'} onclick={() => (viewMode = 'day')}>Day</button>
            <button type="button" data-active={viewMode === 'agenda'} onclick={() => (viewMode = 'agenda')}>Agenda</button>
          </div>
        </label>

        <label>
          <div class="mb-1 text-sm font-medium text-[var(--text-default)]">Horizon start</div>
          <input bind:value={horizonStartInput} type="date" class="of-input" />
        </label>

        <div>
          <div class="mb-1 text-sm font-medium text-[var(--text-default)]">Scenario</div>
          <div class="flex items-center gap-2">
            <span class="of-chip">{scenarioCount} staged</span>
            <button type="button" class="of-btn text-[13px]" onclick={clearScenario} disabled={scenarioCount === 0}>
              Clear
            </button>
          </div>
        </div>
      </div>

      <div class="flex flex-wrap gap-2">
        <button type="button" class="of-btn" onclick={() => moveHorizon(viewMode === 'day' ? -1 : -7)}>
          <span>Previous</span>
        </button>
        <button type="button" class="of-btn" onclick={() => (horizonStartInput = toDateInput(new Date()))}>
          <span>Today</span>
        </button>
        <button type="button" class="of-btn" onclick={() => moveHorizon(viewMode === 'day' ? 1 : 7)}>
          <span>Next</span>
        </button>
        <button type="button" class="of-btn of-btn-primary" onclick={() => void refreshSurface()} disabled={busy}>
          <Glyph name="run" size={15} />
          <span>Refresh queue</span>
        </button>
      </div>
    </div>
  </section>

  {#if error}
    <div class="of-inline-note">{error}</div>
  {/if}

  {#if queue}
    <section class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
      <article class="of-panel px-5 py-4">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Queue depth</div>
        <div class="mt-2 text-[28px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{queue.recommendation.queue_depth}</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">Pending or in-progress pucks in the visible machinery queue.</div>
      </article>
      <article class="of-panel px-5 py-4">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Overdue</div>
        <div class="mt-2 text-[28px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{queue.recommendation.overdue_count}</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">Items whose scheduled window has already started without completion.</div>
      </article>
      <article class="of-panel px-5 py-4">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Capacity load</div>
        <div class="mt-2 text-[28px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{queue.recommendation.total_estimated_minutes}m</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">Estimated runtime minutes stacked in the recommendation engine.</div>
      </article>
      <article class="of-panel px-5 py-4">
        <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Next due</div>
        <div class="mt-2 text-lg font-semibold text-[var(--text-strong)]">{formatTimestamp(queue.recommendation.next_due_at)}</div>
        <div class="mt-1 text-sm text-[var(--text-muted)]">{queue.recommendation.strategy}</div>
      </article>
    </section>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[minmax(0,1fr)_340px]">
    <section class="space-y-4">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Scheduling board</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">
                Drag a puck onto another resource lane or time segment to stage a scenario move.
              </div>
            </div>
            {#if queue}
              <div class="flex flex-wrap gap-2">
                {#each queue.recommendation.capability_load as capability}
                  <span class="of-chip">
                    {capabilityLabel(capability.capability)} {capability.pending_count}
                  </span>
                {/each}
              </div>
            {/if}
          </div>
        </div>

        <div class="px-5 py-5">
          {#if loading}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
              Loading dynamic scheduling workspace...
            </div>
          {:else if !queue || queue.data.length === 0}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-12 text-center text-sm text-[var(--text-muted)]">
              No scheduling queue items are available yet for this ontology object type.
            </div>
          {:else if viewMode === 'agenda'}
            <div class="space-y-3">
              {#each agendaItems() as item (item.id)}
                <article
                  class={`rounded-[18px] border px-4 py-4 ${selectedItemId === item.id ? 'border-[#9fb9ec] bg-[#f8fbff]' : 'border-[var(--border-default)] bg-white'}`}
                >
                  <div class="flex flex-wrap items-start justify-between gap-3">
                    <div>
                      <div class="flex flex-wrap items-center gap-2">
                        <button type="button" class="text-left text-[15px] font-semibold text-[var(--text-strong)]" onclick={() => (selectedItemId = item.id)}>
                          {item.rule_display_name}
                        </button>
                        <span class={`of-chip ${item.status === 'pending' ? 'of-status-warning' : item.status === 'in_progress' ? 'of-status-info' : 'of-status-success'}`}>
                          {item.status.replaceAll('_', ' ')}
                        </span>
                        {#if recommendationRank(item.id)}
                          <span class="of-chip">Rank {recommendationRank(item.id)}</span>
                        {/if}
                        {#if scenarioEdits[item.id]}
                          <span class="of-chip of-status-info">Staged</span>
                        {/if}
                      </div>
                      <div class="mt-2 text-sm text-[var(--text-muted)]">
                        {formatTimestamp(effectiveStart(item))} to {formatTimestamp(effectiveEnd(item))}
                      </div>
                    </div>
                    <div class="text-right text-sm text-[var(--text-muted)]">
                      <div>{capabilityLabel(effectiveCapability(item))}</div>
                      <div class="mt-1">{formatDuration(Math.max(item.estimated_duration_minutes, 30))}</div>
                    </div>
                  </div>
                </article>
              {/each}
            </div>
          {:else}
            <div class="space-y-4 overflow-x-auto pb-2">
              <div
                class="min-w-[980px]"
                style={`display:grid;grid-template-columns:220px minmax(0,1fr);gap:0;`}
              >
                <div class="border-b border-[var(--border-default)] bg-[#f8fbff] px-4 py-3 text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">
                  Resource rows
                </div>
                <div>
                  <div
                    class="grid border-b border-[var(--border-default)]"
                    style={`grid-template-columns: repeat(${segments.length}, minmax(0, 1fr));`}
                  >
                    {#each segments as segment, index (segment.key)}
                      <div class={`border-l border-[var(--border-subtle)] px-2 py-3 text-center ${index === 0 ? 'border-l-0' : ''}`}>
                        <div class="text-xs font-semibold text-[var(--text-default)]">{segment.label}</div>
                        <div class="mt-1 text-[11px] text-[var(--text-soft)]">{segment.secondaryLabel}</div>
                      </div>
                    {/each}
                  </div>
                </div>

                {#each capabilities() as capability}
                  <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-4">
                    <div class="text-sm font-semibold text-[var(--text-strong)]">{capabilityLabel(capability)}</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">{rowItems(capability).length} scheduled item(s)</div>
                    <div class="mt-3">
                      <div class="h-2 rounded-full bg-[#eef2f7]">
                        <div class="h-2 rounded-full bg-[#7aa2e8]" style={`width:${rowUtilization(capability)}%;`}></div>
                      </div>
                      <div class="mt-2 text-xs text-[var(--text-soft)]">{rowUtilization(capability)}% visible load</div>
                    </div>
                  </div>

                  <div class="border-b border-[var(--border-subtle)] bg-white px-0 py-0">
                    <div class="relative h-[92px]">
                      <div
                        class="grid h-full"
                        style={`grid-template-columns: repeat(${segments.length}, minmax(0, 1fr));`}
                      >
                        {#each segments as segment, index (segment.key)}
                          <div
                            class={`border-l border-[var(--border-subtle)] ${index === 0 ? 'border-l-0' : ''} ${timelineHighlight(capability, segment, selectedSuggestions()) ? 'bg-[#eef6ff]' : ''}`}
                            role="button"
                            tabindex="-1"
                            aria-label={`Drop ${capabilityLabel(capability)} puck at ${formatTimestamp(segment.start)}`}
                            ondragover={(event) => event.preventDefault()}
                            ondrop={() => onDropPlacement(capability, segment)}
                          ></div>
                        {/each}
                      </div>

                      {#each rowItems(capability) as item (item.id)}
                        <button
                          type="button"
                          draggable="true"
                          class={`absolute top-3 h-[54px] overflow-hidden rounded-[16px] border px-3 py-2 text-left shadow-sm transition ${selectedItemId === item.id ? 'z-20 border-[#5c8fe3] bg-[#eaf2ff]' : item.status === 'in_progress' ? 'z-10 border-[#8ad3c6] bg-[#f2fcfa]' : 'z-10 border-[#dfd8c8] bg-[#fff7e7]'} ${itemConflicts(item).length > 0 ? 'ring-2 ring-[#ef9b9b]' : ''}`}
                          style={itemStyle(item)}
                          onclick={() => (selectedItemId = item.id)}
                          ondragstart={() => {
                            draggingItemId = item.id;
                            selectedItemId = item.id;
                          }}
                          ondragend={() => (draggingItemId = '')}
                        >
                          <div class="flex items-center justify-between gap-2">
                            <span class="truncate text-[13px] font-semibold text-[var(--text-strong)]">
                              {item.rule_display_name}
                            </span>
                            {#if recommendationRank(item.id)}
                              <span class="rounded-full bg-white/80 px-2 py-0.5 text-[10px] font-semibold text-[var(--text-muted)]">
                                #{recommendationRank(item.id)}
                              </span>
                            {/if}
                          </div>
                          <div class="mt-1 truncate text-[11px] text-[var(--text-muted)]">
                            {formatDuration(Math.max(item.estimated_duration_minutes, 30))} • {formatTimestamp(effectiveStart(item))}
                          </div>
                          {#if scenarioEdits[item.id]}
                            <div class="mt-2 text-[10px] font-semibold uppercase tracking-[0.14em] text-[#2b5bb7]">
                              Scenario staged
                            </div>
                          {/if}
                        </button>
                      {/each}
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </section>

      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Validation rules and queue pressure</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">
            Constraint signals that shape scheduling recommendations and overload visibility.
          </div>
        </div>

        <div class="grid gap-4 px-5 py-5 lg:grid-cols-[minmax(0,1fr)_320px]">
          <div class="space-y-3">
            {#if insights.length === 0}
              <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">
                No dynamic scheduling insights have been generated yet.
              </div>
            {:else}
              {#each insights as insight (insight.rule_id)}
                <article class="rounded-[18px] border border-[var(--border-default)] bg-white px-4 py-4">
                  <div class="flex items-start justify-between gap-3">
                    <div>
                      <div class="text-sm font-semibold text-[var(--text-strong)]">{insight.display_name}</div>
                      <div class="mt-1 text-sm text-[var(--text-muted)]">
                        {insight.pending_schedules} pending • {insight.overdue_schedules} overdue • {insight.matched_runs}/{insight.total_runs} matched runs
                      </div>
                    </div>
                    <span class={`of-chip ${insight.dynamic_pressure === 'elevated' || insight.dynamic_pressure === 'critical' ? 'of-status-warning' : 'of-status-success'}`}>
                      {insight.dynamic_pressure}
                    </span>
                  </div>
                </article>
              {/each}
            {/if}
          </div>

          <div class="rounded-[22px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Builder guidance</div>
            <ul class="mt-3 space-y-2 text-sm leading-6 text-[var(--text-muted)]">
              <li>Use resource lanes to model the rows of the scheduling board.</li>
              <li>Use queue items as schedule pucks with duration and priority.</li>
              <li>Use rule pressure and conflict detection as validation rules.</li>
              <li>Use scenario staging before promoting queue changes into runtime operations.</li>
            </ul>
            <div class="mt-4 rounded-[16px] border border-dashed border-[#d8e2f3] bg-white px-4 py-4 text-sm leading-6 text-[var(--text-muted)]">
              Rules configured for this object type: <span class="font-semibold text-[var(--text-strong)]">{rules.length}</span>.
            </div>
          </div>
        </div>
      </section>
    </section>

    <aside class="space-y-4">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-4 py-4">
          <div class="of-heading-sm">Selected puck</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">Inspect validations, recommendations, and queue actions.</div>
        </div>

        <div class="space-y-4 px-4 py-4">
          {#if selectedItem()}
            {@const item = selectedItem()!}
            <div>
              <div class="text-[16px] font-semibold text-[var(--text-strong)]">{item.rule_display_name}</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">{capabilityLabel(effectiveCapability(item))}</div>
              <div class="mt-3 flex flex-wrap gap-2">
                <span class={`of-chip ${item.status === 'pending' ? 'of-status-warning' : item.status === 'in_progress' ? 'of-status-info' : 'of-status-success'}`}>
                  {item.status.replaceAll('_', ' ')}
                </span>
                {#if scenarioEdits[item.id]}
                  <span class="of-chip of-status-info">Scenario staged</span>
                {/if}
                {#if recommendationRank(item.id)}
                  <span class="of-chip">Recommendation #{recommendationRank(item.id)}</span>
                {/if}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
              <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Schedule</div>
              <div class="mt-2 text-sm text-[var(--text-default)]">{formatTimestamp(effectiveStart(item))}</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">
                {formatDuration(Math.max(item.estimated_duration_minutes, 30))} • ends {formatTimestamp(effectiveEnd(item))}
              </div>
            </div>

            <div class="space-y-2">
              <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Validation rules</div>
              {#each validationSummary(item) as validation}
                <article class="rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-3">
                  <div class="flex items-center gap-2">
                    <span class={`of-chip ${validation.tone}`}>{validation.label}</span>
                  </div>
                  <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{validation.detail}</div>
                </article>
              {/each}
            </div>

            <div class="space-y-2">
              <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Suggestion function</div>
              {#if selectedSuggestions().length === 0}
                <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-4 text-sm text-[var(--text-muted)]">
                  No visible slot suggestions are available in the current horizon.
                </div>
              {:else}
                {#each selectedSuggestions() as suggestion}
                  <button
                    type="button"
                    class="w-full rounded-[16px] border border-[var(--border-default)] bg-white px-4 py-3 text-left transition hover:border-[#b8cae8] hover:bg-[#f8fbff]"
                    onclick={() => stageItemPlacement(item.id, suggestion.capability, suggestion.start)}
                  >
                    <div class="flex items-center justify-between gap-3">
                      <div class="text-sm font-semibold text-[var(--text-strong)]">{capabilityLabel(suggestion.capability)}</div>
                      <span class="of-chip">Score {suggestion.score}</span>
                    </div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">
                      {formatTimestamp(suggestion.start)} to {formatTimestamp(suggestion.end)}
                    </div>
                    <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{suggestion.reason}</div>
                  </button>
                {/each}
              {/if}
            </div>

            <div class="space-y-2">
              <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Operational actions</div>
              <div class="grid gap-2 sm:grid-cols-2">
                <button type="button" class="of-btn of-btn-primary justify-center" onclick={() => void transitionItemStatus(item.id, 'in_progress')} disabled={busy}>
                  Start
                </button>
                <button type="button" class="of-btn justify-center" onclick={() => void transitionItemStatus(item.id, 'completed')} disabled={busy}>
                  Complete
                </button>
                <button type="button" class="of-btn justify-center" onclick={() => void transitionItemStatus(item.id, 'pending')} disabled={busy}>
                  Reset
                </button>
                <button type="button" class="of-btn justify-center" onclick={() => void transitionItemStatus(item.id, 'cancelled')} disabled={busy}>
                  Cancel
                </button>
              </div>
              <button type="button" class="of-btn w-full justify-center" onclick={() => clearItemScenario(item.id)} disabled={!scenarioEdits[item.id]}>
                Clear staged move
              </button>
            </div>
          {:else}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">
              Select a queue puck to inspect validation rules, suggestions, and actions.
            </div>
          {/if}
        </div>
      </section>
    </aside>
  </div>
</div>
