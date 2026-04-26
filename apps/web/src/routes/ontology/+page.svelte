<script lang="ts">
  import Glyph from '$components/ui/Glyph.svelte';
  import {
    listActionTypes,
    listFunctionPackages,
    listLinkTypes,
    listObjectTypes,
    listSharedPropertyTypes,
    searchOntology,
    type ObjectType,
    type SearchResult
  } from '$lib/api/ontology';

  type SearchKind = 'all' | 'object_type' | 'object_instance' | 'action_type' | 'link_type' | 'shared_property_type';

  let objectTypes = $state<ObjectType[]>([]);
  let actionCount = $state(0);
  let functionCount = $state(0);
  let linkCount = $state(0);
  let sharedPropertyCount = $state(0);
  let totalTypes = $state(0);
  let loading = $state(true);
  let loadError = $state('');

  let semanticQuery = $state('');
  let semanticKind = $state<SearchKind>('all');
  let semanticLoading = $state(false);
  let semanticError = $state('');
  let searchResults = $state<SearchResult[]>([]);

  const docSections = [
    { label: 'Overview', href: '#overview' },
    { label: 'Why create an Ontology?', href: '#why-create' },
    { label: 'Models in the Ontology', href: '#models' },
    { label: 'Core concepts', href: '#core-concepts' },
    { label: 'Ontology-aware applications', href: '#applications' }
  ];

  const ontologyGroups = [
    { label: 'Ontologies', href: '#why-create' },
    { label: 'Object and link types', href: '#object-and-link-types' },
    { label: 'Action types', href: '#action-types-and-functions' },
    { label: 'Functions', href: '#action-types-and-functions' },
    { label: 'Interfaces', href: '#interfaces' }
  ];

  const searchGroups = [
    { label: 'Semantic search', href: '#semantic-search' },
    { label: 'Graph exploration', href: '/ontology/graph' }
  ];

  const applicationSurfaces = [
    {
      name: 'Object Explorer',
      route: '/object-explorer',
      glyph: 'search' as const,
      tone: 'bg-[#e8f2ff] text-[#2458b8]',
      summary: 'Search objects, pivot across linked entities, compare saved lists, and reopen explorations from a dedicated walk-up product.'
    },
    {
      name: 'Object workbench',
      route: '/ontology',
      glyph: 'ontology' as const,
      tone: 'bg-[#e7f0fe] text-[#2458b8]',
      summary: 'Schema, object views, inline edits, rules, functions, and machinery runtime in a single lab.'
    },
    {
      name: 'Graph',
      route: '/ontology/graph',
      glyph: 'graph' as const,
      tone: 'bg-[#eef5e8] text-[#2f6d35]',
      summary: 'Traverse schema and object neighborhoods to inspect links, boundaries, and graph topology.'
    },
    {
      name: 'Quiver',
      route: '/quiver',
      glyph: 'artifact' as const,
      tone: 'bg-[#f3ebff] text-[#6f42c1]',
      summary: 'Run ontology-aware analysis with reusable visual functions, joins, and linked drill paths.'
    },
    {
      name: 'Workshop',
      route: '/apps',
      glyph: 'cube' as const,
      tone: 'bg-[#fff1df] text-[#a35a11]',
      summary: 'Publish operational apps that bind directly to ontology objects, actions, and curated views.'
    },
    {
      name: 'Map',
      route: '/geospatial',
      glyph: 'object' as const,
      tone: 'bg-[#e4f7f4] text-[#0f766e]',
      summary: 'Project ontology objects onto map layers, clusters, routes, and geospatial interactions.'
    },
    {
      name: 'Dynamic Scheduling',
      route: '/dynamic-scheduling',
      glyph: 'run' as const,
      tone: 'bg-[#fff1df] text-[#a35a11]',
      summary: 'Operate a dedicated scheduling board with drag-and-drop staging, validation pressure, and queue actions.'
    },
    {
      name: 'Foundry Rules',
      route: '/foundry-rules',
      glyph: 'code' as const,
      tone: 'bg-[#f4efe5] text-[#745224]',
      summary: 'Author low-code rule logic, deploy workflows, operate rule schedules, and generate self-managed transforms.'
    },
    {
      name: 'Machinery',
      route: '/machinery',
      glyph: 'graph' as const,
      tone: 'bg-[#eef5e8] text-[#356b3c]',
      summary: 'Model process graphs, mine operational behavior, and supervise live flow with path and duration analytics.'
    }
  ];

  const searchKindOptions: { value: SearchKind; label: string }[] = [
    { value: 'all', label: 'All' },
    { value: 'object_type', label: 'Object types' },
    { value: 'object_instance', label: 'Objects' },
    { value: 'action_type', label: 'Actions' },
    { value: 'link_type', label: 'Links' },
    { value: 'shared_property_type', label: 'Shared properties' }
  ];

  async function load() {
    loading = true;
    loadError = '';

    try {
      const [typeResponse, actionResponse, functionResponse, linkResponse, sharedPropertyResponse] = await Promise.all([
        listObjectTypes({ page: 1, per_page: 100 }),
        listActionTypes({ page: 1, per_page: 100 }).catch(() => ({ data: [], total: 0, page: 1, per_page: 100 })),
        listFunctionPackages({ page: 1, per_page: 100 }).catch(() => ({ data: [], total: 0, page: 1, per_page: 100 })),
        listLinkTypes({ page: 1, per_page: 100 }).catch(() => ({ data: [], total: 0 })),
        listSharedPropertyTypes({ page: 1, per_page: 100 }).catch(() => ({ data: [], total: 0, page: 1, per_page: 100 }))
      ]);

      objectTypes = typeResponse.data;
      totalTypes = typeResponse.total;
      actionCount = actionResponse.total;
      functionCount = functionResponse.total;
      linkCount = linkResponse.total;
      sharedPropertyCount = sharedPropertyResponse.total;
    } catch (cause) {
      loadError = cause instanceof Error ? cause.message : 'Failed to load ontology building surfaces';
    } finally {
      loading = false;
    }
  }

  async function runSemanticSearch() {
    const query = semanticQuery.trim();
    if (!query) {
      semanticError = '';
      searchResults = [];
      return;
    }

    semanticLoading = true;
    semanticError = '';

    try {
      const response = await searchOntology({
        query,
        kind: semanticKind === 'all' ? undefined : semanticKind,
        limit: 12,
        semantic: true
      });
      searchResults = response.data;
    } catch (cause) {
      semanticError = cause instanceof Error ? cause.message : 'Failed to run ontology search';
    } finally {
      semanticLoading = false;
    }
  }

  function objectTypeGlyph(typeItem: ObjectType) {
    const key = `${typeItem.name} ${typeItem.display_name}`.toLowerCase();
    if (key.includes('route') || key.includes('link')) return 'link';
    if (key.includes('customer') || key.includes('employee') || key.includes('person')) return 'object';
    if (key.includes('dataset') || key.includes('table')) return 'database';
    return 'cube';
  }

  function semanticKindLabel(kind: string) {
    return kind.replaceAll('_', ' ');
  }

  const featuredObjectTypes = $derived(objectTypes.slice(0, 6));

  const suiteStats = $derived([
    {
      label: 'Object types',
      value: totalTypes,
      detail: 'semantic entities modeled across the ontology'
    },
    {
      label: 'Link types',
      value: linkCount,
      detail: 'typed relationships used for graph traversal'
    },
    {
      label: 'Action types',
      value: actionCount,
      detail: 'governed mutations and decision workflows'
    },
    {
      label: 'Functions',
      value: functionCount,
      detail: 'runtime packages backing logic and simulation'
    }
  ]);

  const groupedResults = $derived.by(() => {
    const groups = new Map<string, SearchResult[]>();
    for (const result of searchResults) {
      const label = semanticKindLabel(result.kind);
      if (!groups.has(label)) groups.set(label, []);
      groups.get(label)!.push(result);
    }
    return Array.from(groups.entries());
  });

  $effect(() => {
    load();
  });
