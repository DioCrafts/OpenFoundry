# Matriz completa: Palantir Foundry "Ontology building" vs OpenFoundry

Fecha de analisis: `2026-04-26`

## Alcance

- Esta matriz compara `docs_original_palantir_foundry/foundry-docs/Ontology building` contra el codigo fuente actual de OpenFoundry.
- No usa la documentacion tecnica interna del repo como fuente de verdad funcional.
- Tambien incorpora lo observado en las capturas `*.screenshot.png` del mismo arbol documental para evaluar la capa de producto y UX, no solo el backend.
- Este archivo amplia `ontology-building-checklist.md` hacia cobertura completa de todo el arbol documental.

## Metodo

- Inventario completo revisado: `365` paginas Markdown.
- Agrupacion usada: `24` bloques documentales.
- Criterio:
  - `SI`: existe una capacidad o superficie claramente implementada en codigo.
  - `PARCIAL`: el nucleo existe, pero el producto, la UX o el alcance difieren de forma material.
  - `NO`: no se encontro un equivalente claro en codigo.
  - `N/A`: documento conceptual o arquitectura interna de Palantir no comparable 1:1 desde este repo.
- El apendice hereda el estado del bloque documental al que pertenece cada pagina.

## Resumen ejecutivo

- OpenFoundry **si** coincide con Palantir en un nucleo ontologico real: object types, properties, links, interfaces, shared properties, actions, functions, object sets, search, graph y project/resource scoping.
- OpenFoundry coincide **parcialmente** en la mayor parte de la suite mostrada en los docs e imagenes: hay runtime, endpoints y varias UIs, pero la separacion en productos maduros y especializados es claramente menor.
- Las mayores coincidencias visibles estan en `Interfaces`, `Map/geospatial`, `Foundry Rules`, `Machinery` y el hub raiz de `Ontology building`.
- Las mayores brechas estan ahora en `Object Monitors`, `Configured/Full/Panel Object Views`, `Ontology lifecycle` avanzado y varias partes del permissioning y semantic search descritas por Palantir.

## Cobertura numerica

| Estado | Paginas |
|---|---:|
| `SI` | 115 |
| `PARCIAL` | 232 |
| `NO` | 12 |
| `N/A` | 6 |
| **Total** | **365** |

## Evidencia transversal principal

- Ontologia y API principal: `services/ontology-service/src/main.rs`
- Tipos, propiedades, links, interfaces y shared properties:
  - `services/ontology-service/src/handlers/types.rs`
  - `services/ontology-service/src/handlers/properties.rs`
  - `services/ontology-service/src/handlers/links.rs`
  - `services/ontology-service/src/handlers/interfaces.rs`
  - `services/ontology-service/src/handlers/shared_properties.rs`
- Acciones y object views:
  - `services/ontology-service/src/handlers/actions.rs`
  - `services/ontology-service/src/handlers/objects.rs`
  - `services/ontology-service/src/models/action_type.rs`
  - `services/ontology-service/src/models/object_view.rs`
- Funciones:
  - `services/ontology-service/src/handlers/functions.rs`
  - `services/ontology-service/src/models/function_package.rs`
  - `services/ontology-service/src/models/function_authoring.rs`
  - `services/ontology-service/src/domain/function_runtime.rs`
- Search, graph y semantic:
  - `services/ontology-service/src/handlers/search.rs`
  - `services/ontology-service/src/domain/search/mod.rs`
  - `services/ontology-service/src/domain/search/semantic.rs`
  - `services/ontology-service/src/domain/indexer.rs`
- Permissioning y scoping:
  - `services/ontology-service/src/domain/access.rs`
  - `services/ontology-service/src/domain/project_access.rs`
  - `services/ontology-service/src/handlers/projects.rs`
  - `services/auth-service/src/handlers/restricted_views.rs`
- Geospatial:
  - `apps/web/src/routes/geospatial/+page.svelte`
  - `services/geospatial-service/src/main.rs`
  - `services/geospatial-service/src/handlers/layers.rs`
  - `services/geospatial-service/src/handlers/features.rs`
  - `services/geospatial-service/src/handlers/geocode.rs`
- Frontend ontology surfaces:
  - `apps/web/src/routes/ontology/+page.svelte`
  - `apps/web/src/routes/ontology/[id]/+page.svelte`
  - `apps/web/src/routes/ontology/object-sets/+page.svelte`
  - `apps/web/src/routes/ontology/graph/+page.svelte`
  - `apps/web/src/routes/foundry-rules/+page.svelte`
  - `apps/web/src/routes/machinery/+page.svelte`
  - `apps/web/src/routes/quiver/+page.svelte`
  - `apps/web/src/routes/apps/+page.svelte`

## Matriz principal

