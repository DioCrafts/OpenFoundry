<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    getMachineryInsights,
    getMachineryQueue,
    listObjectTypes,
    listRules,
    type MachineryInsight,
    type MachineryQueueResponse,
    type ObjectType,
    type OntologyRule
  } from '$lib/api/ontology';
  import {
    listWorkflowApprovals,
    listWorkflowRuns,
    listWorkflows,
    type WorkflowApproval,
    type WorkflowDefinition,
    type WorkflowRun,
    type WorkflowStep
  } from '$lib/api/workflows';

  type MetricView = 'historical_count' | 'current_count' | 'historical_duration' | 'current_duration';
  type AnalysisMode = 'path_explorer' | 'duration_distribution';

  type GraphNode = {
    id: string;
    label: string;
    depth: number;
    row: number;
    kind: 'step' | 'terminal' | 'observed';
    step_type?: string;
    currentCount: number;
    historicalCount: number;
    currentDurationMinutes: number;
    historicalDurationMinutes: number;
    observed: boolean;
    deviating?: boolean;
    statusTone?: string;
  };

  type GraphEdge = {
    id: string;
    from: string;
    to: string;
    weight: number;
    label: string;
    kind: 'configured' | 'mined';
    observed: boolean;
  };

  type PathSummary = {
    id: string;
    label: string;
    weight: number;
    nodes: string[];
  };

  type HistogramBucket = {
    label: string;
    value: number;
  };

  type EventRow = {
    id: string;
    timestamp: string;
    kind: string;
    title: string;
    detail: string;
    tone: string;
  };

  let objectTypes = $state<ObjectType[]>([]);
  let rules = $state<OntologyRule[]>([]);
  let insights = $state<MachineryInsight[]>([]);
  let queue = $state<MachineryQueueResponse | null>(null);
  let workflows = $state<WorkflowDefinition[]>([]);
  let approvals = $state<WorkflowApproval[]>([]);
  let workflowRuns = $state<WorkflowRun[]>([]);

  let selectedObjectTypeId = $state('');
  let selectedWorkflowId = $state('');
  let selectedMetricView = $state<MetricView>('historical_count');
  let selectedAnalysisMode = $state<AnalysisMode>('path_explorer');
  let selectedGraphNodeId = $state('');
  let transitionCoveragePct = $state(75);
  let includeProcessObjects = $state(true);
  let includeLogObjects = $state(true);
  let mineStates = $state(true);
  let mineTransitions = $state(true);
  let excludedStates = $state<string[]>([]);
  let excludedTransitions = $state<string[]>([]);
  let loading = $state(true);
  let error = $state('');

  function formatTimestamp(value: string | null | undefined) {
    if (!value) return 'n/a';
    return new Intl.DateTimeFormat('en', {
      month: 'short',
      day: 'numeric',
      hour: 'numeric',
      minute: '2-digit'
    }).format(new Date(value));
  }

  function formatCompactDuration(minutes: number) {
    if (minutes >= 60) {
      const hours = Math.floor(minutes / 60);
      const remainder = minutes % 60;
      return remainder === 0 ? `${hours}h` : `${hours}h ${remainder}m`;
    }
    return `${minutes}m`;
  }

  function titleize(value: string | null | undefined) {
    const normalized = (value || 'general').replaceAll('_', ' ');
    return normalized.charAt(0).toUpperCase() + normalized.slice(1);
  }

  function stepDurationMinutes(step: WorkflowStep) {
    switch (step.step_type) {
      case 'approval':
        return 180;
      case 'submit_action':
        return 75;
      case 'notification':
        return 20;
      case 'action':
        return 60;
      default:
        return 45;
    }
  }

  function stepTone(stepType: string | undefined) {
    switch (stepType) {
      case 'approval':
        return 'bg-[#fff5e8] border-[#eed9af] text-[#8a5a12]';
      case 'submit_action':
        return 'bg-[#eef5e8] border-[#d8e5ca] text-[#356b3c]';
      case 'notification':
        return 'bg-[#f4ebff] border-[#dccbf6] text-[#6f42c1]';
      case 'action':
      default:
        return 'bg-[#edf3ff] border-[#d9e3f4] text-[#335ea8]';
    }
  }

  function selectedWorkflow() {
    return workflows.find((workflow) => workflow.id === selectedWorkflowId) ?? null;
  }

  function selectedObjectType() {
    return objectTypes.find((typeItem) => typeItem.id === selectedObjectTypeId) ?? null;
  }

  function activeSteps() {
    return selectedWorkflow()?.steps ?? [];
  }

  function approvalBacklog() {
    return approvals.filter((approval) => approval.status === 'pending').length;
  }

  function queueDepth() {
    return queue?.recommendation.queue_depth ?? 0;
  }

  function overdueCount() {
    return queue?.recommendation.overdue_count ?? 0;
  }

  function totalEstimatedMinutes() {
    return queue?.recommendation.total_estimated_minutes ?? 0;
  }

  function configuredGraph() {
    const workflow = selectedWorkflow();
    if (!workflow || workflow.steps.length === 0) {
      return { nodes: [] as GraphNode[], edges: [] as GraphEdge[] };
    }

    const steps = workflow.steps;
    const stepById = new Map(steps.map((step) => [step.id, step]));
    const edges: GraphEdge[] = [];
    const outgoing = new Map<string, string[]>();

    for (const step of steps) {
      const targets: string[] = [];
      if (step.next_step_id && stepById.has(step.next_step_id)) {
        targets.push(step.next_step_id);
      }
      for (const branch of step.branches ?? []) {
        if (branch.next_step_id && stepById.has(branch.next_step_id)) {
          targets.push(branch.next_step_id);
        }
      }
      if (targets.length === 0) {
        targets.push('terminal:completed');
      }
      outgoing.set(step.id, targets);
      for (const target of targets) {
        edges.push({
          id: `${step.id}->${target}`,
          from: step.id,
          to: target,
          weight: 0,
          label: target.startsWith('terminal:') ? 'complete' : 'advance',
          kind: 'configured',
          observed: false
        });
      }
    }

    const nodes: GraphNode[] = [];
    const discovered = new Set<string>();
    const visitQueue: Array<{ id: string; depth: number; row: number }> = [{ id: steps[0].id, depth: 0, row: 0 }];
    const rowsAtDepth = new Map<number, number>();

    while (visitQueue.length > 0) {
      const current = visitQueue.shift()!;
      if (discovered.has(current.id)) continue;
      discovered.add(current.id);

      const step = stepById.get(current.id);
      if (!step) continue;

      const row = rowsAtDepth.get(current.depth) ?? current.row;
      rowsAtDepth.set(current.depth, row + 1);
      nodes.push({
        id: step.id,
        label: step.name,
        depth: current.depth,
        row,
        kind: 'step',
        step_type: step.step_type,
        currentCount: 0,
        historicalCount: 0,
        currentDurationMinutes: 0,
        historicalDurationMinutes: 0,
        observed: false
      });

      const children = outgoing.get(step.id) ?? [];
      children.forEach((childId, index) => {
        if (childId.startsWith('terminal:')) return;
        visitQueue.push({
          id: childId,
          depth: current.depth + 1,
          row: row + index
        });
      });
    }

    const terminalNodes = [
      {
        id: 'terminal:completed',
        label: 'Completed',
        depth: Math.max(1, ...nodes.map((node) => node.depth)) + 1,
        row: 0,
        kind: 'terminal' as const,
        currentCount: 0,
        historicalCount: 0,
        currentDurationMinutes: 0,
        historicalDurationMinutes: 0,
        observed: false,
        statusTone: 'bg-[#eef5e8] border-[#d8e5ca] text-[#356b3c]'
      },
      {
        id: 'terminal:cancelled',
        label: 'Cancelled',
        depth: Math.max(1, ...nodes.map((node) => node.depth)) + 1,
        row: 1,
        kind: 'terminal' as const,
        currentCount: 0,
        historicalCount: 0,
        currentDurationMinutes: 0,
        historicalDurationMinutes: 0,
        observed: false,
        statusTone: 'bg-[#fff3f1] border-[#f0d4d0] text-[#b42318]'
      }
    ];

    return { nodes: [...nodes, ...terminalNodes], edges };
  }

  function configuredMetrics(nodes: GraphNode[]) {
    const stepNodes = nodes.filter((node) => node.kind === 'step');
    const durationByNode = new Map(
      activeSteps().map((step) => [step.id, stepDurationMinutes(step)])
    );
    const stepNodeIds = stepNodes.map((node) => node.id);
    const indexMap = new Map(stepNodeIds.map((id, index) => [id, index]));

    const currentByNode = new Map<string, number>();
    const historicalByNode = new Map<string, number>();

    for (const run of workflowRuns) {
      const currentId = run.current_step_id ?? (run.status === 'failed' || run.status === 'cancelled' ? 'terminal:cancelled' : 'terminal:completed');
      currentByNode.set(currentId, (currentByNode.get(currentId) ?? 0) + 1);

      const currentIndex = run.current_step_id ? (indexMap.get(run.current_step_id) ?? stepNodeIds.length - 1) : stepNodeIds.length - 1;
      for (let index = 0; index <= currentIndex && index < stepNodeIds.length; index += 1) {
        const stepId = stepNodeIds[index];
        historicalByNode.set(stepId, (historicalByNode.get(stepId) ?? 0) + 1);
      }

      if (!run.current_step_id && run.finished_at) {
        const terminalId = run.status === 'failed' || run.status === 'cancelled' ? 'terminal:cancelled' : 'terminal:completed';
        historicalByNode.set(terminalId, (historicalByNode.get(terminalId) ?? 0) + 1);
      }
    }

    return nodes.map((node) => {
      const currentCount = currentByNode.get(node.id) ?? 0;
      const historicalCount = historicalByNode.get(node.id) ?? 0;
      const baseDuration = durationByNode.get(node.id) ?? 30;
      return {
        ...node,
        currentCount,
        historicalCount,
        currentDurationMinutes: currentCount * baseDuration,
        historicalDurationMinutes: historicalCount * baseDuration,
        observed: currentCount > 0 || historicalCount > 0
      };
    });
  }

  function applyObservedWeights(edges: GraphEdge[], nodes: GraphNode[]) {
    const historicalByNode = new Map(nodes.map((node) => [node.id, node.historicalCount]));
    return edges.map((edge) => ({
      ...edge,
      weight: Math.max(1, historicalByNode.get(edge.to) ?? 0),
      observed: (historicalByNode.get(edge.to) ?? 0) > 0
    }));
  }

  function activeConfiguredGraph() {
    const graph = configuredGraph();
    const nodes = configuredMetrics(graph.nodes);
    const edges = applyObservedWeights(graph.edges, nodes);
    return { nodes, edges };
  }

  function observedStatusNodes() {
    const pendingApprovals = approvalBacklog();
    const pendingQueue = (queue?.data ?? []).filter((item) => item.status === 'pending').length;
    const inProgressQueue = (queue?.data ?? []).filter((item) => item.status === 'in_progress').length;
    const completedQueue = (queue?.data ?? []).filter((item) => item.status === 'completed').length;
    const cancelledQueue = (queue?.data ?? []).filter((item) => item.status === 'cancelled').length;

    return [
      {
        id: 'observed:submitted',
        label: 'Submitted',
        depth: 0,
        row: 0,
        kind: 'observed' as const,
        currentCount: workflowRuns.filter((run) => run.status !== 'completed' && run.status !== 'failed').length,
        historicalCount: workflowRuns.length,
        currentDurationMinutes: 0,
        historicalDurationMinutes: workflowRuns.length * 45,
        observed: true
      },
      {
        id: 'observed:approval_pending',
        label: 'Approval pending',
        depth: 1,
        row: 0,
        kind: 'observed' as const,
        currentCount: pendingApprovals,
        historicalCount: approvals.length,
        currentDurationMinutes: pendingApprovals * 180,
        historicalDurationMinutes: approvals.length * 180,
        observed: approvals.length > 0
      },
      {
        id: 'observed:queue_pending',
        label: 'Queue pending',
        depth: 2,
        row: 0,
        kind: 'observed' as const,
        currentCount: pendingQueue,
        historicalCount: pendingQueue + inProgressQueue + completedQueue + cancelledQueue,
        currentDurationMinutes: pendingQueue * 60,
        historicalDurationMinutes: (pendingQueue + inProgressQueue + completedQueue + cancelledQueue) * 60,
        observed: pendingQueue + inProgressQueue + completedQueue + cancelledQueue > 0
      },
      {
        id: 'observed:queue_in_progress',
        label: 'In progress',
        depth: 3,
        row: 0,
        kind: 'observed' as const,
        currentCount: inProgressQueue,
        historicalCount: inProgressQueue + completedQueue,
        currentDurationMinutes: inProgressQueue * 90,
        historicalDurationMinutes: (inProgressQueue + completedQueue) * 90,
        observed: inProgressQueue + completedQueue > 0
      },
      {
        id: 'observed:completed',
        label: 'Completed',
        depth: 4,
        row: 0,
        kind: 'observed' as const,
        currentCount: completedQueue,
        historicalCount: workflowRuns.filter((run) => run.finished_at).length + completedQueue,
        currentDurationMinutes: completedQueue * 20,
        historicalDurationMinutes: (workflowRuns.filter((run) => run.finished_at).length + completedQueue) * 20,
        observed: workflowRuns.filter((run) => run.finished_at).length + completedQueue > 0
      },
      {
        id: 'observed:cancelled',
        label: 'Cancelled',
        depth: 4,
        row: 1,
        kind: 'observed' as const,
        currentCount: cancelledQueue,
        historicalCount: cancelledQueue + workflowRuns.filter((run) => run.status === 'failed' || run.status === 'cancelled').length,
        currentDurationMinutes: cancelledQueue * 15,
        historicalDurationMinutes: (cancelledQueue + workflowRuns.filter((run) => run.status === 'failed' || run.status === 'cancelled').length) * 15,
        observed: cancelledQueue > 0
      }
    ];
  }

  function observedStatusEdges() {
    const nodes = observedStatusNodes();
    const count = (id: string) => nodes.find((node) => node.id === id)?.historicalCount ?? 0;
    const edges: GraphEdge[] = [
      {
        id: 'observed:submitted->approval_pending',
        from: 'observed:submitted',
        to: 'observed:approval_pending',
        weight: Math.max(1, count('observed:approval_pending')),
        label: 'review',
        kind: 'mined',
        observed: true
      },
      {
        id: 'observed:approval_pending->queue_pending',
        from: 'observed:approval_pending',
        to: 'observed:queue_pending',
        weight: Math.max(1, count('observed:queue_pending')),
        label: 'approve',
        kind: 'mined',
        observed: true
      },
      {
        id: 'observed:queue_pending->queue_in_progress',
        from: 'observed:queue_pending',
        to: 'observed:queue_in_progress',
        weight: Math.max(1, count('observed:queue_in_progress')),
        label: 'start',
        kind: 'mined',
        observed: true
      },
      {
        id: 'observed:queue_in_progress->completed',
        from: 'observed:queue_in_progress',
        to: 'observed:completed',
        weight: Math.max(1, count('observed:completed')),
        label: 'finish',
        kind: 'mined',
        observed: true
      },
      {
        id: 'observed:queue_pending->cancelled',
        from: 'observed:queue_pending',
        to: 'observed:cancelled',
        weight: Math.max(1, count('observed:cancelled')),
        label: 'drop',
        kind: 'mined',
        observed: true
      }
    ];
    return { nodes, edges };
  }

  function filteredObservedGraph() {
    let { nodes, edges } = observedStatusEdges();

    if (!includeProcessObjects) {
      nodes = nodes.filter((node) => !['observed:submitted', 'observed:approval_pending'].includes(node.id));
      edges = edges.filter((edge) => !['observed:submitted->approval_pending', 'observed:approval_pending->queue_pending'].includes(edge.id));
    }

    if (!includeLogObjects) {
      nodes = nodes.filter((node) => !node.id.startsWith('observed:queue_'));
      edges = edges.filter((edge) => !edge.from.startsWith('observed:queue_') && !edge.to.startsWith('observed:queue_'));
    }

    if (!mineStates) {
      nodes = [];
      edges = [];
    }

    if (!mineTransitions) {
      edges = [];
    }

    nodes = nodes.filter((node) => !excludedStates.includes(node.id));
    edges = edges.filter((edge) => !excludedTransitions.includes(edge.id));

    const sorted = [...edges].sort((left, right) => right.weight - left.weight);
    const total = sorted.reduce((sum, edge) => sum + edge.weight, 0);
    let running = 0;
    const allowed = new Set<string>();
    for (const edge of sorted) {
      running += edge.weight;
      allowed.add(edge.id);
      if (total === 0 || (running / total) * 100 >= transitionCoveragePct) break;
    }
    const filteredEdges = edges.filter((edge) => allowed.has(edge.id));
    const usedNodeIds = new Set(filteredEdges.flatMap((edge) => [edge.from, edge.to]));
    const filteredNodes = nodes.filter((node) => usedNodeIds.has(node.id));

    return { nodes: filteredNodes, edges: filteredEdges };
  }

  function nodeMetricValue(node: GraphNode) {
    switch (selectedMetricView) {
      case 'current_count':
        return String(node.currentCount);
      case 'historical_duration':
        return formatCompactDuration(node.historicalDurationMinutes);
      case 'current_duration':
        return formatCompactDuration(node.currentDurationMinutes);
      case 'historical_count':
      default:
        return String(node.historicalCount);
    }
  }

  function metricLabel() {
    switch (selectedMetricView) {
      case 'current_count':
        return 'Current count';
      case 'historical_duration':
        return 'Historical duration';
      case 'current_duration':
        return 'Current duration';
      case 'historical_count':
      default:
        return 'Historical count';
    }
  }

  function graphLayout(nodes: GraphNode[]) {
    const maxDepth = Math.max(0, ...nodes.map((node) => node.depth));
    const maxRow = Math.max(0, ...nodes.map((node) => node.row));
    return {
      width: Math.max(900, (maxDepth + 1) * 220 + 140),
      height: Math.max(320, (maxRow + 1) * 140 + 120)
    };
  }

  function nodePosition(node: GraphNode) {
    return {
      left: 60 + node.depth * 220,
      top: 40 + node.row * 140
    };
  }

  function edgePath(edge: GraphEdge, nodes: GraphNode[]) {
    const from = nodes.find((node) => node.id === edge.from);
    const to = nodes.find((node) => node.id === edge.to);
    if (!from || !to) return '';

    const fromPos = nodePosition(from);
    const toPos = nodePosition(to);
    const startX = fromPos.left + 160;
    const startY = fromPos.top + 38;
    const endX = toPos.left;
    const endY = toPos.top + 38;
    const midX = (startX + endX) / 2;
    return `M ${startX} ${startY} C ${midX} ${startY}, ${midX} ${endY}, ${endX} ${endY}`;
  }

  function nodeDetail(nodeId: string) {
    return [...activeConfiguredGraph().nodes, ...filteredObservedGraph().nodes].find((node) => node.id === nodeId) ?? null;
  }

  function pathSummaries() {
    const { nodes, edges } = activeConfiguredGraph();
    const root = nodes.find((node) => node.kind === 'step' && node.depth === 0);
    if (!root) return [] as PathSummary[];

    const adjacency = new Map<string, GraphEdge[]>();
    for (const edge of edges) {
      const current = adjacency.get(edge.from) ?? [];
      current.push(edge);
      adjacency.set(edge.from, current);
    }

    const paths: PathSummary[] = [];
    const visit = (nodeId: string, trail: string[]) => {
      const outgoing = adjacency.get(nodeId) ?? [];
      if (outgoing.length === 0) {
        paths.push({
          id: trail.join('>'),
          label: trail.map((id) => nodes.find((node) => node.id === id)?.label ?? id).join(' -> '),
          weight: Math.max(1, ...trail.map((id) => nodes.find((node) => node.id === id)?.historicalCount ?? 0)),
          nodes: trail
        });
        return;
      }
      for (const edge of outgoing) {
        if (edge.to.startsWith('terminal:')) {
          paths.push({
            id: [...trail, edge.to].join('>'),
            label: [...trail, edge.to]
              .map((id) => nodes.find((node) => node.id === id)?.label ?? titleize(id.split(':')[1]))
              .join(' -> '),
            weight: Math.max(1, edge.weight),
            nodes: [...trail, edge.to]
          });
        } else if (!trail.includes(edge.to)) {
          visit(edge.to, [...trail, edge.to]);
        }
      }
    };

    visit(root.id, [root.id]);
    return paths.sort((left, right) => right.weight - left.weight).slice(0, 6);
  }

  function durationHistogram() {
    const buckets: HistogramBucket[] = [
      { label: '< 30m', value: 0 },
      { label: '30-60m', value: 0 },
      { label: '1-2h', value: 0 },
      { label: '2-4h', value: 0 },
      { label: '4h+', value: 0 }
    ];

    for (const item of queue?.data ?? []) {
      const duration = Math.max(item.estimated_duration_minutes, 0);
      if (duration < 30) buckets[0].value += 1;
      else if (duration < 60) buckets[1].value += 1;
      else if (duration < 120) buckets[2].value += 1;
      else if (duration < 240) buckets[3].value += 1;
      else buckets[4].value += 1;
    }

    return buckets;
  }

  function capabilityLoad() {
    return queue?.recommendation.capability_load ?? [];
  }

  function interventionRows() {
    const queueRows = (queue?.data ?? [])
      .filter((item) => item.status === 'pending' || item.status === 'in_progress')
      .map((item) => ({
        id: item.id,
        title: item.rule_display_name,
        detail: `${titleize(item.required_capability)} · ${formatCompactDuration(item.estimated_duration_minutes)}`,
        status: item.status,
        timestamp: item.scheduled_for
      }));

    const approvalRows = approvals.map((approval) => ({
      id: approval.id,
      title: approval.title,
      detail: approval.instructions,
      status: approval.status,
      timestamp: approval.requested_at
    }));

    return [...queueRows, ...approvalRows]
      .sort((left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime())
      .slice(0, 8);
  }

  function eventTimeline() {
    const events: EventRow[] = [];

    workflowRuns.forEach((run) => {
      events.push({
        id: `run:${run.id}`,
        timestamp: run.started_at,
        kind: 'workflow_run',
        title: `Workflow ${run.status}`,
        detail: `${run.trigger_type} trigger ${run.current_step_id ? `at ${run.current_step_id}` : 'completed'}`,
        tone: 'bg-[#edf3ff] text-[#335ea8]'
      });
    });

    approvals.forEach((approval) => {
      events.push({
        id: `approval:${approval.id}`,
        timestamp: approval.requested_at,
        kind: 'approval',
        title: approval.title,
        detail: approval.instructions,
        tone: 'bg-[#fff4e5] text-[#9a6c2f]'
      });
    });

    (queue?.data ?? []).forEach((item) => {
      events.push({
        id: `queue:${item.id}`,
        timestamp: item.updated_at,
        kind: 'queue',
        title: item.rule_display_name,
        detail: `${item.status} · ${titleize(item.required_capability)}`,
        tone: item.status === 'completed' ? 'bg-[#eef5e8] text-[#356b3c]' : 'bg-[#f4ebff] text-[#6f42c1]'
      });
    });

    return events
      .sort((left, right) => new Date(right.timestamp).getTime() - new Date(left.timestamp).getTime())
      .slice(0, 18);
  }

  function toggleExcludedState(nodeId: string) {
    excludedStates = excludedStates.includes(nodeId)
      ? excludedStates.filter((item) => item !== nodeId)
      : [...excludedStates, nodeId];
  }

  function toggleExcludedTransition(edgeId: string) {
    excludedTransitions = excludedTransitions.includes(edgeId)
      ? excludedTransitions.filter((item) => item !== edgeId)
      : [...excludedTransitions, edgeId];
  }

  async function loadWorkflowsAndRuns() {
    const workflowResponse = await listWorkflows({ per_page: 50 });
    workflows = workflowResponse.data;

    if (!selectedWorkflowId && workflows.length > 0) {
      selectedWorkflowId = workflows[0].id;
    } else if (selectedWorkflowId && !workflows.find((workflow) => workflow.id === selectedWorkflowId)) {
      selectedWorkflowId = workflows[0]?.id ?? '';
    }

    const [runResponse, approvalResponse] = await Promise.all([
      selectedWorkflowId ? listWorkflowRuns(selectedWorkflowId, { per_page: 50 }) : Promise.resolve({ data: [], page: 1, per_page: 50, total: 0 }),
      listWorkflowApprovals({ per_page: 50, workflow_id: selectedWorkflowId || undefined })
    ]);

    workflowRuns = runResponse.data;
    approvals = approvalResponse.data;
  }

  async function loadOntologyProcessData() {
    const [typesResponse] = await Promise.all([
      listObjectTypes({ page: 1, per_page: 100 })
    ]);

    objectTypes = typesResponse.data;
    if (!selectedObjectTypeId && objectTypes.length > 0) {
      selectedObjectTypeId = objectTypes[0].id;
    }

    const [ruleResponse, insightResponse, queueResponse] = await Promise.all([
      listRules(selectedObjectTypeId ? { object_type_id: selectedObjectTypeId, per_page: 100 } : { per_page: 100 }),
      getMachineryInsights(selectedObjectTypeId ? { object_type_id: selectedObjectTypeId } : undefined),
      getMachineryQueue(selectedObjectTypeId ? { object_type_id: selectedObjectTypeId } : undefined)
    ]);

    rules = ruleResponse.data;
    insights = insightResponse.data;
    queue = queueResponse;
  }

  async function load() {
    loading = true;
    error = '';
    try {
      await Promise.all([loadWorkflowsAndRuns(), loadOntologyProcessData()]);
    } catch (cause) {
      error = cause instanceof Error ? cause.message : 'Failed to load Machinery surfaces';
    } finally {
      loading = false;
    }
  }

  async function refreshWorkflow(nextId: string) {
    selectedWorkflowId = nextId;
    await loadWorkflowsAndRuns();
  }

  async function refreshObjectType(nextId: string) {
    selectedObjectTypeId = nextId;
    await loadOntologyProcessData();
  }

  const configured = $derived(activeConfiguredGraph());
  const observed = $derived(filteredObservedGraph());
  const detailNode = $derived(nodeDetail(selectedGraphNodeId));
  const topPaths = $derived(pathSummaries());
  const durationBuckets = $derived(durationHistogram());
  const timelineEvents = $derived(eventTimeline());
  const interventions = $derived(interventionRows());

  $effect(() => {
    void load();
  });
</script>

<svelte:head>
  <title>OpenFoundry - Machinery</title>
</svelte:head>

<div class="space-y-6">
  <section class="overflow-hidden rounded-[28px] border border-[var(--border-default)] bg-[linear-gradient(135deg,#fbfcff_0%,#eef5ff_35%,#f4fbf7_68%,#fff8ef_100%)] shadow-[var(--shadow-panel)]">
    <div class="grid gap-8 px-6 py-7 lg:grid-cols-[minmax(0,1.35fr)_360px] lg:px-8">
      <div>
        <div class="of-eyebrow">Machinery</div>
        <h1 class="mt-3 max-w-4xl text-[34px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">
          Model a process, mine its behavior, and supervise operational flow with real-time human intervention.
        </h1>
        <p class="mt-4 max-w-3xl text-[15px] leading-7 text-[var(--text-muted)]">
          Machinery in OpenFoundry now has its own product surface for process analysis. It connects workflow
          definitions, workflow runs, approvals, and machinery queue telemetry into one graph-first operating layer.
        </p>

        <div class="mt-6 flex flex-wrap gap-3">
          <a href="#draw-graph" class="of-btn of-btn-primary">
            <Glyph name="graph" size={16} />
            <span>Draw process graph</span>
          </a>
          <a href="#process-mining" class="of-btn">
            <Glyph name="search" size={16} />
            <span>Mine process behavior</span>
          </a>
          <a href="#monitoring" class="of-btn">
            <Glyph name="run" size={16} />
            <span>Analyze and monitor</span>
          </a>
        </div>

        <div class="mt-7 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Processes</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{workflows.length}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Workflow-backed process definitions available for graph analysis.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Active flow</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{workflowRuns.filter((run) => !run.finished_at).length}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Runs still traversing the configured process graph.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Interventions</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{approvalBacklog() + queueDepth()}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">{approvalBacklog()} approvals and {queueDepth()} queue items waiting on operators.</div>
          </article>
          <article class="rounded-[18px] border border-white/75 bg-white/85 px-4 py-4 backdrop-blur">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Pressure</div>
            <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{overdueCount()}</div>
            <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">Overdue scheduled actions across {formatCompactDuration(totalEstimatedMinutes())} of estimated work.</div>
          </article>
        </div>
      </div>

      <div class="rounded-[24px] border border-white/75 bg-white/80 p-5 shadow-[var(--shadow-panel)] backdrop-blur">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Process mining coverage</div>
            <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">Connected sources</div>
          </div>
          <span class="rounded-full border border-[#d8e3f4] bg-[#f4f8ff] px-3 py-1 text-xs font-semibold text-[#335ea8]">Dedicated app</span>
        </div>
        <div class="mt-5 grid gap-3">
          <article class="rounded-[18px] border border-[#dbe7f6] bg-[#f8fbff] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Process objects</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{queueDepth()} queue items and {rules.length} rule outputs driving state changes over time.</div>
          </article>
          <article class="rounded-[18px] border border-[#d8eadf] bg-[#f4fbf7] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Log objects</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{workflowRuns.length} workflow runs and {approvals.length} approvals available for operational mining.</div>
          </article>
          <article class="rounded-[18px] border border-[#e8dec8] bg-[#fffaf1] p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Outputs</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">Process graph, bottleneck views, path explorer, duration distribution, and intervention watchlists.</div>
          </article>
        </div>
      </div>
    </div>
  </section>

  {#if error}
    <div class="of-inline-note">{error}</div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[280px_minmax(0,1fr)_320px]">
    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Machinery</div>
        </div>
        <div class="space-y-4 px-4 py-4">
          <div>
            <label for="workflow-selector" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Process definition</label>
            <select
              id="workflow-selector"
              bind:value={selectedWorkflowId}
              class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
              onchange={(event) => void refreshWorkflow((event.currentTarget as HTMLSelectElement).value)}
            >
              {#each workflows as workflow (workflow.id)}
                <option value={workflow.id}>{workflow.name}</option>
              {/each}
            </select>
          </div>

          <div>
            <label for="object-selector" class="mb-2 block text-sm font-medium text-[var(--text-strong)]">Process object type</label>
            <select
              id="object-selector"
              bind:value={selectedObjectTypeId}
              class="w-full rounded-xl border border-[var(--border-default)] bg-white px-3 py-2 text-sm"
              onchange={(event) => void refreshObjectType((event.currentTarget as HTMLSelectElement).value)}
            >
              {#each objectTypes as typeItem (typeItem.id)}
                <option value={typeItem.id}>{typeItem.display_name}</option>
              {/each}
            </select>
          </div>

          <div class="rounded-[16px] border border-[#e7edf5] bg-[#fbfcfe] px-4 py-4">
            <div class="text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Sections</div>
            <div class="mt-3 space-y-1">
              <a href="#draw-graph" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                <Glyph name="chevron-right" size={14} />
                <span>Draw a graph</span>
              </a>
              <a href="#connect-data" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                <Glyph name="chevron-right" size={14} />
                <span>Connect data</span>
              </a>
              <a href="#process-mining" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                <Glyph name="chevron-right" size={14} />
                <span>Process mining</span>
              </a>
              <a href="#monitoring" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                <Glyph name="chevron-right" size={14} />
                <span>Analyze and monitor</span>
              </a>
            </div>
          </div>
        </div>
      </section>
    </aside>

    <main class="space-y-6">
      <section id="draw-graph" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Draw a graph</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Workflow steps define the configured process graph. Machinery turns them into a state-and-transition canvas with reusable metric views.</div>
            </div>
            <a href="/workflows" class="of-btn">
              <Glyph name="graph" size={15} />
              <span>Open workflow builder</span>
            </a>
          </div>
        </div>

        <div class="space-y-5 px-5 py-5">
          <div class="flex flex-wrap gap-2">
            <button type="button" class={`of-btn ${selectedMetricView === 'historical_count' ? 'of-btn-primary' : ''}`} onclick={() => selectedMetricView = 'historical_count'}>Historical count</button>
            <button type="button" class={`of-btn ${selectedMetricView === 'current_count' ? 'of-btn-primary' : ''}`} onclick={() => selectedMetricView = 'current_count'}>Current count</button>
            <button type="button" class={`of-btn ${selectedMetricView === 'historical_duration' ? 'of-btn-primary' : ''}`} onclick={() => selectedMetricView = 'historical_duration'}>Historical duration</button>
            <button type="button" class={`of-btn ${selectedMetricView === 'current_duration' ? 'of-btn-primary' : ''}`} onclick={() => selectedMetricView = 'current_duration'}>Current duration</button>
          </div>

          {#if configured.nodes.length === 0}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">No process definition is available yet for Machinery.</div>
          {:else}
            <div class="overflow-x-auto rounded-[20px] border border-[var(--border-default)] bg-[#fcfdff] p-4">
              <div class="relative" style={`width:${graphLayout(configured.nodes).width}px;height:${graphLayout(configured.nodes).height}px;`}>
                <svg class="absolute inset-0 h-full w-full">
                  {#each configured.edges as edge (edge.id)}
                    <path
                      d={edgePath(edge, configured.nodes)}
                      fill="none"
                      stroke={edge.observed ? '#9bb4dd' : '#d3dce8'}
                      stroke-width={Math.max(2, Math.min(8, edge.weight / 2))}
                      stroke-dasharray={edge.observed ? undefined : '6 6'}
                      opacity={edge.observed ? 0.95 : 0.6}
                    ></path>
                  {/each}
                </svg>

                {#each configured.nodes as node (node.id)}
                  <button
                    type="button"
                    onclick={() => selectedGraphNodeId = node.id}
                    class={`absolute w-[160px] rounded-[18px] border px-4 py-3 text-left shadow-sm transition ${node.kind === 'step' ? stepTone(node.step_type) : node.statusTone || 'bg-white border-[var(--border-default)] text-[var(--text-strong)]'} ${selectedGraphNodeId === node.id ? 'ring-2 ring-[#7aa0de]' : ''}`}
                    style={`left:${nodePosition(node).left}px;top:${nodePosition(node).top}px;`}
                  >
                    <div class="text-sm font-semibold">{node.label}</div>
                    <div class="mt-1 text-[11px] uppercase tracking-[0.08em] opacity-70">{node.kind === 'step' ? titleize(node.step_type) : node.kind}</div>
                    <div class="mt-3 text-xs opacity-70">{metricLabel()}</div>
                    <div class="mt-1 text-lg font-semibold">{nodeMetricValue(node)}</div>
                  </button>
                {/each}
              </div>
            </div>
          {/if}
        </div>
      </section>

      <section id="connect-data" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Connect data to Machinery</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">Rules, queue telemetry, runs, and approvals become process objects and log objects for graph analysis and mining.</div>
        </div>
        <div class="grid gap-4 px-5 py-5 lg:grid-cols-2 xl:grid-cols-4">
          <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Process objects</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{selectedObjectType()?.display_name || 'n/a'} contributes {queue?.data.length ?? 0} scheduled object instances.</div>
          </article>
          <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Log objects</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{workflowRuns.length} workflow runs and {approvals.length} approvals capture observed process behavior.</div>
          </article>
          <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Transitions</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{configured.edges.length} configured edges and {observed.edges.length} mined transitions currently visible.</div>
          </article>
          <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
            <div class="text-sm font-semibold text-[var(--text-strong)]">Rule outputs</div>
            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{insights.length} rule insights and {rules.length} active rules contribute operational state changes.</div>
          </article>
        </div>
      </section>

      <section id="process-mining" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="of-heading-sm">Process mining</div>
          <div class="mt-1 text-sm text-[var(--text-muted)]">Mine observed states and transitions from workflow and queue telemetry, then filter noisy behavior down to the process flow you want to supervise.</div>
        </div>

        <div class="grid gap-6 px-5 py-5 xl:grid-cols-[minmax(0,1fr)_320px]">
          <div class="space-y-4">
            {#if observed.nodes.length === 0}
              <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">No mined process graph is visible with the current source and filter settings.</div>
            {:else}
              <div class="overflow-x-auto rounded-[20px] border border-[var(--border-default)] bg-[#fcfdff] p-4">
                <div class="relative" style={`width:${graphLayout(observed.nodes).width}px;height:${graphLayout(observed.nodes).height}px;`}>
                  <svg class="absolute inset-0 h-full w-full">
                    {#each observed.edges as edge (edge.id)}
                      <path
                        d={edgePath(edge, observed.nodes)}
                        fill="none"
                        stroke="#c48f45"
                        stroke-width={Math.max(2, Math.min(10, edge.weight / 2))}
                        opacity="0.88"
                      ></path>
                    {/each}
                  </svg>

                  {#each observed.nodes as node (node.id)}
                    <button
                      type="button"
                      onclick={() => toggleExcludedState(node.id)}
                      class={`absolute w-[160px] rounded-[18px] border px-4 py-3 text-left shadow-sm transition ${excludedStates.includes(node.id) ? 'border-[#f0d4d0] bg-[#fff3f1] text-[#b42318]' : 'border-[#e8d5b2] bg-[#fff8ef] text-[#8a5a12]'}`}
                      style={`left:${nodePosition(node).left}px;top:${nodePosition(node).top}px;`}
                    >
                      <div class="text-sm font-semibold">{node.label}</div>
                      <div class="mt-1 text-[11px] uppercase tracking-[0.08em] opacity-70">Observed state</div>
                      <div class="mt-3 text-xs opacity-70">{metricLabel()}</div>
                      <div class="mt-1 text-lg font-semibold">{nodeMetricValue(node)}</div>
                    </button>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="grid gap-4 lg:grid-cols-2">
              <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Visible transitions</div>
                <div class="mt-3 space-y-3">
                  {#each observed.edges as edge (edge.id)}
                    <button
                      type="button"
                      onclick={() => toggleExcludedTransition(edge.id)}
                      class={`w-full rounded-[14px] border px-4 py-3 text-left transition ${excludedTransitions.includes(edge.id) ? 'border-[#f0d4d0] bg-[#fff3f1]' : 'border-[var(--border-subtle)] hover:border-[#d9c7a5] hover:bg-[#fffaf2]'}`}
                    >
                      <div class="flex items-center justify-between gap-3">
                        <div class="text-sm font-semibold text-[var(--text-strong)]">{edge.label}</div>
                        <div class="text-xs text-[var(--text-soft)]">weight {edge.weight}</div>
                      </div>
                      <div class="mt-1 text-sm text-[var(--text-muted)]">{edge.from.split(':').pop()} -> {edge.to.split(':').pop()}</div>
                    </button>
                  {/each}
                </div>
              </div>

              <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Current filters</div>
                <div class="mt-4 space-y-4">
                  <div>
                    <div class="mb-2 text-xs font-semibold uppercase tracking-[0.12em] text-[var(--text-soft)]">Transition filter</div>
                    <input type="range" min="10" max="100" step="5" bind:value={transitionCoveragePct} class="w-full" />
                    <div class="mt-2 text-sm text-[var(--text-muted)]">Show the top {transitionCoveragePct}% cumulative transition weight.</div>
                  </div>
                  <div class="space-y-2">
                    <label class="flex items-center justify-between rounded-[14px] border border-[var(--border-subtle)] px-3 py-3">
                      <span class="text-sm text-[var(--text-strong)]">Process objects</span>
                      <input type="checkbox" bind:checked={includeProcessObjects} class="h-4 w-4" />
                    </label>
                    <label class="flex items-center justify-between rounded-[14px] border border-[var(--border-subtle)] px-3 py-3">
                      <span class="text-sm text-[var(--text-strong)]">Log objects</span>
                      <input type="checkbox" bind:checked={includeLogObjects} class="h-4 w-4" />
                    </label>
                    <label class="flex items-center justify-between rounded-[14px] border border-[var(--border-subtle)] px-3 py-3">
                      <span class="text-sm text-[var(--text-strong)]">States</span>
                      <input type="checkbox" bind:checked={mineStates} class="h-4 w-4" />
                    </label>
                    <label class="flex items-center justify-between rounded-[14px] border border-[var(--border-subtle)] px-3 py-3">
                      <span class="text-sm text-[var(--text-strong)]">Transitions</span>
                      <input type="checkbox" bind:checked={mineTransitions} class="h-4 w-4" />
                    </label>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Excluded states</div>
              <div class="mt-3 flex flex-wrap gap-2">
                {#each excludedStates as item (item)}
                  <button type="button" class="rounded-full border border-[#f0d4d0] bg-[#fff3f1] px-3 py-1.5 text-xs font-medium text-[#b42318]" onclick={() => toggleExcludedState(item)}>
                    {item.split(':').pop()}
                  </button>
                {/each}
                {#if excludedStates.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No excluded states.</div>
                {/if}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Excluded transitions</div>
              <div class="mt-3 flex flex-wrap gap-2">
                {#each excludedTransitions as item (item)}
                  <button type="button" class="rounded-full border border-[#f0d4d0] bg-[#fff3f1] px-3 py-1.5 text-xs font-medium text-[#b42318]" onclick={() => toggleExcludedTransition(item)}>
                    {item}
                  </button>
                {/each}
                {#if excludedTransitions.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No excluded transitions.</div>
                {/if}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Mining legend</div>
              <div class="mt-3 space-y-3 text-sm text-[var(--text-muted)]">
                <div>Amber nodes and edges represent mined operational behavior.</div>
                <div>Removing noisy states or transitions lets us converge on a cleaner conformance model.</div>
                <div>The coverage slider trims the long tail of infrequent transitions.</div>
              </div>
            </div>
          </div>
        </div>
      </section>

      <section id="monitoring" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Analyze and monitor a process</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">Cycle between path exploration and duration analysis, then supervise active interventions across the process.</div>
            </div>
            <div class="flex flex-wrap gap-2">
              <button type="button" class={`of-btn ${selectedAnalysisMode === 'path_explorer' ? 'of-btn-primary' : ''}`} onclick={() => selectedAnalysisMode = 'path_explorer'}>Path explorer</button>
              <button type="button" class={`of-btn ${selectedAnalysisMode === 'duration_distribution' ? 'of-btn-primary' : ''}`} onclick={() => selectedAnalysisMode = 'duration_distribution'}>Duration distribution</button>
            </div>
          </div>
        </div>

        <div class="grid gap-6 px-5 py-5 xl:grid-cols-[minmax(0,1fr)_340px]">
          <div class="space-y-4">
            {#if selectedAnalysisMode === 'path_explorer'}
              <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Most likely process paths</div>
                <div class="mt-3 space-y-3">
                  {#if topPaths.length === 0}
                    <div class="text-sm text-[var(--text-muted)]">No path summaries are available yet.</div>
                  {:else}
                    {#each topPaths as path (path.id)}
                      <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                        <div class="flex items-center justify-between gap-3">
                          <div class="text-sm font-semibold text-[var(--text-strong)]">{path.label}</div>
                          <div class="text-xs text-[var(--text-soft)]">weight {path.weight}</div>
                        </div>
                        <div class="mt-2 text-sm text-[var(--text-muted)]">{path.nodes.length} graph states in this path.</div>
                      </div>
                    {/each}
                  {/if}
                </div>
              </div>
            {:else}
              <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Duration distribution</div>
                <div class="mt-4 grid grid-cols-5 gap-3">
                  {#each durationBuckets as bucket (bucket.label)}
                    <div class="flex flex-col items-center gap-2">
                      <div class="flex h-32 w-full items-end rounded-[14px] bg-[#f6f8fb] px-2 pb-2">
                        <div
                          class="w-full rounded-[10px] bg-[linear-gradient(180deg,#78a6ee_0%,#3d6dd8_100%)]"
                          style={`height:${Math.max(10, Math.round((bucket.value / Math.max(1, ...durationBuckets.map((item) => item.value))) * 100))}%`}
                        ></div>
                      </div>
                      <div class="text-center text-[11px] text-[var(--text-soft)]">{bucket.label}</div>
                      <div class="text-sm font-semibold text-[var(--text-strong)]">{bucket.value}</div>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Intervention watchlist</div>
              <div class="mt-3 space-y-3">
                {#if interventions.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No live interventions to monitor.</div>
                {:else}
                  {#each interventions as item (item.id)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex items-start justify-between gap-3">
                        <div>
                          <div class="text-sm font-semibold text-[var(--text-strong)]">{item.title}</div>
                          <div class="mt-1 text-sm text-[var(--text-muted)]">{item.detail}</div>
                        </div>
                        <span class="rounded-full bg-[#fff4e5] px-2 py-1 text-[11px] font-semibold text-[#9a6c2f]">{item.status}</span>
                      </div>
                      <div class="mt-2 text-xs text-[var(--text-soft)]">{formatTimestamp(item.timestamp)}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>

            <div class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
              <div class="text-sm font-semibold text-[var(--text-strong)]">Event timeline</div>
              <div class="mt-3 space-y-3">
                {#if timelineEvents.length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No events recorded yet.</div>
                {:else}
                  {#each timelineEvents as event (event.id)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex items-start justify-between gap-3">
                        <div>
                          <div class="flex items-center gap-2">
                            <span class={`rounded-full px-2 py-1 text-[11px] font-semibold ${event.tone}`}>{event.kind}</span>
                            <span class="text-sm font-semibold text-[var(--text-strong)]">{event.title}</span>
                          </div>
                          <div class="mt-2 text-sm text-[var(--text-muted)]">{event.detail}</div>
                        </div>
                        <div class="text-xs text-[var(--text-soft)]">{formatTimestamp(event.timestamp)}</div>
                      </div>
                    </div>
                  {/each}
                {/if}
              </div>
            </div>
          </div>

          <div class="space-y-4">
            <section class="of-panel overflow-hidden">
              <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Node details</div>
              </div>
              <div class="space-y-4 px-4 py-4">
                {#if detailNode}
                  <div>
                    <div class="text-lg font-semibold text-[var(--text-strong)]">{detailNode.label}</div>
                    <div class="mt-1 text-sm text-[var(--text-muted)]">{detailNode.kind === 'step' ? titleize(detailNode.step_type) : titleize(detailNode.kind)}</div>
                  </div>
                  <div class="grid gap-3">
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-soft)]">Historical count</div>
                      <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">{detailNode.historicalCount}</div>
                    </div>
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-soft)]">Current count</div>
                      <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">{detailNode.currentCount}</div>
                    </div>
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-soft)]">Historical duration</div>
                      <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">{formatCompactDuration(detailNode.historicalDurationMinutes)}</div>
                    </div>
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="text-xs font-semibold uppercase tracking-[0.08em] text-[var(--text-soft)]">Current duration</div>
                      <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">{formatCompactDuration(detailNode.currentDurationMinutes)}</div>
                    </div>
                  </div>
                {:else}
                  <div class="text-sm text-[var(--text-muted)]">Select a graph node to inspect its process metrics.</div>
                {/if}
              </div>
            </section>

            <section class="of-panel overflow-hidden">
              <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
                <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Capability load</div>
              </div>
              <div class="space-y-3 px-4 py-4">
                {#if capabilityLoad().length === 0}
                  <div class="text-sm text-[var(--text-muted)]">No capability load available yet.</div>
                {:else}
                  {#each capabilityLoad() as item (item.capability)}
                    <div class="rounded-[14px] border border-[var(--border-subtle)] px-4 py-3">
                      <div class="flex items-center justify-between gap-3">
                        <div class="text-sm font-semibold text-[var(--text-strong)]">{titleize(item.capability)}</div>
                        <div class="text-xs text-[var(--text-soft)]">{item.pending_count} items</div>
                      </div>
                      <div class="mt-2 h-2 rounded-full bg-[#eef2f7]">
                        <div
                          class="h-2 rounded-full bg-[linear-gradient(90deg,#71c19c_0%,#2d8a67_100%)]"
                          style={`width:${Math.min(100, Math.max(8, item.pending_count * 16))}%`}
                        ></div>
                      </div>
                      <div class="mt-2 text-sm text-[var(--text-muted)]">{formatCompactDuration(item.total_estimated_minutes)}</div>
                    </div>
                  {/each}
                {/if}
              </div>
            </section>
          </div>
        </div>
      </section>
    </main>

    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="rounded-[22px] border border-[#d6e7d8] bg-[#f4fbf6] p-4">
        <div class="text-sm font-semibold text-[#244f2a]">Operational reuse</div>
        <p class="mt-2 text-sm leading-6 text-[#4f6d53]">
          Machinery now sits between workflow authoring, Foundry Rules, and Dynamic Scheduling:
          define the process in one place, mine its behavior from telemetry, and intervene directly in the live flow.
        </p>
      </section>
    </aside>
  </div>
</div>