</script>

<svelte:head>
  <title>OpenFoundry - Ontology building</title>
</svelte:head>

<div class="space-y-6">
  <section class="overflow-hidden rounded-[30px] border border-[var(--border-default)] bg-[linear-gradient(135deg,#fbfcff_0%,#f2f7ff_42%,#eef8f3_100%)] shadow-[var(--shadow-panel)]">
    <div class="grid gap-8 px-6 py-7 lg:grid-cols-[minmax(0,1.3fr)_340px] lg:px-8">
      <div>
        <div class="of-eyebrow">Ontology building</div>
        <h1 class="mt-3 max-w-4xl text-[34px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">
          Build the operational ontology that powers applications, search, graph analysis, and governed actioning.
        </h1>
        <p class="mt-4 max-w-3xl text-[15px] leading-7 text-[var(--text-muted)]">
          The ontology in OpenFoundry sits on top of data assets and turns them into objects, links,
          actions, functions, and interfaces that can be reused across workbenches, applications, and
          decision flows.
        </p>

        <div class="mt-6 flex flex-wrap gap-3">
          <a href="/ontology/types" class="of-btn of-btn-primary">
            <Glyph name="plus" size={16} />
            <span>Define object type</span>
          </a>
          <a href="/ontology/graph" class="of-btn">
            <Glyph name="graph" size={16} />
            <span>Open graph</span>
          </a>
          <a href="#semantic-search" class="of-btn">
            <Glyph name="search" size={16} />
            <span>Run semantic search</span>
          </a>
        </div>

        <div class="mt-7 grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          {#each suiteStats as stat}
            <article class="rounded-[18px] border border-white/75 bg-white/80 px-4 py-4 backdrop-blur">
              <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">{stat.label}</div>
              <div class="mt-2 text-[30px] font-semibold tracking-[-0.03em] text-[var(--text-strong)]">{stat.value}</div>
              <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">{stat.detail}</div>
            </article>
          {/each}
        </div>
      </div>

      <div class="rounded-[26px] border border-white/70 bg-white/75 p-5 shadow-[var(--shadow-panel)] backdrop-blur">
        <div class="flex items-center justify-between">
          <div>
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Capability map</div>
            <div class="mt-1 text-lg font-semibold text-[var(--text-strong)]">Ontology building overview</div>
          </div>
          <span class="rounded-full border border-[#d7e3f8] bg-[#edf3ff] px-3 py-1 text-xs font-semibold text-[#335ea8]">
            Live runtime
          </span>
        </div>

        <div class="mt-5 grid gap-3">
          <div class="rounded-[18px] border border-[#d9e5f6] bg-[#f8fbff] p-4">
            <div class="flex items-start gap-3">
              <span class="of-icon-box h-10 w-10 bg-[#e4eefc] text-[#2b5bb7]">
                <Glyph name="ontology" size={18} />
              </span>
              <div>
                <div class="text-sm font-semibold text-[var(--text-strong)]">Objects and links</div>
                <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">
                  Model entities, shape reusable schemas, and traverse live graph topology.
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-[18px] border border-[#e9dfc9] bg-[#fffaf1] p-4">
            <div class="flex items-start gap-3">
              <span class="of-icon-box h-10 w-10 bg-[#f6e9d4] text-[#9a6c2f]">
                <Glyph name="run" size={18} />
              </span>
              <div>
                <div class="text-sm font-semibold text-[var(--text-strong)]">Actions and functions</div>
                <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">
                  Govern edits, simulate what-if branches, and bind business logic to runtime packages.
                </div>
              </div>
            </div>
          </div>

          <div class="rounded-[18px] border border-[#d9efe9] bg-[#f4fbf8] p-4">
            <div class="flex items-start gap-3">
              <span class="of-icon-box h-10 w-10 bg-[#dff5ee] text-[#0f766e]">
                <Glyph name="artifact" size={18} />
              </span>
              <div>
                <div class="text-sm font-semibold text-[var(--text-strong)]">Interfaces and applications</div>
                <div class="mt-1 text-sm leading-6 text-[var(--text-muted)]">
                  Reuse object shapes across apps, graph lenses, maps, and Workshop runtime surfaces.
                </div>
              </div>
            </div>
          </div>
        </div>

        <div class="mt-5 rounded-[18px] border border-dashed border-[#cdd9ec] px-4 py-4">
          <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Powering decision-making</div>
          <p class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
            Reuse the same ontology across object workbenches, graph exploration, Quiver analysis,
            Workshop applications, and geospatial operations.
          </p>
        </div>
      </div>
    </div>
  </section>

  {#if loadError}
    <div class="of-inline-note">{loadError}</div>
  {/if}

  <div class="grid gap-6 xl:grid-cols-[260px_minmax(0,1fr)_240px]">
    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Ontology building</div>
        </div>
        <div class="space-y-5 px-4 py-4">
          <div>
            <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Root</div>
            <div class="mt-2 space-y-1">
              {#each docSections as item}
                <a href={item.href} class="flex items-center justify-between rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                  <span>{item.label}</span>
                  <Glyph name="chevron-right" size={14} />
                </a>
              {/each}
            </div>
          </div>

          <div>
            <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Define ontologies</div>
            <div class="mt-2 space-y-1">
              {#each ontologyGroups as item}
                <a href={item.href} class="flex items-center justify-between rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                  <span>{item.label}</span>
                  <Glyph name="chevron-right" size={14} />
                </a>
              {/each}
            </div>
          </div>

          <div>
            <div class="text-xs font-semibold uppercase tracking-[0.16em] text-[var(--text-soft)]">Ontology search</div>
            <div class="mt-2 space-y-1">
              {#each searchGroups as item}
                <a href={item.href} class="flex items-center justify-between rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
                  <span>{item.label}</span>
                  <Glyph name="chevron-right" size={14} />
                </a>
              {/each}
            </div>
          </div>
        </div>
      </section>
    </aside>

    <main class="space-y-6">
      <section id="overview" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="text-sm text-[var(--text-muted)]">Ontology building / Overview</div>
        </div>
        <div class="space-y-8 px-5 py-5">
          <div class="grid gap-6 lg:grid-cols-[minmax(0,1.2fr)_280px]">
            <div>
              <h2 class="text-[40px] font-semibold tracking-[-0.04em] text-[var(--text-strong)]">Ontology building</h2>
              <p class="mt-4 text-[15px] leading-8 text-[var(--text-muted)]">
                OpenFoundry uses the ontology as the operational layer of the platform. It connects
                datasets, models, applications, and runtime automation to real-world entities through
                semantic elements such as objects, properties, links, actions, functions, and governed
                access.
              </p>
            </div>

            <div class="rounded-[24px] border border-[#dce5f3] bg-[linear-gradient(180deg,#ffffff_0%,#f5f8fe_100%)] p-5">
              <div class="relative mx-auto h-[210px] max-w-[220px] overflow-hidden rounded-[20px] border border-[#dfe7f4] bg-white">
                <div class="absolute left-1/2 top-[18px] h-[92px] w-[92px] -translate-x-1/2 rotate-45 border border-[#d7e0ee] bg-[#fbfcff]"></div>
                <div class="absolute left-1/2 top-[68px] h-[78px] w-[78px] -translate-x-1/2 rotate-45 border border-[#cfd9e8] bg-[#f7faff]"></div>
                <div class="absolute left-[62px] top-[98px] h-[12px] w-[96px] rounded-full bg-[#dfe7f4]"></div>
                <div class="absolute left-[100px] top-[67px] h-[54px] w-[8px] rounded-full bg-[#5d6f88]"></div>
                <div class="absolute left-[96px] top-[59px] h-[16px] w-[40px] rotate-[30deg] rounded-full bg-[#6f829b]"></div>
                <div class="absolute left-[124px] top-[74px] h-[12px] w-[26px] rotate-[-38deg] rounded-full bg-[#91a0b5]"></div>
                <div class="absolute right-[22px] top-[54px] w-[74px] rounded-[14px] border border-[#dbe4f2] bg-[#ffffff] px-3 py-2 shadow-sm">
                  <div class="h-2 rounded-full bg-[#dfe7f4]"></div>
                  <div class="mt-2 h-2 w-10 rounded-full bg-[#e9eef8]"></div>
                  <div class="mt-2 h-7 rounded-[8px] border border-[#e4ebf7] bg-[#f9fbff]"></div>
                </div>
              </div>
            </div>
          </div>

          <section id="why-create" class="space-y-4">
            <h3 class="of-heading-lg">Why create an Ontology?</h3>
            <p class="text-[15px] leading-7 text-[var(--text-muted)]">
              The ontology gives you one shared operational model. Instead of rebuilding meaning in each
              app, analysis, or automation, teams define reusable semantic assets once and project them
              across graph, search, apps, maps, and governed runtime workflows.
            </p>
            <div class="grid gap-4 md:grid-cols-3">
              <article class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Unify data and operations</div>
                <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                  Bind raw data to objects and relationships that operators, analysts, and applications can all understand.
                </div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Govern change</div>
                <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                  Put actions, notifications, simulations, and approval logic behind controlled operational edits.
                </div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                <div class="text-sm font-semibold text-[var(--text-strong)]">Reuse everywhere</div>
                <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                  Expose the same ontology inside Quiver, Workshop, graph exploration, object sets, and map surfaces.
                </div>
              </article>
            </div>
          </section>

          <section id="models" class="space-y-4">
            <h3 class="of-heading-lg">Models in the Ontology</h3>
            <p class="text-[15px] leading-7 text-[var(--text-muted)]">
              The platform combines semantic structure with runtime behavior. Object and link types describe
              what exists, while action types, functions, rules, search, and views describe what users can do
              with those entities in production.
            </p>
            <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-4">
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="flex items-center gap-3">
                  <span class="of-icon-box h-10 w-10 bg-[#e4eefc] text-[#2b5bb7]">
                    <Glyph name="cube" size={18} />
                  </span>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Object types</div>
                </div>
                <div class="mt-3 text-sm leading-6 text-[var(--text-muted)]">{totalTypes} modeled semantic entities.</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="flex items-center gap-3">
                  <span class="of-icon-box h-10 w-10 bg-[#eff5e8] text-[#356b3c]">
                    <Glyph name="link" size={18} />
                  </span>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Link types</div>
                </div>
                <div class="mt-3 text-sm leading-6 text-[var(--text-muted)]">{linkCount} relationship contracts for graph traversal.</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="flex items-center gap-3">
                  <span class="of-icon-box h-10 w-10 bg-[#fff1df] text-[#a35a11]">
                    <Glyph name="run" size={18} />
                  </span>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Action types</div>
                </div>
                <div class="mt-3 text-sm leading-6 text-[var(--text-muted)]">{actionCount} governed mutations and workflows.</div>
              </article>
              <article class="rounded-[18px] border border-[var(--border-default)] bg-white p-4">
                <div class="flex items-center gap-3">
                  <span class="of-icon-box h-10 w-10 bg-[#f3ebff] text-[#6f42c1]">
                    <Glyph name="code" size={18} />
                  </span>
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Functions</div>
                </div>
                <div class="mt-3 text-sm leading-6 text-[var(--text-muted)]">{functionCount} runtime packages for logic, simulation, and automation.</div>
              </article>
            </div>
          </section>

          <section id="core-concepts" class="space-y-6">
            <h3 class="of-heading-lg">Core concepts</h3>

            <div id="object-and-link-types" class="space-y-3">
              <h4 class="of-heading-md">Object and link types</h4>
              <p class="text-[15px] leading-7 text-[var(--text-muted)]">
                Defining the semantics of your organization starts with object types, properties, shared properties,
                and links. OpenFoundry lets teams model a reusable foundation for applications, graph traversal,
                and operational state.
              </p>
              <div class="flex flex-wrap gap-3">
                <a href="/ontology/types" class="of-btn">
                  <Glyph name="cube" size={16} />
                  <span>Manage object types</span>
                </a>
                <a href="/ontology/graph" class="of-btn">
                  <Glyph name="link" size={16} />
                  <span>Explore graph structure</span>
                </a>
              </div>
            </div>

            <div id="action-types-and-functions" class="space-y-3">
              <h4 class="of-heading-md">Action types and functions</h4>
              <p class="text-[15px] leading-7 text-[var(--text-muted)]">
                Operational kinetics come from governed actions and executable functions. OpenFoundry already exposes
                validation, execution, batch runs, notifications, what-if simulation, runtime metrics, and authoring
                surfaces from the ontology workbench.
              </p>
              <div class="grid gap-4 md:grid-cols-2">
                <article class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Action runtime</div>
                  <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                    Inline edits, webhooks, function-backed execution, audit emission, notification side effects, and what-if branches.
                  </div>
                </article>
                <article class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                  <div class="text-sm font-semibold text-[var(--text-strong)]">Function platform</div>
                  <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                    TypeScript and Python packages, authoring templates, package metrics, and simulation-backed execution.
                  </div>
                </article>
              </div>
            </div>

            <div id="interfaces" class="space-y-3">
              <h4 class="of-heading-md">Interfaces</h4>
              <p class="text-[15px] leading-7 text-[var(--text-muted)]">
                Interfaces provide reusable shapes across object types. The ontology service already supports interface
                CRUD, interface properties, and bindings that let multiple object types share a common semantic contract.
              </p>
              <div class="rounded-[18px] border border-[var(--border-default)] bg-[#f8fbff] px-4 py-4 text-sm leading-6 text-[var(--text-muted)]">
                Shared properties available today: <span class="font-semibold text-[var(--text-strong)]">{sharedPropertyCount}</span>.
                Interface authoring is currently exposed through backend/runtime capabilities and the object workbench.
              </div>
            </div>
          </section>

          <section id="applications" class="space-y-4">
            <h3 class="of-heading-lg">Ontology-aware applications</h3>
            <p class="text-[15px] leading-7 text-[var(--text-muted)]">
              The ontology becomes valuable when it is reused in user-facing products. OpenFoundry already projects
              ontology primitives into object workbenches, graph tools, Quiver, Workshop apps, and geospatial surfaces.
            </p>

            <div class="grid gap-4 lg:grid-cols-2">
              {#each applicationSurfaces as surface}
                <a href={surface.route} class="rounded-[20px] border border-[var(--border-default)] bg-white p-5 transition hover:-translate-y-[1px] hover:border-[#b8cae8] hover:shadow-sm">
                  <div class="flex items-start gap-4">
                    <span class={`inline-flex h-12 w-12 items-center justify-center rounded-[16px] ${surface.tone}`}>
                      <Glyph name={surface.glyph} size={20} />
                    </span>
                    <div class="min-w-0">
                      <div class="text-[17px] font-semibold text-[var(--text-strong)]">{surface.name}</div>
                      <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{surface.summary}</div>
                    </div>
                  </div>
                </a>
              {/each}
            </div>
          </section>
        </div>
      </section>

      <section id="semantic-search" class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Ontology search</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">
                Search object types, objects, action types, links, and shared properties across the ontology runtime.
              </div>
            </div>
            <a href="/ontology/graph" class="of-link text-sm">Open graph explorer</a>
          </div>
        </div>

        <div class="space-y-4 px-5 py-5">
          <form
            class="space-y-4"
            onsubmit={(event) => {
              event.preventDefault();
              runSemanticSearch();
            }}
          >
            <div class="of-search-shell">
              <label class="of-search-filter">
                <select bind:value={semanticKind} class="w-full border-0 bg-transparent pr-5 text-sm font-medium text-[var(--text-default)]">
                  {#each searchKindOptions as option}
                    <option value={option.value}>{option.label}</option>
                  {/each}
                </select>
              </label>
              <div class="of-search-input-wrap">
                <Glyph name="search" size={18} />
                <input
                  bind:value={semanticQuery}
                  class="of-search-input"
                  placeholder="Search ontology entities, actions, and object runtime..."
                />
              </div>
              <button type="submit" class="px-4 text-sm font-medium text-[var(--text-link)]">
                {semanticLoading ? 'Searching...' : 'Search'}
              </button>
            </div>
          </form>

          {#if semanticError}
            <div class="of-inline-note">{semanticError}</div>
          {/if}

          {#if semanticQuery.trim() && searchResults.length === 0 && !semanticLoading && !semanticError}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-8 text-center text-sm text-[var(--text-muted)]">
              No ontology entities matched that query yet.
            </div>
          {:else if searchResults.length > 0}
            <div class="space-y-4">
              {#each groupedResults as [group, items]}
                <section class="rounded-[18px] border border-[var(--border-default)] bg-[#fbfcfe] p-4">
                  <div class="flex items-center justify-between">
                    <div class="text-sm font-semibold text-[var(--text-strong)]">{group}</div>
                    <span class="of-badge">{items.length}</span>
                  </div>
                  <div class="mt-4 space-y-3">
                    {#each items as result (result.kind + ':' + result.id)}
                      <article class="rounded-[14px] border border-[var(--border-subtle)] bg-white px-4 py-3">
                        <div class="flex items-start justify-between gap-4">
                          <div class="min-w-0">
                            <div class="flex flex-wrap items-center gap-2">
                              <span class="of-chip">{semanticKindLabel(result.kind)}</span>
                              <span class="text-xs text-[var(--text-soft)]">score {result.score.toFixed(2)}</span>
                            </div>
                            <div class="mt-2 text-sm font-semibold text-[var(--text-strong)]">{result.title}</div>
                            {#if result.subtitle}
                              <div class="mt-1 text-sm text-[var(--text-muted)]">{result.subtitle}</div>
                            {/if}
                            <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">{result.snippet}</div>
                          </div>
                          <a href={result.route} class="of-btn shrink-0 text-[13px]">Open</a>
                        </div>
                      </article>
                    {/each}
                  </div>
                </section>
              {/each}
            </div>
          {/if}
        </div>
      </section>

      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#fbfcfe] px-5 py-4">
          <div class="flex flex-wrap items-center justify-between gap-3">
            <div>
              <div class="of-heading-sm">Featured object types</div>
              <div class="mt-1 text-sm text-[var(--text-muted)]">
                Current semantic entities available for modeling, search, graph exploration, and runtime actioning.
              </div>
            </div>
            <a href="/ontology/types" class="of-link text-sm">Create or manage types</a>
          </div>
        </div>

        <div class="px-5 py-5">
          {#if loading}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">
              Loading ontology surfaces...
            </div>
          {:else if featuredObjectTypes.length === 0}
            <div class="rounded-[16px] border border-dashed border-[var(--border-default)] px-4 py-10 text-center text-sm text-[var(--text-muted)]">
              No object types have been defined yet.
            </div>
          {:else}
            <div class="grid gap-4 md:grid-cols-2 xl:grid-cols-3">
              {#each featuredObjectTypes as typeItem (typeItem.id)}
                <a href="/ontology/{typeItem.id}" class="rounded-[18px] border border-[var(--border-default)] bg-white p-4 transition hover:-translate-y-[1px] hover:border-[#b8cae8] hover:shadow-sm">
                  <div class="flex items-start gap-3">
                    <span
                      class="flex h-11 w-11 shrink-0 items-center justify-center rounded-[14px] text-white"
                      style={`background:${typeItem.color || '#4d8cf0'}`}
                    >
                      <Glyph name={objectTypeGlyph(typeItem)} size={18} />
                    </span>
                    <div class="min-w-0">
                      <div class="text-sm font-semibold text-[var(--text-strong)]">{typeItem.display_name}</div>
                      <div class="mt-1 font-mono text-xs text-[var(--text-soft)]">{typeItem.name}</div>
                      <div class="mt-2 text-sm leading-6 text-[var(--text-muted)]">
                        {typeItem.description || 'Semantic entity available for applications, graph, and governed runtime workflows.'}
                      </div>
                    </div>
                  </div>
                </a>
              {/each}
            </div>
          {/if}
        </div>
      </section>
    </main>

    <aside class="space-y-4 xl:sticky xl:top-5 xl:self-start">
      <section class="of-panel overflow-hidden">
        <div class="border-b border-[var(--border-subtle)] bg-[#f7fafc] px-4 py-3">
          <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-[var(--text-soft)]">Contents</div>
        </div>
        <div class="space-y-1 px-4 py-4">
          <a href="#overview" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Ontology building</span>
          </a>
          <a href="#object-and-link-types" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Object and link types</span>
          </a>
          <a href="#action-types-and-functions" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Action types and functions</span>
          </a>
          <a href="#interfaces" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Interfaces</span>
          </a>
          <a href="#applications" class="flex items-center gap-2 rounded-[10px] px-3 py-2 text-sm text-[var(--text-default)] hover:bg-[var(--bg-hover)]">
            <Glyph name="chevron-right" size={14} />
            <span>Powering decision-making</span>
          </a>
        </div>
      </section>

      <section class="rounded-[22px] border border-[#d6e7d8] bg-[#f4fbf6] p-4">
        <div class="text-sm font-semibold text-[#244f2a]">Operational reuse</div>
        <p class="mt-2 text-sm leading-6 text-[#4f6d53]">
          Once modeled, ontology assets can be reused across graph, search, object sets, Workshop apps,
          Quiver lenses, and map surfaces without redefining semantics per tool.
        </p>
      </section>
    </aside>
  </div>
</div>