| Bloque documental | Paginas | Estado | Diagnostico | Evidencia principal |
|---|---:|---|---|---|
| `(root)` | 5 | `SI` | La landing de `Ontology building` ya funciona como hub editorial y operativo: separa capacidades, expone superficies de producto, mantiene busqueda semantica en contexto y replica la estructura conceptual del overview de referencia. | `apps/web/src/routes/ontology/+page.svelte`, `services/ontology-service/src/main.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte`, `apps/web/src/routes/apps/+page.svelte`, `apps/web/src/routes/quiver/+page.svelte` |
| `Applications/Dynamic Scheduling` | 17 | `SI` | Ya existe una app dedicada de Dynamic Scheduling con board temporal por resource lanes, pucks de queue, staging drag-and-drop, panel de suggestion function, validaciones y acciones operativas sobre la machinery queue. | `apps/web/src/routes/dynamic-scheduling/+page.svelte`, `services/ontology-service/src/models/rule.rs`, `services/ontology-service/src/domain/rules.rs`, `services/ontology-service/src/handlers/rules.rs`, `apps/web/src/lib/api/ontology.ts` |
| `Applications/Foundry Rules` | 27 | `SI` | Ya existe una app dedicada de Foundry Rules con authoring de reglas, simulacion y apply, deploy workflow control, approval queue, vistas time-series y customization pipeline con self-managed transforms. | `apps/web/src/routes/foundry-rules/+page.svelte`, `services/ontology-service/src/handlers/rules.rs`, `services/ontology-service/src/models/rule.rs`, `apps/web/src/lib/api/ontology.ts`, `apps/web/src/lib/api/workflows.ts` |
| `Applications/Machinery` | 6 | `SI` | Ya existe una app dedicada de Machinery con process graph, sources de process/log objects, process mining con filtros y monitoring operativo sobre workflow runs, approvals e insights de machinery queue. | `apps/web/src/routes/machinery/+page.svelte`, `services/ontology-service/src/handlers/rules.rs`, `services/ontology-service/src/domain/rules.rs`, `services/ontology-service/src/models/rule.rs`, `apps/web/src/lib/api/workflows.ts`, `apps/web/src/lib/api/ontology.ts` |
| `Applications/Map` | 37 | `SI` | Hay una correspondencia fuerte: capas, spatial query, clustering, routing, geocoding, visualizacion y una superficie de mapa operativa. | `apps/web/src/routes/geospatial/+page.svelte`, `services/geospatial-service/src/main.rs`, `services/geospatial-service/src/handlers/layers.rs`, `services/geospatial-service/src/handlers/features.rs`, `services/geospatial-service/src/handlers/geocode.rs` |
| `Applications/Object Explorer` | 15 | `SI` | Ya existe una app dedicada de Object Explorer con global search, tabs por tipo de resultado, sidebar de filtros, preview de Object View, pivot sobre linked objects, chart-led exploration, comparison de saved lists y guardado de explorations walk-up. | `apps/web/src/routes/object-explorer/+page.svelte`, `apps/web/src/routes/ontology/object-sets/+page.svelte`, `apps/web/src/routes/ontology/graph/+page.svelte`, `apps/web/src/routes/queries/+page.svelte`, `apps/web/src/lib/api/ontology.ts` |
| `Applications/Object Monitors [Sunset]` | 11 | `NO` | No encontre una app de Object Monitors. Solo hay primitivas cercanas en reglas, notificaciones, auditoria y workflows. | `services/ontology-service/src/handlers/actions.rs`, `apps/web/src/routes/workflows/+page.svelte`, `apps/web/src/routes/audit/+page.svelte`, `services/audit-service/src/main.rs` |
| `Applications/Object Views` | 21 | `PARCIAL` | Hay object view runtime e inspector, pero no `Configured`, `Full` y `Panel Object Views` como producto configurable de primer nivel. | `services/ontology-service/src/handlers/objects.rs`, `services/ontology-service/src/models/object_view.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` |
| `Applications/Ontology Manager` | 9 | `PARCIAL` | Hay edicion real de ontologia, shared props, interfaces y scoping por proyectos, pero no un Ontology Manager separado con import/export/change management equivalente. | `services/ontology-service/src/handlers/types.rs`, `services/ontology-service/src/handlers/properties.rs`, `services/ontology-service/src/handlers/interfaces.rs`, `services/ontology-service/src/handlers/shared_properties.rs`, `services/ontology-service/src/handlers/projects.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` |
| `Applications/Vertex` | 27 | `PARCIAL` | Hay grafo, Quiver y superficies de graph exploration, pero no un producto Vertex equivalente con escenarios, media layers, events y graph templating con ese mismo alcance. | `apps/web/src/routes/quiver/+page.svelte`, `apps/web/src/routes/ontology/graph/+page.svelte`, `services/ontology-service/src/handlers/search.rs`, `services/ontology-service/src/models/quiver.rs` |
| `Define Ontologies/Action types` | 32 | `PARCIAL` | Action types estan bastante avanzados: validate, execute, batch, what-if, webhook, function-backed, inline edit, notifications y audit. Aun faltan varias piezas de lifecycle y UX first-class. | `services/ontology-service/src/handlers/actions.rs`, `services/ontology-service/src/models/action_type.rs`, `services/ontology-service/src/main.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` |
| `Define Ontologies/Functions` | 63 | `PARCIAL` | Hay function packages, authoring surface, validate, simulate, runs, metrics y runtime TS/Python inline. No vi cobertura equivalente para todo el publishing, testing y model workflow descrito por Palantir. | `services/ontology-service/src/handlers/functions.rs`, `services/ontology-service/src/models/function_package.rs`, `services/ontology-service/src/models/function_authoring.rs`, `services/ontology-service/src/domain/function_runtime.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` |
| `Define Ontologies/Interfaces` | 8 | `SI` | CRUD de interfaces, propiedades de interfaz y binding a object types estan claramente implementados. | `services/ontology-service/src/handlers/interfaces.rs`, `services/ontology-service/src/models/interface.rs`, `services/ontology-service/src/main.rs` |
| `Define Ontologies/Object and link types` | 46 | `PARCIAL` | Object types, properties, links, shared properties y edicion existen. No encontre subsistemas equivalentes completos para value types, structs, render hints, statuses, type classes o derived properties first-class. | `services/ontology-service/src/handlers/types.rs`, `services/ontology-service/src/handlers/properties.rs`, `services/ontology-service/src/handlers/links.rs`, `services/ontology-service/src/handlers/shared_properties.rs`, `services/ontology-service/src/models/property.rs`, `services/ontology-service/src/domain/type_system.rs` |
| `Define Ontologies/Ontologies` | 9 | `PARCIAL` | Hay proyectos, memberships y resource bindings que se parecen a espacios o scoping compartido, pero no un ontology lifecycle completo con proposals, branches y testing como producto. | `services/ontology-service/src/handlers/projects.rs`, `services/ontology-service/src/models/project.rs`, `services/ontology-service/migrations/20260426200500_ontology_projects.sql` |
| `Define Ontologies/Ontology design Best practices and anti-patterns.md` | 1 | `N/A` | Documento de guia conceptual. No se evalua como feature implementada. | `N/A` |
| `Ontology architecture/Indexing` | 5 | `PARCIAL` | Hay funnel batch, source health y documentos de busqueda indexados. La historia de indexing y streaming no tiene el mismo alcance que la documentacion de Palantir. | `services/ontology-service/src/handlers/funnel.rs`, `services/ontology-service/src/models/funnel.rs`, `services/ontology-service/src/domain/indexer.rs`, `services/ontology-service/migrations/20260426213000_ontology_funnel.sql` |
| `Ontology architecture/Object databases` | 1 | `N/A` | La pagina trata de almacenamiento interno de Palantir (`OSv1`/`Phonograph`). No hay correspondencia directa verificable desde este repo. | `N/A` |
| `Ontology architecture/Object edits and materializations` | 6 | `PARCIAL` | Hay write path, inline edits, actions gobernadas, simulaciones y materializacion de object sets, pero no un modelo equivalente de merge/conflict/edit-history/migrations de edits. | `services/ontology-service/src/handlers/objects.rs`, `services/ontology-service/src/handlers/actions.rs`, `services/ontology-service/src/models/object_view.rs`, `services/ontology-service/migrations/20260426050000_object_sets_runtime.sql` |
| `Ontology architecture/Object permissioning` | 7 | `PARCIAL` | Hay org scoping, markings/clearance, restricted views y acceso por proyecto, pero el modelo es bastante mas simple que el permissioning descrito por Palantir. | `services/ontology-service/src/domain/access.rs`, `services/ontology-service/src/domain/project_access.rs`, `services/ontology-service/src/handlers/projects.rs`, `services/auth-service/src/handlers/restricted_views.rs` |
| `Ontology architecture/Overview and getting started` | 4 | `N/A` | Son paginas de arquitectura y migracion (`OSv1`, `OSv2`) especificas de Palantir. No comparables 1:1 desde OpenFoundry. | `N/A` |
| `Ontology search/Derived properties.md` | 1 | `NO` | No encontre `derived properties` como primitive first-class del ontology/search layer. | `services/ontology-service/src/handlers/properties.rs`, `services/ontology-service/src/domain/indexer.rs`, `services/ontology-service/src/models/property.rs` |
| `Ontology search/Search syntax.md` | 1 | `PARCIAL` | Hay endpoint de search y UI de semantic search, pero no vi una DSL o search syntax equiparable como superficie documentada o expresiva. | `services/ontology-service/src/models/search.rs`, `services/ontology-service/src/handlers/search.rs`, `apps/web/src/routes/ontology/+page.svelte` |
| `Ontology search/Semantic search` | 6 | `PARCIAL` | Hybrid/semantic search existe, con embeddings provider-backed y KNN sobre objetos. No vi el mismo alcance en document processing multimodal, OAG y workflows de modelos. | `services/ontology-service/src/domain/search/mod.rs`, `services/ontology-service/src/domain/search/semantic.rs`, `services/ontology-service/src/handlers/objects.rs`, `services/ai-service/src/handlers/knowledge.rs` |

