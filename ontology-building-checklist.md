# Checklist de correspondencia: Ontology Building vs OpenFoundry

Estado de referencia basado en el código actual del monorepo.

Leyenda:

- `✅` implementado o claramente presente
- `⚠️` presente de forma parcial, incipiente o repartida
- `❌` no encontrado en el código revisado
- `🔘` no aplica o no se puede verificar solo desde este repositorio

## 1. Tipos de acción

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| CRUD de action types | ✅ | `services/ontology-service/src/main.rs`, `services/ontology-service/src/handlers/actions.rs` | Hay endpoints para crear, listar, leer, actualizar y borrar acciones. |
| Esquema tipado de inputs | ✅ | `services/ontology-service/src/models/action_type.rs` | `ActionInputField` soporta tipo, requerido, descripción y `default_value`. |
| Validación previa a ejecución | ✅ | `POST /api/v1/ontology/actions/{id}/validate`, `plan_action(...)` | La acción puede validarse antes de aplicarse. |
| Ejecución por lotes | ✅ | `POST /api/v1/ontology/actions/{id}/execute-batch` | Existe ejecución batch explícita. |
| What-if branches | ✅ | `ActionWhatIfBranch`, rutas `/what-if` | Ya hay ramas de simulación para comparar antes y después. |
| Acciones function-backed | ✅ | `invoke_function`, `domain/function_runtime.rs` | Soportado como tipo de operación. |
| Acciones webhook-backed | ✅ | `invoke_webhook`, `validate_http_invocation_config(...)` | Soporte HTTP con método, URL y cabeceras. |
| Confirmación con justificación | ✅ | `confirmation_required`, `ensure_confirmation_justification(...)` | Las acciones pueden exigir explicación del usuario. |
| Autorización granular por acción | ✅ | `ActionAuthorizationPolicy`, `ensure_action_actor_permission(...)` | Soporta permisos, roles, atributos, markings y clearance. |
| Notificaciones como side effect nativo del action type | ✅ | `services/ontology-service/src/handlers/actions.rs`, `services/notification-service/src/handlers/send.rs` | El `config` del action type ya soporta `notification_side_effects` nativos, disparados tras una ejecución exitosa. |
| Formularios con secciones, overrides y layout condicional | ⚠️ | `services/ontology-service/src/models/action_type.rs`, `services/ontology-service/src/handlers/actions.rs`, `apps/web/src/lib/components/ontology/ActionExecutor.svelte`, `apps/web/src/routes/ontology/[id]/+page.svelte` | Ya existe `form_schema` nativo con `sections`, `parameter_overrides` y evaluación condicional, además de renderer web estructurado. El authoring sigue siendo JSON-first, no un diseñador visual completo. |
| Inline edits respaldados por acciones | ⚠️ | `services/ontology-service/src/models/property.rs`, `services/ontology-service/src/handlers/actions.rs`, `services/ontology-service/src/handlers/properties.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` | Ya existe `inline_edit_config` por propiedad, un endpoint dedicado que ejecuta el action type asociado y una UI de configuración/ejecución en Object Lab. Sigue siendo una superficie acotada, no una experiencia inline transversal en Object Explorer o tablas nativas. |
| Adjuntos o media como parámetros de acción | ⚠️ | `services/ontology-service/src/domain/type_system.rs`, `services/ontology-service/src/handlers/actions.rs`, `apps/web/src/lib/components/ontology/ActionExecutor.svelte`, `apps/web/src/routes/ontology/[id]/+page.svelte` | `ActionInputField` ya puede usar `media_reference` porque reutiliza el sistema de tipos del ontology. La ejecución y el renderer aceptan URI/URL o un objeto con `uri`/`url`. Aún no existe subida binaria nativa ni un workflow first-class de attachments. |
| Contrato protobuf maduro para acciones | ❌ | `proto/ontology/action.proto` está vacío | La API existe por REST, pero el contrato proto aún no está definido. |

