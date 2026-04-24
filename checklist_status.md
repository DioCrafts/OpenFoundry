# Estado del Checklist de Foundry vs OpenFoundry

Este documento traduce el `checklist.md` original a una matriz de estado basada en la evidencia actual del código de OpenFoundry. El archivo original se mantiene como referencia; aquí sintetizo cumplimiento, evidencia visible y gap concreto.

## Método

- `Cumple`: existe implementación funcional clara y verificable en el repo.
- `Parcial`: hay base técnica, endpoints o UI, pero la capacidad está incompleta, simulada o limitada.
- `No`: no encontré implementación real, o lo existente es placeholder o vacío.
- La evaluación está hecha sobre el código y la estructura del repositorio, no sobre despliegues externos ni integraciones productivas.
- Verificaciones previas relevantes: `cargo check --workspace` OK, `cargo test --workspace` OK pero sin tests efectivos en crates Rust, `pnpm --filter @open-foundry/web check` OK, `pnpm --filter @open-foundry/web test` fallando por mezcla de Vitest con un spec E2E de Playwright.
- Verificaciones P1 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p data-connector -p dataset-service -p of-cli` OK, `cargo test -p data-connector -p dataset-service -p of-cli` OK y `target/debug/of smoke run --scenario smoke/scenarios/p0-critical-path.json --output smoke/results/p0-critical-path.json` OK contra servicios vivos y Postgres local.

## Resumen por Dominio

| Dominio | Estado global | Cumple | Parcial | No |
| --- | --- | ---: | ---: | ---: |
| 1. CONECTIVIDAD E INTEGRACIÓN DE DATOS | Parcial medio | 6 | 22 | 23 |
| 2. ONTOLOGY (CAPA SEMÁNTICA Y OPERACIONAL) | Parcial medio | 5 | 10 | 21 |
| 3. CONECTIVIDAD Y DESARROLLO DE MODELOS (MLOps) | Parcial bajo | 1 | 19 | 9 |
| 4. DESARROLLO DE CASOS DE USO (APLICACIONES) | Parcial medio | 2 | 9 | 13 |
| 5. ANALYTICS | Parcial medio | 5 | 14 | 21 |
| 6. PRODUCT DELIVERY (DevOps y Marketplace) | Parcial bajo | 1 | 6 | 3 |
| 7. SEGURIDAD Y GOBERNANZA | Parcial medio | 3 | 9 | 10 |
| 8. MANAGEMENT Y ENABLEMENT | Bajo | 0 | 8 | 4 |
| 9. DEVELOPER TOOLCHAIN (APIs y SDKs) | Parcial medio | 2 | 7 | 5 |
| 10. INFRAESTRUCTURA Y DEPLOYMENT | Parcial medio | 0 | 3 | 3 |

## Matriz Detallada

### 1.1 Data Connection (Conectores y Fuentes)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.1.1. Conectores nativos (200+) | No | En `connections.rs` solo se enrutan `postgresql`, `csv`, `json` y `rest_api`; conectores enterprise como `sap.rs`, `salesforce.rs`, `snowflake.rs`, `bigquery.rs` y `kafka.rs` están vacíos. | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.2. Conector SAP (Foundry Connector 2.0) | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.3. Streaming data sources | Parcial | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.4. REST API source | Cumple | `rest_api.rs` hace `test_connection` HTTP real, sincroniza payloads JSON reales y el smoke `create_connection -> test_connection -> sync -> preview` quedó verde contra `/health` del gateway. | Queda endurecer incrementalidad, paginación avanzada, secretos y políticas de egress. |
| 1.1.5. Generic source / custom connectors | Parcial | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.6. IoT / IIoT data sources | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.7. On-premises agent | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.8. Virtual tables (zero-copy) | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.9. Auto-registration de tablas | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.10. Bulk registration | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.11. Update detection / versioning | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |
| 1.1.12. Export / egress controls | No | `services/data-connector/src/handlers/connections.rs`, `services/data-connector/src/handlers/sync_ops.rs`, `services/data-connector/src/connectors/*` | Implementar conectores enterprise reales, `test_connection` de verdad y sync persistente/asíncrono. |

### 1.2 Pipeline Builder

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.2.1. Interfaz visual drag-and-drop | Parcial | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.2. Transforms batch (PySpark) | No | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.3. Transforms ligeros (Polars/Python) | Parcial | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.4. LLM-powered transforms | No | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.5. External compute orchestration | No | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.6. Streaming pipelines | Parcial | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.7. Scheduling e integración de builds | Parcial | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.8. Incremental transforms | No | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.9. Multi-language pipelines | Cumple | `domain/engine/mod.rs` ejecuta nodos `sql`, `python`, `wasm` y `passthrough`. | Añadir más runtimes reales y tests end-to-end del engine. |
| 1.2.10. AI Assist en Pipeline Builder | Parcial | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |

### 1.3 Code Repositories

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.3.1. Web-based IDE | Parcial | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.2. Control de versiones (Git) | Parcial | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.3. CI/CD integrado (ci/foundry-publish) | Parcial | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.4. Soporte TypeScript v2 | No | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.5. Soporte Python | Parcial | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.6. Plantillas de repositorio | Parcial | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |
| 1.3.7. Libraries side panel | No | `services/code-repo-service/src/main.rs`, `services/code-repo-service/src/domain/ci.rs`, `services/code-repo-service/src/domain/git/*`, `apps/web/src/routes/code-repos/+page.svelte` | Sustituir commits/CI sintéticos por Git y CI reales. |

### 1.4 Streaming

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.4.1. Stream creation con schema | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.2. Hot buffer + cold storage archiving | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.3. Fault tolerance con checkpoints | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.4. Job graph visualization | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.5. Streaming syncs desde fuentes externas | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.6. Transform de streams en Pipeline Builder | Parcial | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |
| 1.4.7. Push manual via API | No | `services/streaming-service/src/handlers/streams.rs`, `services/streaming-service/src/handlers/topologies.rs`, `services/streaming-service/src/domain/engine/processor.rs` | Pasar de simulación a streaming real con ingesta, checkpoints, archivado y push API. |

### 1.5 Data Lineage

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.5.1. Grafo interactivo de linaje | Parcial | `services/pipeline-service/src/domain/lineage/mod.rs`, `services/pipeline-service/src/domain/executor.rs`, `apps/web/src/routes/lineage/+page.svelte` | Ampliar el lineage a impacto operativo, workflows y herencia de seguridad. |
| 1.5.2. Data Lineage (datasets) | Cumple | `domain/executor.rs` registra lineage y `domain/lineage/mod.rs` lo expone por API. | Extender análisis de impacto/lineage operativo y su cobertura de tests. |
| 1.5.3. Workflow Lineage (GA) | No | `services/pipeline-service/src/domain/lineage/mod.rs`, `services/pipeline-service/src/domain/executor.rs`, `apps/web/src/routes/lineage/+page.svelte` | Ampliar el lineage a impacto operativo, workflows y herencia de seguridad. |
| 1.5.4. Upstream/downstream impact analysis | Parcial | `services/pipeline-service/src/domain/lineage/mod.rs`, `services/pipeline-service/src/domain/executor.rs`, `apps/web/src/routes/lineage/+page.svelte` | Ampliar el lineage a impacto operativo, workflows y herencia de seguridad. |
| 1.5.5. Builds desde Data Lineage | No | `services/pipeline-service/src/domain/lineage/mod.rs`, `services/pipeline-service/src/domain/executor.rs`, `apps/web/src/routes/lineage/+page.svelte` | Ampliar el lineage a impacto operativo, workflows y herencia de seguridad. |
| 1.5.6. Propagación de markings por linaje | No | `services/pipeline-service/src/domain/lineage/mod.rs`, `services/pipeline-service/src/domain/executor.rs`, `apps/web/src/routes/lineage/+page.svelte` | Ampliar el lineage a impacto operativo, workflows y herencia de seguridad. |

### 1.6 Datasets y Filesystem

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.6.1. Dataset con transacciones | Parcial | `services/dataset-service/src/handlers/upload.rs`, `branches.rs`, `quality.rs`, `preview.rs` | Completar views/filesystem/preview rico y transacciones más formales. |
| 1.6.2. Branching de datasets | Cumple | `branches.rs` ya soporta listar, crear, checkout, `merge` y `promote`; además el smoke valida creación de rama, checkout y promoción por API. | Endurecer concurrencia real, locks/transactions y conflictos de contenido más ricos. |
| 1.6.3. Dataset Views | No | `services/dataset-service/src/handlers/upload.rs`, `branches.rs`, `quality.rs`, `preview.rs` | Completar views/filesystem/preview rico y transacciones más formales. |
| 1.6.4. Dataset Preview | Cumple | `preview.rs` ahora devuelve muestra real, columnas inferidas, tipos, `total_rows`, errores/warnings y soporte para `branch`/`version`; el smoke valida preview real tras una sync. | Falta ampliar a views/materializaciones y formatos más complejos con profiling más profundo. |
| 1.6.5. Data Health checks | Cumple | `quality.rs` permite crear reglas y perfiles de calidad. | Ampliar reglas avanzadas, alertas continuas y test coverage. |
| 1.6.6. Filesystem navegable | Parcial | `export.rs` expone `GET /api/v1/datasets/:id/files` con listado navegable por prefijo y el smoke valida la presencia del objeto versionado en storage. | Falta crecer de listado mínimo por dataset a filesystem completo de folders/projects y operaciones de navegación más ricas. |
| 1.6.7. Linter / anti-patterns detector | No | `services/dataset-service/src/handlers/upload.rs`, `branches.rs`, `quality.rs`, `preview.rs` | Completar views/filesystem/preview rico y transacciones más formales. |

### 1.7 HyperAuto (SDDI)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.7.1. Generación automática de pipelines ERP | No | No encontré servicio dedicado; solo referencias en `README.md` y `ROADMAP.md` | Crear el flujo automático ERP -> pipelines -> ontology. |
| 1.7.2. Generación de Ontology desde ERP | No | No encontré servicio dedicado; solo referencias en `README.md` y `ROADMAP.md` | Crear el flujo automático ERP -> pipelines -> ontology. |

### 2.1 Ontology Manager — Tipos Semánticos

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.1.1. Object Types | Cumple | `object_type.rs` define el modelo y hay CRUD de object types en el servicio. | Completar capas semánticas avanzadas del modelo. |
| 2.1.2. Link Types | Cumple | `link_type.rs` y los handlers de links cubren tipos de relación e instancias. | Añadir traversal/query de mayor nivel y enforcement más rico. |
| 2.1.3. Properties con Value Types | Parcial | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.4. Interfaces / Polimorfismo | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.5. Shared Property Types | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.6. Time-dependent properties | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.7. Geo-point properties | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.8. Media references | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.9. Semantic search (unstructured data) | No | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |
| 2.1.10. Digital twin / espejo del mundo real | Parcial | `services/ontology-service/src/models/object_type.rs`, `property.rs`, `link_type.rs`, `interface.rs` | Extender el modelo semántico con interfaces, props compartidas y tipos avanzados. |

### 2.2 Action Types (Kinética del Ontology)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.2.1. Action Types (formularios punto a clic) | Parcial | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |
| 2.2.2. Function-backed Actions | Parcial | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |
| 2.2.3. Ontology Edits TypeScript API | No | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |
| 2.2.4. Batch apply actions | No | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |
| 2.2.5. Action validation | Cumple | `validate_action` valida configuración básica antes de persistir o ejecutar. | Conectar validación con permisos, auditoría y ejecución batch. |
| 2.2.6. Object Storage V2 (escritura inmediata) | Cumple | `apply_object_patch` aplica cambios y devuelve el objeto actualizado. | Añadir auditoría rica, sandbox y garantías transaccionales más fuertes. |
| 2.2.7. Webhook / External system actions | Cumple | `InvokeWebhook` e `InvokeFunction` usan `invoke_http_action` como integración básica. | Añadir retries, secretos seguros y observabilidad. |
| 2.2.8. Permisos granulares por Action | Parcial | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |
| 2.2.9. Scenario / what-if branching | No | `services/ontology-service/src/handlers/actions.rs` | Añadir enforcement, ejecución batch y sandboxing más completo. |

### 2.3 Functions (Lógica de Negocio)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.3.1. Functions en TypeScript v2 | No | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.2. Functions en Python | No | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.3. Object Set Queries | No | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.4. Link Traversals | Parcial | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.5. External Functions | Parcial | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.6. Platform SDK en Functions | No | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |
| 2.3.7. LLM en Functions (Language Model Service) | No | Solo hay invocación HTTP desde acciones de ontology; no existe runtime Functions separado | Crear una plataforma Functions real con runtimes TS/Python y SDK. |

### 2.4 Object Views, Explorer y Vertex

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.4.1. Object Views | Parcial | `apps/web/src/routes/ontology/+page.svelte`, `apps/web/src/lib/components/ontology/*`, `services/ontology-service` | Endurecer explorer/views y construir Vertex/simulación de verdad. |
| 2.4.2. Object Explorer | Parcial | `apps/web/src/routes/ontology/+page.svelte`, `apps/web/src/lib/components/ontology/*`, `services/ontology-service` | Endurecer explorer/views y construir Vertex/simulación de verdad. |
| 2.4.3. Vertex — System graphs | Parcial | `apps/web/src/routes/ontology/+page.svelte`, `apps/web/src/lib/components/ontology/*`, `services/ontology-service` | Endurecer explorer/views y construir Vertex/simulación de verdad. |
| 2.4.4. Vertex — Simulaciones end-to-end | No | `apps/web/src/routes/ontology/+page.svelte`, `apps/web/src/lib/components/ontology/*`, `services/ontology-service` | Endurecer explorer/views y construir Vertex/simulación de verdad. |

### 2.5 Foundry Rules y Machinery

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.5.1. Foundry Rules | No | No hay módulos dedicados de rules engine ni process mining | Implementar un rules/process engine dedicado. |
| 2.5.2. Machinery (Process Mining) | No | No hay módulos dedicados de rules engine ni process mining | Implementar un rules/process engine dedicado. |
| 2.5.3. Machinery widget de monitoreo | No | No hay módulos dedicados de rules engine ni process mining | Implementar un rules/process engine dedicado. |
| 2.5.4. Dynamic Scheduling | No | No hay módulos dedicados de rules engine ni process mining | Implementar un rules/process engine dedicado. |

### 2.6 Gotham Integration

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.6.1. Type mapping Foundry ↔ Gotham | No | No hay integración Gotham en el código actual | Diseñar interoperabilidad Gotham. |
| 2.6.2. Object Set Service | No | No hay integración Gotham en el código actual | Diseñar interoperabilidad Gotham. |

### 3.1 Model Assets y Modeling Objectives

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.1.1. Modeling Objectives | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.2. Model development in-platform | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.3. Import de modelos externos | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.4. Batch deployment | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.5. Live/online deployment | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.6. Model adapters | No | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.7. Versioning y reproducibilidad | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.8. Governance y audit trail de modelos | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.9. Staging y release to production | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.10. ML feedback loops | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.11. MLflow integration | No | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.12. Marketplace de modelos (DevOps) | Parcial | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |
| 3.1.13. Compute Modules (containers serverless) | No | `services/ml-service/src/main.rs`, handlers `models.rs`, `training.rs`, `deployments.rs`, `predictions.rs`, y `domain/training/mod.rs` | Conectar entrenamiento/inferencia reales, adapters, compute y empaquetado. |

### 3.2 AIP — Language Model Service

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.2.1. Interfaz unificada multi-LLM | Parcial | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.2. LLM en redes privadas | No | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.3. Multimodal / Vision-Language Models | No | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.4. LLM cost governance | Parcial | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.5. Evaluations (benchmarking LLMs) | Parcial | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |

### 3.3 AIP Agent Studio

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.3.1. AIP Agents con herramientas del Ontology | Parcial | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.2. Deploy interno (Workshop widget) | No | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.3. Deploy externo (OSDK / APIs) | Parcial | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.4. Agents como Functions (para Automate) | No | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.5. Tool use / Function calling | Parcial | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.6. AI FDE (natural language platform ops) | No | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |

### 3.4 AIP Logic y Automate

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.4.1. AIP Logic (LLM decision logic builder) | No | `services/workflow-service/src/main.rs`, `services/workflow-service/src/domain/executor.rs`, `services/notification-service/src/main.rs` | Completar lógica declarativa LLM y automatizaciones con acciones reales. |
| 3.4.2. Automate (event-driven triggers) | Parcial | `services/workflow-service/src/main.rs`, `services/workflow-service/src/domain/executor.rs`, `services/notification-service/src/main.rs` | Completar lógica declarativa LLM y automatizaciones con acciones reales. |
| 3.4.3. Automate — notificaciones | Cumple | `workflow-service` ejecuta pasos y `notification-service` expone envío, historial y websocket. | Añadir más canales, políticas y condiciones. |
| 3.4.4. Automate — submit Actions automáticas | Parcial | `services/workflow-service/src/main.rs`, `services/workflow-service/src/domain/executor.rs`, `services/notification-service/src/main.rs` | Completar lógica declarativa LLM y automatizaciones con acciones reales. |
| 3.4.5. Proposal-based pattern (human-in-the-loop) | Parcial | `services/workflow-service/src/main.rs`, `services/workflow-service/src/domain/executor.rs`, `services/notification-service/src/main.rs` | Completar lógica declarativa LLM y automatizaciones con acciones reales. |

### 4.1 Workshop (No-code / Low-code App Builder)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.1.1. Punto a clic (drag-and-drop) | Parcial | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |
| 4.1.2. Pro-code customizations | Parcial | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |
| 4.1.3. Widget library (continuamente actualizada) | Cumple | `models/widget_type.rs` define un catálogo de widgets. | Expandir la librería y eliminar componentes UI aún vacíos. |
| 4.1.4. AIP Interactive widget (agent embed) | No | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |
| 4.1.5. Scenario / what-if en Workshop | No | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |
| 4.1.6. Embedded Quiver dashboards | No | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |
| 4.1.7. Embedded Map | Cumple | Hay `map widget` y un `geospatial-service` separado. | Endurecer bindings geoespaciales, seguridad y casos avanzados. |
| 4.1.8. Consumer mode (usuarios externos B2C/B2B) | No | `services/app-builder-service/src/main.rs`, `handlers/apps.rs`, `models/widget_type.rs`, `apps/web/src/routes/apps/+page.svelte` | Endurecer el app builder con escenarios, consumer mode y más embeds. |

### 4.2 Slate (Pro-code App Builder)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.2.1. HTML / CSS / JavaScript custom apps | Parcial | `app-builder-service` y `web` permiten apps configurables, pero no hay un Slate separado | Formalizar un builder pro-code separado. |
| 4.2.2. Integración con Ontology layer | Parcial | `app-builder-service` y `web` permiten apps configurables, pero no hay un Slate separado | Formalizar un builder pro-code separado. |
| 4.2.3. Acceso directo a datasets | Parcial | `app-builder-service` y `web` permiten apps configurables, pero no hay un Slate separado | Formalizar un builder pro-code separado. |
| 4.2.4. Drag-and-drop + código | Parcial | `app-builder-service` y `web` permiten apps configurables, pero no hay un Slate separado | Formalizar un builder pro-code separado. |

### 4.3 OSDK React Applications

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.3.1. React UI con OSDK como backend | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.2. TypeScript bindings type-safe | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.3. Soporte NPM, Pip/Conda, Maven | Parcial | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.4. Developer Console | Parcial | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.5. VS Code Workspaces in-platform | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.6. Palantir extension for VS Code | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.7. Palantir MCP (Model Context Protocol) | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |
| 4.3.8. Ontology MCP | No | `apps/web/src/routes/developers/+page.svelte`, `tools/of-cli`, `apps/web/static/generated/*` | Publicar SDKs/OSDK/IDE/MCP reales. |

### 4.4 Workflow Building

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.4.1. Automate (ver sección AIP) | Parcial | `workflow-service` y la ruta web de workflows cubren una base parcial | Falta un solution designer/workspaces curados/use-case app. |
| 4.4.2. Solution Designer | No | `workflow-service` y la ruta web de workflows cubren una base parcial | Falta un solution designer/workspaces curados/use-case app. |
| 4.4.3. Carbon (curated workspaces) | No | `workflow-service` y la ruta web de workflows cubren una base parcial | Falta un solution designer/workspaces curados/use-case app. |
| 4.4.4. Use Case app | No | `workflow-service` y la ruta web de workflows cubren una base parcial | Falta un solution designer/workspaces curados/use-case app. |

### 5.1 Contour (Top-down Analysis)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.1.1. Exploración top-down visual | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.2. Transform boards (joins, filtros, agregaciones) | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.3. Display boards (gráficos, tablas) | Parcial | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.4. Paths y secuencias de análisis | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.5. Parámetros de análisis | Parcial | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.6. Dashboards con chart-to-chart filtering | Parcial | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.7. Export a dataset (materialización) | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.8. Export PDF | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |
| 5.1.9. Fullscreen presentation view | No | `apps/web/src/routes/dashboards/+page.svelte`, componentes dashboard y `report-service` | Construir una experiencia tipo Contour completa. |

### 5.2 Quiver (Time Series y Ontology Analytics)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.2.1. Análisis de time series | Parcial | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.2. Point-and-click sin código | Parcial | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.3. Navegación de relaciones entre object types | Parcial | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.4. Joins entre object sets | Parcial | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.5. Visual functions (bloques de lógica reutilizables) | No | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.6. Dashboards interactivos y paramétricos | Cumple | La ruta de dashboards y componentes como `DashboardGrid.svelte` y `FilterBar.svelte` cubren filtrado/composición básica. | Conectarlo a object sets/series temporales reales y a semántica Quiver completa. |
| 5.2.7. Embed en Workshop, Object Views, Carbon | Parcial | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |
| 5.2.8. Vega plots | No | dashboards + query service + ontology UI cubren parte; no hay un servicio Quiver dedicado | Construir Quiver real para series temporales, object sets y Vega. |

### 5.3 Map (Geospatial Analysis)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.3.1. Análisis geoespacial y temporal | Cumple | `geospatial-service` expone features/capas y hay una vista dedicada en web. | Completar GIS/raster ingestion y validarlo con datos productivos. |
| 5.3.2. Track analysis (movimiento histórico) | Parcial | `services/geospatial-service/src/main.rs`, `features.rs`, `tiles.rs`, y `apps/web/src/routes/geospatial/+page.svelte` | Endurecer GIS/raster/time-series. |
| 5.3.3. Raster imagery y capas GIS | Parcial | `services/geospatial-service/src/main.rs`, `features.rs`, `tiles.rs`, y `apps/web/src/routes/geospatial/+page.svelte` | Endurecer GIS/raster/time-series. |
| 5.3.4. Color/style por valor de dato | Cumple | `models/style.rs` y las respuestas de tiles cubren estilos básicos/vector tiles. | Añadir estilos más ricos y edición visual. |
| 5.3.5. Combinación con time series y sensores | Parcial | `services/geospatial-service/src/main.rs`, `features.rs`, `tiles.rs`, y `apps/web/src/routes/geospatial/+page.svelte` | Endurecer GIS/raster/time-series. |
| 5.3.6. Standalone o embebido en Workshop | Cumple | El servicio geoespacial es independiente y hay integración embebible vía widgets/mapas. | Terminar el acoplamiento Workshop/apps y permisos. |

### 5.4 Notepad (Collaborative Documents)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.4.1. Editor de texto enriquecido colaborativo | No | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |
| 5.4.2. Embed de widgets de Contour, Quiver, etc. | No | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |
| 5.4.3. Templates de Notepad | No | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |
| 5.4.4. Export / print de documentos | No | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |
| 5.4.5. Indexado por AIP Assist | Parcial | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |
| 5.4.6. Marketplace support | No | No existe servicio estilo Notepad; solo knowledge bases dentro de `ai-service` | Crear editor colaborativo de documentos e integraciones. |

### 5.5 Fusion (Spreadsheet Bidireccional)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.5.1. Spreadsheet editable sincronizado con dataset | No | `fusion-service` resuelve entidades, no una hoja bidireccional estilo Contour | Implementar spreadsheet bidireccional. |
| 5.5.2. Query de datos del Ontology en spreadsheet | No | `fusion-service` resuelve entidades, no una hoja bidireccional estilo Contour | Implementar spreadsheet bidireccional. |

### 5.6 Code Workspaces y Code Workbook

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.6.1. JupyterLab® integrado | Parcial | `services/notebook-service/src/main.rs`, `execute.rs`, `python.rs`, `sql.rs` | Completar workspaces de ciencia de datos e integración AI. |
| 5.6.2. RStudio® Workbench integrado | No | `services/notebook-service/src/main.rs`, `execute.rs`, `python.rs`, `sql.rs` | Completar workspaces de ciencia de datos e integración AI. |
| 5.6.3. LLMs en notebooks | No | `services/notebook-service/src/main.rs`, `execute.rs`, `python.rs`, `sql.rs` | Completar workspaces de ciencia de datos e integración AI. |
| 5.6.4. Code Workbook (legacy) | Parcial | `services/notebook-service/src/main.rs`, `execute.rs`, `python.rs`, `sql.rs` | Completar workspaces de ciencia de datos e integración AI. |

### 5.7 Integraciones BI Externas

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.7.1. Conector Tableau | No | `query-service`, `gateway` y clientes REST en `apps/web/src/lib/api/*` | Añadir conectores BI y SDKs oficiales. |
| 5.7.2. Conector Power BI | No | `query-service`, `gateway` y clientes REST en `apps/web/src/lib/api/*` | Añadir conectores BI y SDKs oficiales. |
| 5.7.3. ODBC/JDBC drivers | No | `query-service`, `gateway` y clientes REST en `apps/web/src/lib/api/*` | Añadir conectores BI y SDKs oficiales. |
| 5.7.4. Python SDK | No | `query-service`, `gateway` y clientes REST en `apps/web/src/lib/api/*` | Añadir conectores BI y SDKs oficiales. |
| 5.7.5. REST API (Foundry API) | Cumple | La API REST es amplia vía gateway y microservicios, y el frontend la consume desde `lib/api/*`. | Versionar mejor la API y cubrir superficies admin/filesystem que faltan. |

### 6.1 Foundry DevOps

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 6.1.1. Packaging de productos | Parcial | `marketplace-service`, versionado en app-builder y metadata en code-repo | Convertir versionado/instalación en packaging y fleet management completo. |
| 6.1.2. Release channels / versioning | Parcial | `marketplace-service`, versionado en app-builder y metadata en code-repo | Convertir versionado/instalación en packaging y fleet management completo. |
| 6.1.3. Gestión de instalaciones (fleet) | Parcial | `marketplace-service`, versionado en app-builder y metadata en code-repo | Convertir versionado/instalación en packaging y fleet management completo. |
| 6.1.4. Maintenance windows | No | `marketplace-service`, versionado en app-builder y metadata en code-repo | Convertir versionado/instalación en packaging y fleet management completo. |
| 6.1.5. Foundry Branching (beta) | No | `marketplace-service`, versionado en app-builder y metadata en code-repo | Convertir versionado/instalación en packaging y fleet management completo. |

### 6.2 Marketplace

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 6.2.1. Storefront de productos | Cumple | Marketplace cubre overview/browse/publish/reviews/installs y tiene ruta web dedicada. | Conectar la instalación con despliegue y activación reales. |
| 6.2.2. Guided installation | Parcial | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |
| 6.2.3. Recommended products | No | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |
| 6.2.4. Starter packs / ejemplos | Parcial | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |
| 6.2.5. Instalaciones multi-space | Parcial | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |

### 7.1 Control de Acceso

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.1.1. Role-based access control (RBAC) | Cumple | RBAC real en `rbac.rs`, `role_mgmt.rs` y `group_mgmt.rs`. | Complementarlo con mandatory controls, markings y aislamiento multi-org. |
| 7.1.2. Markings (mandatory access controls) | Parcial | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.3. Propagación de markings por linaje | No | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.4. Classification-based access controls (CBAC) | No | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.5. Scoped sessions | No | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.6. Organization-level isolation | Parcial | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.7. Guest access cross-organization | No | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.8. Restricted views | Parcial | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |
| 7.1.9. Consumer mode (external users) | No | `services/auth-service/src/domain/rbac.rs`, `role_mgmt.rs`, `group_mgmt.rs`, `policy_mgmt.rs` | Añadir markings, controles obligatorios y aislamiento multi-org. |

### 7.2 Autenticación y Cifrado

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.2.1. Single Sign-On (SSO / SAML 2.0) | Parcial | `services/auth-service/src/handlers/sso.rs`, `mfa.rs`, `oauth.rs`, `jwt.rs` | Completar SAML end-to-end, cifrado platform-wide y zero-trust ops. |
| 7.2.2. Multi-factor authentication (MFA) | Cumple | MFA implementado en `mfa.rs`. | Añadir más factores, políticas basadas en riesgo y tests. |
| 7.2.3. OAuth 2.0 (client credentials, auth code) | Parcial | `services/auth-service/src/handlers/sso.rs`, `mfa.rs`, `oauth.rs`, `jwt.rs` | Completar SAML end-to-end, cifrado platform-wide y zero-trust ops. |
| 7.2.4. Encryption in transit y at rest | Parcial | `services/auth-service/src/handlers/sso.rs`, `mfa.rs`, `oauth.rs`, `jwt.rs` | Completar SAML end-to-end, cifrado platform-wide y zero-trust ops. |
| 7.2.5. Zero-trust security architecture | No | `services/auth-service/src/handlers/sso.rs`, `mfa.rs`, `oauth.rs`, `jwt.rs` | Completar SAML end-to-end, cifrado platform-wide y zero-trust ops. |

### 7.3 Governance y Privacidad

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.3.1. Audit logging completo | Cumple | `audit-service` y el middleware audit del gateway registran eventos. | Expandir cobertura cross-service y retención/integridad global. |
| 7.3.2. Approvals (change management) | Parcial | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.3. Checkpoint (justification prompts) | No | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.4. Cipher (cryptographic operations) | No | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.5. Sensitive Data Scanner (SDS) | No | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.6. Data Lifetime / retention policies | Parcial | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.7. Compliances: HIPAA, GDPR, ITAR | Parcial | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |
| 7.3.8. Project templates para governance estándar | No | `services/audit-service`, middleware audit del gateway y approvals en workflows | Cerrar huecos de checkpoints, cipher/SDS, retención y compliance. |

### 8.1 Control Panel y Administración

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.1.1. Control Panel centralizado | Parcial | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |
| 8.1.2. Enrollment vs Organization permissions | Parcial | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |
| 8.1.3. Resource Management | No | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |
| 8.1.4. Upgrade Assistant | No | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |
| 8.1.5. Identity provider mapping (SAML org assignment) | No | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |
| 8.1.6. Custom platform branding | Parcial | ruta de settings y bases parciales en auth/audit/developers | Construir control panel, gestión de recursos y flujos de upgrade reales. |

### 8.2 Enablement y Documentación

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.2.1. AIP Assist (platform-wide chatbot) | Parcial | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |
| 8.2.2. Custom documentation in-platform | Parcial | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |
| 8.2.3. Walkthroughs (tutoriales interactivos) | No | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |

### 8.3 Multi-Organization Ecosystems

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.3.1. Private + shared spaces | Parcial | `services/nexus-service/src/main.rs`, `peers.rs`, `shares.rs`, `contracts.rs` | Endurecer el ecosistema multi-org más allá de Nexus base. |
| 8.3.2. Data sharing controlado entre orgs | Parcial | `services/nexus-service/src/main.rs`, `peers.rs`, `shares.rs`, `contracts.rs` | Endurecer el ecosistema multi-org más allá de Nexus base. |
| 8.3.3. Host organization + partners | Parcial | `services/nexus-service/src/main.rs`, `peers.rs`, `shares.rs`, `contracts.rs` | Endurecer el ecosistema multi-org más allá de Nexus base. |

### 9.1 APIs REST

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 9.1.1. Foundry Platform API (v1 y v2) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.2. Datasets API | Parcial | `dataset-service` ya cubre CRUD, upload, preview rico, schema, versions, branching (`merge/promote`) y filesystem mínimo `/files`, todo accesible vía gateway y validado por smoke. | Homogeneizar contratos, añadir dataset views/v2 y reforzar más tests de integración específicos. |
| 9.1.3. Ontologies API (Objects, Links, Actions) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.4. Orchestration API (Builds, Jobs, Schedules) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.5. Streams API (real-time, second latency) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.6. Connectivity API (external systems) | Parcial | `connections.rs` y `sync_ops.rs` ya exponen creación, `test_connection` real, sync jobs persistentes, retries y estados; los conectores PostgreSQL/CSV/JSON/REST API/Salesforce están implementados en los caminos principales. | Queda ampliar cobertura enterprise, incrementalidad, descubrimiento de fuentes y contratos más homogéneos entre conectores. |
| 9.1.7. Filesystem API (folders, projects) | No | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.8. SQL Queries API | Cumple | `query-service` expone `/execute` y `/explain`. | Añadir federación, drivers externos y observabilidad. |
| 9.1.9. Admin API (Users, Groups, Markings, Orgs) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |

### 9.2 SDKs

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 9.2.1. Foundry Platform SDK (Python) | No | `tools/of-cli/src/openapi.rs`, `apps/web/static/generated/openapi/openfoundry.json` | Publicar specs versionadas y derivar SDKs oficiales. |
| 9.2.2. OSDK (TypeScript/NPM) | No | `tools/of-cli/src/openapi.rs`, `apps/web/static/generated/openapi/openfoundry.json` | Publicar specs versionadas y derivar SDKs oficiales. |
| 9.2.3. OSDK (Python/Pip) | No | `tools/of-cli/src/openapi.rs`, `apps/web/static/generated/openapi/openfoundry.json` | Publicar specs versionadas y derivar SDKs oficiales. |
| 9.2.4. OSDK (Java/Maven) | No | `tools/of-cli/src/openapi.rs`, `apps/web/static/generated/openapi/openfoundry.json` | Publicar specs versionadas y derivar SDKs oficiales. |
| 9.2.5. OpenAPI spec (any language) | Cumple | `openapi.rs` genera la spec y el JSON generado está en `static/generated/openapi/openfoundry.json`. | Publicar/versionar oficialmente la spec y derivar SDKs. |

### 10. INFRAESTRUCTURA Y DEPLOYMENT

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 10.1. SaaS multi-cloud (AWS, Azure, GCP, OCI) | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |
| 10.2. On-premises / air-gapped deployment | Parcial | Hay auto-hosting básico con `docker-compose` y Helm, pero no un story enterprise completo. | Completar on-prem/air-gapped, conectores privados y operación offline. |
| 10.3. Apollo (CI/CD autónomo) | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |
| 10.4. Kubernetes autoscaling build system | Parcial | Existen manifests de HPA y KEDA `ScaledObject`. | Hace falta autoscaling validado en build/run, no solo manifests. |
| 10.5. High availability / autoscaling compute mesh | Parcial | Helm + HPA/KEDA dan una base, pero no prueban HA/distribución completa. | Hace falta demostrar alta disponibilidad y reparto real de carga. |
| 10.6. Geo-restricted enrollments | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |

## Notas Finales

- Lo más sólido hoy: auth base (RBAC/MFA/SSO), CRUD y versionado básico de datasets, ontology base, workflows/notificaciones, query engine y notebooks.
- Lo más débil: conectividad enterprise real, streaming real, Git/CI real, ML/AI de producción y cobertura de tests.
- También pesan en la evaluación los placeholders y archivos vacíos detectados durante la revisión, porque indican que varias capacidades del checklist todavía no están cerradas.