## Hallazgos transversales

### 1. El motor ontologico es real

OpenFoundry no tiene solo una UI superficial. El `ontology-service` ya expone un dominio amplio con tipos, propiedades, links, interfaces, shared properties, acciones, funciones, reglas, object sets, proyectos, search y graph.

### 2. La mayor brecha esta en el empaquetado de producto

Las capturas de Palantir muestran superficies maduras y separadas como `Object Explorer`, `Configured Object Views` o `Vertex`. En OpenFoundry, gran parte de esas capacidades aun viven concentradas en workbenches mas tecnicos, especialmente `apps/web/src/routes/ontology/[id]/+page.svelte`.

### 3. Las capas avanzadas de lifecycle y governance estan menos desarrolladas

La diferencia mas clara no esta en CRUD basico, sino en todo lo que rodea al ontology lifecycle: proposals, branching, testing de cambios, permissioning muy granular, derived properties first-class y productos de runtime con UX especializada.

## Apendice A: inventario completo por bloque

### (root) - `SI`

Estas paginas ya quedan cubiertas por una landing de `Ontology building` mas cercana a la referencia: overview editorial, navegacion por grupos, secciones conceptuales y accesos claros a las superficies operativas reales.

- `Core concepts.md`
- `Models in the Ontology.md`
- `Ontology-aware applications.md`
- `Overview.md`
- `Why create an Ontology.md`

### Applications/Dynamic Scheduling - `SI`

Estas paginas ya quedan cubiertas por una app dedicada de Dynamic Scheduling con board temporal por resource lanes, pucks de queue, staging drag-and-drop, suggestion surface, validaciones y acciones operativas sobre la machinery queue.