## 2. Functions

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| Registro de function packages | ✅ | `services/ontology-service/src/models/function_package.rs` | Se modelan paquetes con runtime, source y entrypoint. |
| CRUD de function packages | ✅ | `services/ontology-service/src/handlers/functions.rs` | Hay create, list, get, update y delete. |
| Validación de paquetes | ✅ | `/api/v1/ontology/functions/{id}/validate` | Existe endpoint dedicado. |
| Simulación de paquetes | ✅ | `/api/v1/ontology/functions/{id}/simulate` | Existe simulación con contexto de objeto. |
| Capacidades declarativas | ✅ | `FunctionCapabilities` | Controla lectura/escritura ontology, AI, red, timeout y tamaño máximo. |
| Integración con acciones | ✅ | `invoke_function`, `synthetic_action(...)` | Las funciones ya pueden operar como backend de acciones. |
| Runtimes inline soportados | ✅ | `domain/function_runtime.rs` | El runtime ya resuelve configuraciones inline. |
| Versionado o auto-upgrade de funciones | ⚠️ | `services/ontology-service/src/models/function_package.rs`, `services/ontology-service/src/domain/function_runtime.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` | Los function packages ya tienen `version` semver y las acciones pueden resolver por `function_package_name` + `function_package_version`, con `function_package_auto_upgrade` para upgrades compatibles en la misma major estable. Sigue faltando un publish flow inmutable y más producto alrededor del release lifecycle. |
| Repositorios/plantillas first-class para authoring de funciones ontológicas | ✅ | `services/ontology-service/src/handlers/functions.rs`, `services/ontology-service/src/models/function_authoring.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte`, `tools/of-cli/src/main.rs` | El ontology-service ya expone un catálogo nativo de authoring kits con templates TypeScript/Python, referencias a SDKs y comandos de scaffold. La UI de Functions Platform lo consume y `of-cli` ya puede crear workspaces `function-typescript` y `function-python`. Sigue sin ser un producto completo de repos Git alojados, pero la capacidad pedida de repos/templates first-class ya existe. |
| Métricas o monitoring nativo de funciones | ✅ | `services/ontology-service/src/domain/function_metrics.rs`, `services/ontology-service/src/handlers/functions.rs`, `apps/web/src/routes/ontology/[id]/+page.svelte` | Ya existe un ledger nativo de runs por function package, con endpoints de histórico y agregados (success/failure, action vs simulation, avg/P95 duration) y un panel visible en Functions Platform. Hoy se centra en paquetes reutilizables y en invocaciones de actions que resuelven a esos paquetes. |

## 3. Búsqueda semántica

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| Búsqueda híbrida full-text + semantic | ✅ | `services/ontology-service/src/domain/search/mod.rs`, `services/ontology-service/src/domain/search/semantic.rs`, `services/ontology-service/src/models/search.rs` | `ontology-service` ya hace hybrid retrieval en dos fases: recall léxico + recall semántico barato para candidate pool, seguido de semantic reranking provider-backed y fusión explícita (`rrf` o `weighted`). Mantiene fallback `deterministic-hash`, pero ya no depende solo de él. |
| Flag para activar/desactivar parte semántica | ✅ | `SearchRequest.semantic`, `semantic_enabled` | El comportamiento semántico puede controlarse por request. |
| Knowledge bases con embeddings | ✅ | `services/ai-service/src/handlers/knowledge.rs` | Hay knowledge bases, documentos, provider de embeddings y retrieval. |
| Chunk retrieval para RAG | ✅ | `services/ai-service/src/domain/rag/*` | Ya existe path claro de embedding y retrieval. |
| Vector store como librería de plataforma | ✅ | `libs/vector-store` | Existe librería dedicada. |
| KNN sobre propiedades vectoriales del ontology | ✅ | `services/ontology-service/src/handlers/objects.rs`, `services/ontology-service/src/models/search.rs`, `services/ontology-service/src/domain/function_runtime.rs` | Ya existe una API nativa `POST /api/v1/ontology/types/{type_id}/objects/knn` sobre propiedades `vector`, con query por `anchor_object_id` o `query_vector`, métricas `cosine`/`dot_product`/`euclidean` y exposición también en el runtime de funciones. |
| Multimodal semantic search | ❌ | No encontrado | No aparece una ruta multimodal explícita. |
| Search permission-aware end-to-end | ⚠️ | `indexer::build_search_documents(...)` usa `claims` | Hay señales de scoping por usuario, pero la arquitectura completa aún parece repartida. |