- `Applications/Dynamic Scheduling/Core concepts.md`
- `Applications/Dynamic Scheduling/Getting started.md`
- `Applications/Dynamic Scheduling/Inline metrics.md`
- `Applications/Dynamic Scheduling/Ontology primitives.md`
- `Applications/Dynamic Scheduling/Overview.md`
- `Applications/Dynamic Scheduling/Row-level interactions/Overview.md`
- `Applications/Dynamic Scheduling/Row-level interactions/Triggering actions and events.md`
- `Applications/Dynamic Scheduling/Schedule layer (puck) styling.md`
- `Applications/Dynamic Scheduling/Schedule layer-level interactions/Drag and drop.md`
- `Applications/Dynamic Scheduling/Schedule layer-level interactions/Drag to create.md`
- `Applications/Dynamic Scheduling/Schedule layer-level interactions/Overview.md`
- `Applications/Dynamic Scheduling/Scheduling Calendar widget/Overview.md`
- `Applications/Dynamic Scheduling/Scheduling Calendar widget/Widget configuration.md`
- `Applications/Dynamic Scheduling/Scheduling Gantt Chart widget.md`
- `Applications/Dynamic Scheduling/Search Functions.md`
- `Applications/Dynamic Scheduling/Suggestion Functions.md`
- `Applications/Dynamic Scheduling/Validation rules.md`

### Applications/Foundry Rules - `SI`

Estas paginas ya quedan cubiertas por una app dedicada de Foundry Rules con catalogo de reglas, authoring y edicion, simulate/apply, deploy workflow control, approval queue, time-series scheduling y customization pipeline con self-managed transforms.

- `Applications/Foundry Rules/Add Foundry Rules to a Marketplace product.md`
- `Applications/Foundry Rules/Core concepts/Foundry Rules workflow configuration.md`
- `Applications/Foundry Rules/Core concepts/Object model.md`
- `Applications/Foundry Rules/Core concepts/Overview.md`
- `Applications/Foundry Rules/Core concepts/Rule logic.md`
- `Applications/Foundry Rules/Core concepts/Workshop application.md`
- `Applications/Foundry Rules/Deploy/Author and run a rule.md`
- `Applications/Foundry Rules/Deploy/Configure workflow.md`
- `Applications/Foundry Rules/Deploy/Deploy workflow.md`
- `Applications/Foundry Rules/Deploy/Overview.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Configure Workshop application.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Configure rule Actions.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Configure time series for Foundry Rules.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Configure transforms pipeline.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Migrate to Foundry Rules.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Overview.md`
- `Applications/Foundry Rules/Legacy Foundry Rules setup (Taurus)/Upgrade to use rule Actions.md`
- `Applications/Foundry Rules/Overview.md`
- `Applications/Foundry Rules/Settings & customization/Add a custom property.md`
- `Applications/Foundry Rules/Settings & customization/Customize Foundry Rules pipeline.md`
- `Applications/Foundry Rules/Settings & customization/Enable optional features.md`
- `Applications/Foundry Rules/Settings & customization/Overview.md`
- `Applications/Foundry Rules/Settings & customization/Permissions for editing rules.md`
- `Applications/Foundry Rules/Settings & customization/Permitted and default output values.md`
- `Applications/Foundry Rules/Time series/Deploy time series Foundry Rules.md`
- `Applications/Foundry Rules/Time series/Time series rules [Sunset].md`
- `Applications/Foundry Rules/Troubleshooting reference.md`

### Applications/Machinery - `SI`

Estas paginas ya quedan cubiertas por una app dedicada de Machinery con process graph configurable, connect-data sobre workflows y queue, process mining con filtros y transiciones observadas, y una superficie de analyze-and-monitor sobre path explorer, duration distribution e intervention watchlists.

- `Applications/Machinery/Analyze and monitor a process.md`
- `Applications/Machinery/Connect data to Machinery.md`
- `Applications/Machinery/Core concepts.md`
- `Applications/Machinery/Draw a graph.md`
- `Applications/Machinery/Overview.md`
- `Applications/Machinery/Process mining.md`

### Applications/Map - `SI`

Es uno de los bloques con mayor alineacion funcional.

- `Applications/Map/Add Map templates to a Marketplace product.md`
- `Applications/Map/Configuration/Control Panel.md`
- `Applications/Map/Configuration/Settings.md`
- `Applications/Map/Core concepts.md`
- `Applications/Map/Getting started.md`
- `Applications/Map/Integrate data for the map/Functions for the map.md`
- `Applications/Map/Integrate data for the map/Map Layer Editor.md`
- `Applications/Map/Integrate data for the map/Ontology Actions for the map.md`
- `Applications/Map/Integrate data for the map/Ontology objects for the map.md`
- `Applications/Map/Integrate data for the map/Search Arounds for the map.md`
- `Applications/Map/Interact with maps/Actions.md`
- `Applications/Map/Interact with maps/Add data to a map.md`
- `Applications/Map/Interact with maps/Annotations.md`
- `Applications/Map/Interact with maps/Create and save maps.md`
- `Applications/Map/Interact with maps/Histogram and filtering.md`
- `Applications/Map/Interact with maps/Layer management.md`
- `Applications/Map/Interact with maps/Map interface overview.md`
- `Applications/Map/Interact with maps/Navigation.md`
- `Applications/Map/Interact with maps/Selection.md`
- `Applications/Map/Interact with maps/Shapes.md`
- `Applications/Map/Overview.md`
- `Applications/Map/Templates and Workshop widget/Embed a Map template in a Workshop module.md`
- `Applications/Map/Templates and Workshop widget/Map templates.md`
- `Applications/Map/Time/Events.md`
- `Applications/Map/Time/Overview.md`
- `Applications/Map/Time/Series panel [Planned deprecation].md`
- `Applications/Map/Time/Time selection.md`
- `Applications/Map/Time/Time series.md`
- `Applications/Map/Time/Timeline.md`
- `Applications/Map/Visualize Ontology data/Choropleths.md`
- `Applications/Map/Visualize Ontology data/Clusters.md`
- `Applications/Map/Visualize Ontology data/Lines and polygons.md`
- `Applications/Map/Visualize Ontology data/Loading methods.md`
- `Applications/Map/Visualize Ontology data/Overview.md`
- `Applications/Map/Visualize Ontology data/Points (icons and circles).md`
- `Applications/Map/Visualize Ontology data/Timeline events.md`
- `Applications/Map/Visualize Ontology data/Tracks (moving objects).md`

### Applications/Object Explorer - `SI`

Estas paginas ya quedan cubiertas por una app dedicada de Object Explorer con home/search unificado, tabs por categoria, sidebar de filtros, preview de Object View, pivots sobre linked objects, comparacion de object sets, chart-led exploration, SQL handoff y guardado de explorations walk-up.

- `Applications/Object Explorer/Analyze and compare/Apply Actions.md`
- `Applications/Object Explorer/Analyze and compare/Compare object sets.md`
- `Applications/Object Explorer/Analyze and compare/Explore with charts.md`
- `Applications/Object Explorer/Analyze and compare/Filter results.md`
- `Applications/Object Explorer/Analyze and compare/Pivot to explore linked objects.md`
- `Applications/Object Explorer/Analyze and compare/Save explorations.md`
- `Applications/Object Explorer/Analyze and compare/Save lists.md`
- `Applications/Object Explorer/Analyze and compare/View results.md`
- `Applications/Object Explorer/Configure Object Explorer.md`
- `Applications/Object Explorer/Generate Object Explorer URLs.md`
- `Applications/Object Explorer/Getting started.md`
- `Applications/Object Explorer/Overview.md`
- `Applications/Object Explorer/Search and explore objects/Analyze using SQL.md`
- `Applications/Object Explorer/Search and explore objects/Search for objects.md`
- `Applications/Object Explorer/Search and explore objects/Search syntax.md`

### Applications/Object Monitors [Sunset] - `NO`

No encontre un equivalente claro como producto o app dedicada.

- `Applications/Object Monitors [Sunset]/Core concepts/Actions.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Activity.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Conditions.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Evaluation.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Inputs.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Monitors.md`
- `Applications/Object Monitors [Sunset]/Core concepts/Notifications.md`
- `Applications/Object Monitors [Sunset]/Create a new object monitor.md`
- `Applications/Object Monitors [Sunset]/Error reference.md`
- `Applications/Object Monitors [Sunset]/Monitoring limits.md`
- `Applications/Object Monitors [Sunset]/Overview.md`

### Applications/Object Views - `PARCIAL`

Existe object view como runtime y panel tecnico, pero no la familia de Object Views configurable mostrada por Palantir.

- `Applications/Object Views/Add Object Views to a Marketplace product.md`
- `Applications/Object Views/Branching object views.md`
- `Applications/Object Views/Comment on objects.md`
- `Applications/Object Views/Configured Object View overview.md`
- `Applications/Object Views/Full Object Views/Configure full Object Views.md`
- `Applications/Object Views/Full Object Views/Use full Object Views in the platform.md`
- `Applications/Object Views/Generate Object View URLs.md`
- `Applications/Object Views/Legacy Object Views/Apps and Files.md`
- `Applications/Object Views/Legacy Object Views/Configure legacy Object Views.md`
- `Applications/Object Views/Legacy Object Views/Configure profiles.md`
- `Applications/Object Views/Legacy Object Views/Configure tabs.md`
- `Applications/Object Views/Legacy Object Views/Configure the applications sidebar.md`
- `Applications/Object Views/Legacy Object Views/Filtering.md`
- `Applications/Object Views/Legacy Object Views/Layout.md`
- `Applications/Object Views/Legacy Object Views/Properties and Links.md`
- `Applications/Object Views/Legacy Object Views/Visualization.md`
- `Applications/Object Views/Manage configured Object View versions.md`
- `Applications/Object Views/Overview.md`
- `Applications/Object Views/Panel Object Views/Configure panel Object Views.md`
- `Applications/Object Views/Panel Object Views/Use panel Object Views in the platform.md`
- `Applications/Object Views/Standard Object Views.md`

### Applications/Ontology Manager - `PARCIAL`

El workbench ontologico existe, pero no la misma superficie de producto separada.

- `Applications/Ontology Manager/Change management/Review and restore changes.md`
- `Applications/Ontology Manager/Change management/Save changes to the Ontology.md`
- `Applications/Ontology Manager/Export, edit, and import an Ontology.md`
- `Applications/Ontology Manager/Migrate to project-based permissions.md`
- `Applications/Ontology Manager/Navigation.md`
- `Applications/Ontology Manager/Ontology cleanup.md`
- `Applications/Ontology Manager/Ontology roles migration [Legacy].md`
- `Applications/Ontology Manager/Overview.md`
- `Applications/Ontology Manager/Viewing usage.md`

### Applications/Vertex - `PARCIAL`

Hay graph exploration y Quiver, pero no un Vertex equivalente con el mismo alcance funcional.

- `Applications/Vertex/Add Vertex graph templates to a Marketplace product.md`
- `Applications/Vertex/Configuration/Configure link merging.md`
- `Applications/Vertex/Configuration/Configure settings in Control Panel.md`
- `Applications/Vertex/Events and time series/Configure events.md`
- `Applications/Vertex/Events and time series/Configure thresholds.md`
- `Applications/Vertex/Events and time series/Explore related events.md`
- `Applications/Vertex/Events and time series/Explore related time series.md`
- `Applications/Vertex/Events and time series/Overview.md`
- `Applications/Vertex/Events and time series/Use time selection.md`
- `Applications/Vertex/Events and time series/View and filter events on a timeline.md`
- `Applications/Vertex/Example use cases.md`
- `Applications/Vertex/Graphs/Create a graph template.md`
- `Applications/Vertex/Graphs/Derive properties using Functions.md`
- `Applications/Vertex/Graphs/Embed a graph in a Workshop module.md`
- `Applications/Vertex/Graphs/Explore existing graphs.md`
- `Applications/Vertex/Graphs/Explore object relationships.md`
- `Applications/Vertex/Graphs/Generate a graph from other applications.md`
- `Applications/Vertex/Graphs/Generate graphs using Functions.md`
- `Applications/Vertex/Graphs/Media layers and image annotations.md`
- `Applications/Vertex/Graphs/Object and edge display options.md`
- `Applications/Vertex/Graphs/Read-only mode.md`
- `Applications/Vertex/Graphs/Save, share, and collaborate.md`
- `Applications/Vertex/Overview.md`
- `Applications/Vertex/Scenarios/Configure chained models [Sunset].md`
- `Applications/Vertex/Scenarios/Getting started.md`
- `Applications/Vertex/Scenarios/Overview.md`
- `Applications/Vertex/Scenarios/Scenario options.md`