## 4. Permissioning de objetos

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| RBAC básico | ✅ | `services/auth-service/src/domain/rbac.rs`, `libs/auth-middleware` | Permisos y roles están bien presentes. |
| ABAC | ✅ | `services/auth-service/src/domain/abac.rs` | Evaluación por atributos, resource, row filters y denies. |
| Restricted views | ✅ | `services/auth-service/src/handlers/restricted_views.rs` | Hay CRUD y validación de restricted views. |
| Row filters | ✅ | `Policy.row_filter`, `render_row_filter(...)` | La evaluación ABAC devuelve filtros de fila. |
| Hidden columns | ✅ | `hidden_columns` | Ya existe redacción a nivel de columnas. |
| Markings y clearance | ✅ | `allowed_markings`, `minimum_clearance` | Presente en ABAC, object sets y acciones. |
| Guest session restrictions | ✅ | `deny_guest_sessions`, validaciones de sesión | El modelo contempla sesiones invitadas restringidas. |
| Enforcements sobre actions | ✅ | `ensure_action_actor_permission(...)`, `ensure_action_target_permission(...)` | La autorización de mutaciones ya existe. |
| Políticas sobre object sets | ✅ | `services/ontology-service/src/domain/object_sets.rs` | Los object sets pueden exigir markings, clearance y restricted views. |
| Permissioning “project-based ontology resources” | ❌ | No encontrado como concepto explícito | No aparece un modelo equivalente de permisos por proyecto/carpeta para recursos ontológicos. |

## 5. Indexing y materialización

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| Ruta de ingestión batch | ✅ | `services/ontology-service/src/handlers/funnel.rs` | Ya existe una ruta batch explícita: el ontology funnel coordina dataset, pipeline opcional y upsert sobre `object_instances`. |
| Ruta de ingestión streaming | ⚠️ | `services/streaming-service` | Hay servicio de streaming, pero no una semántica end-to-end totalmente documentada para ontology indexing. |
| Indexador de documentos de búsqueda | ✅ | `services/ontology-service/src/domain/indexer.rs` | La búsqueda depende de documentos indexados. |
| Materialización de object sets | ✅ | `materialized_snapshot`, `materialized_at`, `materialized_row_count` | Sí existe snapshot materializado. |
| Orquestador tipo “funnel” para Ontology | ✅ | `services/ontology-service/src/handlers/funnel.rs`, `services/ontology-service/src/models/funnel.rs` | `ontology-service` ya expone funnel sources y funnel runs como capa explícita de orquestación batch hacia la ontología. |
| Observabilidad dedicada de indexing ontológico | ✅ | `services/ontology-service/src/handlers/funnel.rs`, `services/ontology-service/src/models/funnel.rs` | Ya existe una superficie nativa de funnel health con métricas agregadas, clasificación por source y estado operativo del indexing batch. |

## 6. Ediciones, writeback y resolución de conflictos

| Capacidad | Estado | Evidencia principal | Observación |
|---|---|---|---|
| Mutación directa de object instances | ✅ | `services/ontology-service/src/handlers/objects.rs` | La ontología ya no es solo de lectura. |
| Ediciones mediante acciones | ✅ | `services/ontology-service/src/handlers/actions.rs` | Es la vía gobernada principal. |
| Links creados como consecuencia de acciones | ✅ | `create_link`, `FunctionLinkInstruction` | Las acciones pueden afectar relaciones, no solo propiedades. |
| Branches what-if para cambios | ✅ | `ActionWhatIfBranch` | Hay base para simulación de cambios. |
| Historial durable de edits separado del datasource | ⚠️ | Señales en `audit-service`, pero no un modelo claro de writeback merge | Hay trazabilidad posible, pero no una capa explícita de “edits queue + merge”. |
| Estrategia “user edits win” o “most recent value wins” | ❌ | No encontrado | No aparece configuración de conflicto por datasource/propiedad. |
| Revert nativo de acciones completadas | ❌ | No encontrado | Hay simulación/what-if, pero no revert explícito de una acción aplicada. |
| Edit-only properties | ❌ | No encontrado | No aparece un tipo de propiedad o estrategia dedicada a edit-only. |
| Migraciones de edits ante cambios de schema | ❌ | No encontrado | No se ve una capa equivalente a schema migrations de user edits. |

## 7. Conclusión rápida

- `✅` OpenFoundry ya tiene una base real y bastante seria para acciones, funciones, permissioning y recuperación semántica.
- `⚠️` Donde más se nota que aún está en fase de consolidación es en streaming/indexing ontológico avanzado, release lifecycle de funciones y semántica completa de writeback.
- `❌` Las brechas más claras frente a una plataforma ontology madura están en conflicto entre datasource y user edits, contratos protobuf completos y semántica avanzada de streaming/index lifecycle.
- `⚠️` Los formularios de acciones ya tienen contrato y renderer estructurado, pero todavía no alcanzan el nivel de un diseñador visual avanzado.