### Define Ontologies/Action types - `PARCIAL`

Es uno de los bloques mas desarrollados del repo, pero no cubre todo el lifecycle del set documental.

- `Define Ontologies/Action types/Action log.md`
- `Define Ontologies/Action types/Action metrics.md`
- `Define Ontologies/Action types/Actions on interfaces.md`
- `Define Ontologies/Action types/Actions on structs.md`
- `Define Ontologies/Action types/Add action types to a Marketplace product.md`
- `Define Ontologies/Action types/Configure sections.md`
- `Define Ontologies/Action types/Function-backed actions/Batched execution.md`
- `Define Ontologies/Action types/Function-backed actions/Getting started.md`
- `Define Ontologies/Action types/Function-backed actions/Overview.md`
- `Define Ontologies/Action types/Getting started.md`
- `Define Ontologies/Action types/Inline edits.md`
- `Define Ontologies/Action types/Monitoring.md`
- `Define Ontologies/Action types/Overview.md`
- `Define Ontologies/Action types/Parameters/Filter results of a parameter dropdown.md`
- `Define Ontologies/Action types/Parameters/Object dropdown security considerations.md`
- `Define Ontologies/Action types/Parameters/Override parameter configurations.md`
- `Define Ontologies/Action types/Parameters/Overview.md`
- `Define Ontologies/Action types/Parameters/Performance considerations.md`
- `Define Ontologies/Action types/Parameters/Set parameter default value.md`
- `Define Ontologies/Action types/Permissions.md`
- `Define Ontologies/Action types/Rules.md`
- `Define Ontologies/Action types/Scale and property limits.md`
- `Define Ontologies/Action types/Side effects/Notifications.md`
- `Define Ontologies/Action types/Side effects/Overview.md`
- `Define Ontologies/Action types/Side effects/Set up a notification.md`
- `Define Ontologies/Action types/Side effects/Set up a webhook.md`
- `Define Ontologies/Action types/Side effects/Webhooks.md`
- `Define Ontologies/Action types/Submission criteria.md`
- `Define Ontologies/Action types/Undo or revert Actions.md`
- `Define Ontologies/Action types/Upload attachments.md`
- `Define Ontologies/Action types/Upload media.md`
- `Define Ontologies/Action types/Use actions in the platform.md`

### Define Ontologies/Functions - `PARCIAL`

Hay base funcional fuerte, pero el arbol documental de Palantir cubre bastante mas ciclo de vida y tooling.

- `Define Ontologies/Functions/Feature support by language.md`
- `Define Ontologies/Functions/Function consumption/Function metrics.md`
- `Define Ontologies/Functions/Function consumption/Optimize performance.md`
- `Define Ontologies/Functions/Function consumption/Use functions in the platform.md`
- `Define Ontologies/Functions/Function consumption/Version range dependencies.md`
- `Define Ontologies/Functions/Function management/Add functions to a Marketplace product.md`
- `Define Ontologies/Functions/Function management/Function monitoring.md`
- `Define Ontologies/Functions/Function management/Function versioning.md`
- `Define Ontologies/Functions/Function management/Instrumentation and telemetry.md`
- `Define Ontologies/Functions/Function management/Manage published functions.md`
- `Define Ontologies/Functions/Function management/Permissions.md`
- `Define Ontologies/Functions/Functions on objects/Attachments.md`
- `Define Ontologies/Functions/Functions on objects/Create a custom aggregation.md`
- `Define Ontologies/Functions/Functions on objects/Getting started.md`
- `Define Ontologies/Functions/Functions on objects/Import object, interface, and link types.md`
- `Define Ontologies/Functions/Functions on objects/Media.md`
- `Define Ontologies/Functions/Functions on objects/Object identifiers.md`
- `Define Ontologies/Functions/Functions on objects/Object sets.md`
- `Define Ontologies/Functions/Functions on objects/Objects and links.md`
- `Define Ontologies/Functions/Functions on objects/Overview.md`
- `Define Ontologies/Functions/Getting started.md`
- `Define Ontologies/Functions/Language agnostic features/Configure notifications.md`
- `Define Ontologies/Functions/Language agnostic features/Deploy functions.md`
- `Define Ontologies/Functions/Language agnostic features/Make API calls from functions.md`
- `Define Ontologies/Functions/Language agnostic features/Ontology edits.md`
- `Define Ontologies/Functions/Language agnostic features/Publish and call query functions through API gateway.md`
- `Define Ontologies/Functions/Language agnostic features/Use platform APIs with the Foundry platform SDK.md`
- `Define Ontologies/Functions/Language agnostic features/User-facing errors.md`
- `Define Ontologies/Functions/Models/Function interfaces.md`
- `Define Ontologies/Functions/Models/Functions on models.md`
- `Define Ontologies/Functions/Models/Language models in TypeScript v1 functions.md`
- `Define Ontologies/Functions/Models/Language models in TypeScript v2 and Python functions.md`
- `Define Ontologies/Functions/Models/Legacy language models in functions.md`
- `Define Ontologies/Functions/Overview.md`
- `Define Ontologies/Functions/Python/Create a custom aggregation.md`
- `Define Ontologies/Functions/Python/Functions on objects.md`
- `Define Ontologies/Functions/Python/Getting started.md`
- `Define Ontologies/Functions/Python/Local development.md`
- `Define Ontologies/Functions/Python/Ontology edits.md`
- `Define Ontologies/Functions/Python/Use Python functions in Pipeline Builder.md`
- `Define Ontologies/Functions/Python/Use Python functions in Workshop.md`
- `Define Ontologies/Functions/TypeScript v1/Add NPM dependencies.md`
- `Define Ontologies/Functions/TypeScript v1/Debug functions.md`
- `Define Ontologies/Functions/TypeScript v1/Decorators.md`
- `Define Ontologies/Functions/TypeScript v1/Error types.md`
- `Define Ontologies/Functions/TypeScript v1/Generate unique IDs for new objects.md`
- `Define Ontologies/Functions/TypeScript v1/Getting started.md`
- `Define Ontologies/Functions/TypeScript v1/Handle undefined values.md`
- `Define Ontologies/Functions/TypeScript v1/Import resources into Code Repositories.md`
- `Define Ontologies/Functions/TypeScript v1/Migrate from TypeScript v1 to TypeScript v2.md`
- `Define Ontologies/Functions/TypeScript v1/Ontology edits.md`
- `Define Ontologies/Functions/TypeScript v1/Use webhooks.md`
- `Define Ontologies/Functions/TypeScript v2/Getting started.md`
- `Define Ontologies/Functions/TypeScript v2/Ontology edits.md`
- `Define Ontologies/Functions/TypeScript v2/Ontology transactions.md`
- `Define Ontologies/Functions/Types reference.md`
- `Define Ontologies/Functions/Unit testing/Create stub objects.md`
- `Define Ontologies/Functions/Unit testing/Debug.md`
- `Define Ontologies/Functions/Unit testing/Getting started.md`
- `Define Ontologies/Functions/Unit testing/Mock dates, timestamps, and UUIDs.md`
- `Define Ontologies/Functions/Unit testing/Mock users and groups.md`
- `Define Ontologies/Functions/Unit testing/Stub object searches and aggregations.md`
- `Define Ontologies/Functions/Unit testing/Verify Ontology edits.md`

### Define Ontologies/Interfaces - `SI`

Hay correspondencia funcional clara.

- `Define Ontologies/Interfaces/Create an interface.md`
- `Define Ontologies/Interfaces/Edit an interface definition.md`
- `Define Ontologies/Interfaces/Edit an interface implementation.md`
- `Define Ontologies/Interfaces/Extend an interface.md`
- `Define Ontologies/Interfaces/Implement an interface.md`
- `Define Ontologies/Interfaces/Interface link types.md`
- `Define Ontologies/Interfaces/Metadata reference.md`
- `Define Ontologies/Interfaces/Overview.md`

### Define Ontologies/Object and link types - `PARCIAL`

La base ontologica existe, pero no todo el sistema de tipos ampliado del set documental.

- `Define Ontologies/Object and link types/Add Ontology types to a Marketplace product.md`
- `Define Ontologies/Object and link types/Allow users to edit objects and links.md`
- `Define Ontologies/Object and link types/Link types/Create a link type.md`
- `Define Ontologies/Object and link types/Link types/Edit link types.md`
- `Define Ontologies/Object and link types/Link types/Metadata reference.md`
- `Define Ontologies/Object and link types/Link types/Overview.md`
- `Define Ontologies/Object and link types/Metadata/Render hints.md`
- `Define Ontologies/Object and link types/Metadata/Statuses.md`
- `Define Ontologies/Object and link types/Metadata/Type classes.md`
- `Define Ontologies/Object and link types/Object type groups.md`
- `Define Ontologies/Object and link types/Object types/Copy object type configuration.md`
- `Define Ontologies/Object and link types/Object types/Create Ontology objects from Gaia.md`
- `Define Ontologies/Object and link types/Object types/Create an object type.md`
- `Define Ontologies/Object and link types/Object types/Edit object types.md`
- `Define Ontologies/Object and link types/Object types/Enable Gotham integration.md`
- `Define Ontologies/Object and link types/Object types/Metadata reference.md`
- `Define Ontologies/Object and link types/Object types/Overview.md`
- `Define Ontologies/Object and link types/Properties/Add conditional formatting.md`
- `Define Ontologies/Object and link types/Properties/Add value formatting.md`
- `Define Ontologies/Object and link types/Properties/Base types.md`
- `Define Ontologies/Object and link types/Properties/Derived properties.md`
- `Define Ontologies/Object and link types/Properties/Edit object type properties.md`
- `Define Ontologies/Object and link types/Properties/Edit-only properties.md`
- `Define Ontologies/Object and link types/Properties/Mandatory control properties.md`
- `Define Ontologies/Object and link types/Properties/Metadata reference.md`
- `Define Ontologies/Object and link types/Properties/Overview.md`
- `Define Ontologies/Object and link types/Properties/Property reducers.md`
- `Define Ontologies/Object and link types/Properties/Required properties.md`
- `Define Ontologies/Object and link types/Shared properties/Create shared properties.md`
- `Define Ontologies/Object and link types/Shared properties/Edit shared properties.md`
- `Define Ontologies/Object and link types/Shared properties/Metadata reference.md`
- `Define Ontologies/Object and link types/Shared properties/Overview.md`
- `Define Ontologies/Object and link types/Shared properties/Use shared properties on object types.md`
- `Define Ontologies/Object and link types/Structs/Automapping struct types.md`
- `Define Ontologies/Object and link types/Structs/Create a struct type.md`
- `Define Ontologies/Object and link types/Structs/Designate struct main fields.md`
- `Define Ontologies/Object and link types/Structs/Edit struct types.md`
- `Define Ontologies/Object and link types/Structs/Overview.md`
- `Define Ontologies/Object and link types/Structs/Structs and shared properties.md`
- `Define Ontologies/Object and link types/Types reference.md`
- `Define Ontologies/Object and link types/Value types/Create a value type.md`
- `Define Ontologies/Object and link types/Value types/Overview.md`
- `Define Ontologies/Object and link types/Value types/Use value types.md`
- `Define Ontologies/Object and link types/Value types/Value type constraints.md`
- `Define Ontologies/Object and link types/Value types/Value type permissions.md`
- `Define Ontologies/Object and link types/Value types/Value type versions.md`

### Define Ontologies/Ontologies - `PARCIAL`

Hay scoping por proyectos y recursos, pero no un lifecycle de ontologias equivalente al de Palantir.

- `Define Ontologies/Ontologies/Migrating between ontologies.md`
- `Define Ontologies/Ontologies/Ontology branches [Legacy].md`
- `Define Ontologies/Ontologies/Overview.md`
- `Define Ontologies/Ontologies/Review ontology proposals.md`
- `Define Ontologies/Ontologies/Shared ontologies.md`
- `Define Ontologies/Ontologies/Test changes in the ontology.md`
- `Define Ontologies/Ontologies/Usage/Ontology indexing compute.md`
- `Define Ontologies/Ontologies/Usage/Ontology query compute.md`
- `Define Ontologies/Ontologies/Usage/Ontology volume.md`

### Define Ontologies/Ontology design Best practices and anti-patterns.md - `N/A`

Documento conceptual, no feature verificable.

- `Define Ontologies/Ontology design Best practices and anti-patterns.md`

### Ontology architecture/Indexing - `PARCIAL`

Existe funnel batch/indexing, pero no todo el marco documental avanzado de Palantir.

- `Ontology architecture/Indexing/Data restrictions.md`
- `Ontology architecture/Indexing/FAQ.md`
- `Ontology architecture/Indexing/Funnel batch pipelines.md`
- `Ontology architecture/Indexing/Funnel streaming pipelines.md`
- `Ontology architecture/Indexing/Overview.md`

### Ontology architecture/Object databases - `N/A`

Tema de implementacion interna de Palantir no comparable desde este repo.

- `Ontology architecture/Object databases/Object Storage V1 (Phonograph) [Planned deprecation].md`

### Ontology architecture/Object edits and materializations - `PARCIAL`

Hay write path y simulacion, pero no el mismo modelo de edit history/conflict management.

- `Ontology architecture/Object edits and materializations/Enable user edit history.md`
- `Ontology architecture/Object edits and materializations/How user edits are applied.md`
- `Ontology architecture/Object edits and materializations/Manage schema changes.md`
- `Ontology architecture/Object edits and materializations/Materializations.md`
- `Ontology architecture/Object edits and materializations/Overview.md`
- `Ontology architecture/Object edits and materializations/Permission checks for Actions.md`

### Ontology architecture/Object permissioning - `PARCIAL`

Hay seguridad real, pero mas simple que la arquitectura descrita por Palantir.

- `Ontology architecture/Object permissioning/Configuring restricted-view-backed object types.md`
- `Ontology architecture/Object permissioning/Legacy ontology permissions.md`
- `Ontology architecture/Object permissioning/Managing object security.md`
- `Ontology architecture/Object permissioning/Multi-datasource object types (MDOs).md`
- `Ontology architecture/Object permissioning/Object security policies.md`
- `Ontology architecture/Object permissioning/Ontology permissions.md`
- `Ontology architecture/Object permissioning/Overview.md`

### Ontology architecture/Overview and getting started - `N/A`

Arquitectura y migraciones especificas de Palantir (`OSv1`, `OSv2`).

- `Ontology architecture/Overview and getting started/Aggregation considerations.md`
- `Ontology architecture/Overview and getting started/Breaking changes between OSv1 and OSv2.md`
- `Ontology architecture/Overview and getting started/Migrate from OSv1 to OSv2.md`
- `Ontology architecture/Overview and getting started/Overview.md`

### Ontology search/Derived properties.md - `NO`

No encontre esta capacidad como primitive first-class del codigo actual.

- `Ontology search/Derived properties.md`

### Ontology search/Search syntax.md - `PARCIAL`

Hay search, pero no una sintaxis o DSL equivalente claramente expuesta.

- `Ontology search/Search syntax.md`

### Ontology search/Semantic search - `PARCIAL`

Hay semantic search real, pero no todo el producto ampliado de Palantir alrededor de documentos, multimodalidad y OAG.

- `Ontology search/Semantic search/Document processing.md`
- `Ontology search/Semantic search/Ontology augmented generation.md`
- `Ontology search/Semantic search/Overview.md`
- `Ontology search/Semantic search/Process multimodal and embedding models.md`
- `Ontology search/Semantic search/Use Palantir-provided models to create a semantic search workflow.md`
- `Ontology search/Semantic search/Use custom models to create a semantic search workflow.md`

## Conclusiones operativas

- Si la pregunta es "OpenFoundry ya tiene una ontologia real comparable en nucleo?" la respuesta es **si**.
- Si la pregunta es "coincide 1:1 con la suite de `Ontology building` que muestran los docs e imagenes de Palantir?" la respuesta es **no; coincide parcialmente**.
- La distancia principal no esta en CRUD basico sino en:
  - separacion de productos,
  - lifecycle de ontologias,
  - UX especializada,
  - permissioning avanzado,
  - y ciertas capacidades first-class como `Configured Object Views`, `Dynamic Scheduling` o `Derived properties`.
