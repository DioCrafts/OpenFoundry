# Estado del Checklist de Foundry vs OpenFoundry

Este documento traduce el `checklist.md` original a una matriz de estado basada en la evidencia actual del código de OpenFoundry. El archivo original se mantiene como referencia; aquí sintetizo cumplimiento, evidencia visible y gap concreto.

## Método

- `Cumple`: existe implementación funcional clara y verificable en el repo.
- `Parcial`: hay base técnica, endpoints o UI, pero la capacidad está incompleta, simulada o limitada.
- `No`: no encontré implementación real, o lo existente es placeholder o vacío.
- La evaluación está hecha sobre el código y la estructura del repositorio, no sobre despliegues externos ni integraciones productivas.
- Verificaciones base posteriores a P0: `cargo check --workspace` OK, `cargo test --workspace` OK como baseline del monorepo, `pnpm --filter @open-foundry/web check` OK y la separación `unit/E2E` del frontend quedó integrada en CI.
- Verificaciones P1 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p data-connector -p dataset-service -p of-cli` OK, `cargo test -p data-connector -p dataset-service -p of-cli` OK y `target/debug/of smoke run --scenario smoke/scenarios/p0-critical-path.json --output smoke/results/p0-critical-path.json` OK contra servicios vivos y Postgres local.
- Verificaciones P2 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p gateway -p pipeline-service -p streaming-service -p of-cli` OK, `cargo test -p gateway -p pipeline-service -p streaming-service -p of-cli` OK y `target/debug/of smoke run --scenario smoke/scenarios/p2-runtime-critical-path.json --output smoke/results/p2-runtime-critical-path.json` OK contra stack vivo y Postgres Docker. El smoke verificó batch SQL real con materialización a dataset, re-ejecución incremental con `skipped`, lineage expuesto por API, push streaming, window aggregation, checkpointing, sink a dataset y rerun sin nuevos eventos.
- Verificaciones P3 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p ontology-service -p of-cli` OK, `cargo test -p ontology-service -p of-cli` OK y `target/debug/of smoke run --scenario smoke/scenarios/p3-semantic-governance-critical-path.json --output smoke/results/p3-semantic-governance-critical-path.json` OK contra gateway + ontology-service + audit-service vivos y Postgres Docker. El smoke verificó interfaces/shared properties/time-dependent, `execute-batch`, runtime Python real, `403` cross-org, checkpoint por justificación, object queries/traversals y auditoría cross-service.
- Verificaciones P4 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p code-repo-service -p marketplace-service -p of-cli` OK, `cargo test -p code-repo-service -p marketplace-service -p of-cli` OK, `cargo run -p of-cli -- docs validate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk` OK, `pnpm --dir apps/web exec tsc -p ../../sdks/typescript/openfoundry-sdk/tsconfig.json --noEmit` OK y `target/debug/of smoke run --scenario smoke/scenarios/p4-developer-platform-critical-path.json --output smoke/results/p4-developer-platform-critical-path.json` OK contra stack vivo y Postgres Docker. El smoke verificó repos Git-backed reales con rama/commit/diff/CI y una instalación de Marketplace que activa y publica una app real.
- Verificaciones P5 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p of-cli -p ai-service -p ml-service` OK, `cargo test -p ai-service -p ml-service -p of-cli` OK y `target/debug/of smoke run --scenario smoke/scenarios/p5-ai-ml-critical-path.json --output smoke/results/p5-ai-ml-critical-path.json` OK contra gateway + ai-service + ml-service + mock provider vivos y Postgres Docker. El smoke verificó provider HTTP real tipo OpenAI-compatible, embeddings provider-backed, búsqueda RAG real, `chat completion` con citas, agent execution con tool HTTP real sobre la API ML, entrenamiento tabular real, registro de versión, deployment y predicción online real.
- Verificaciones P6 posteriores: `env RUSTFLAGS='-D warnings' cargo check -p auth-service -p gateway -p report-service -p nexus-service -p of-cli` OK, `cargo test -p nexus-service -p report-service -p of-cli` OK, `pnpm --filter @open-foundry/web check` OK, `pnpm lint` OK con warnings conocidos, y `target/debug/of smoke run --scenario smoke/scenarios/p6-analytics-enterprise-critical-path.json --output smoke/results/p6-analytics-enterprise-critical-path.json` OK contra gateway + auth-service + data-connector + dataset-service + report-service + geospatial-service + nexus-service vivos y Postgres Docker. El smoke verificó analytics real con dataset sincronizado desde fixture JSON, query geoespacial real, report generation live, `GET/PUT /api/v1/control-panel`, share rechazado con consumer pendiente, share endurecido y bloqueo de `DELETE` en federated query. La validación Helm quedó automatizada en `.github/workflows/helm-check.yml`; no pude ejecutar `helm lint/template` localmente porque `helm` no está instalado en esta máquina.
- Verificaciones post-P6 de ontology/functions: `env RUSTFLAGS='-D warnings' cargo check -p ontology-service` OK, `cargo test -p ontology-service` OK y `pnpm --filter @open-foundry/web check` OK. La validación cubrió `geo_point`, `media_reference`, búsqueda `fulltext + semantic`, graph/vertex backend real, runtime inline `typescript`, SDK de plataforma dentro de Functions y helper `llm.complete(...)` para funciones TypeScript.
- Verificaciones post-P6 de ontology/functions platform hardening: `env RUSTFLAGS='-D warnings' cargo check -p ontology-service` OK, `cargo test -p ontology-service` OK, `pnpm --filter @open-foundry/web check` OK y `pnpm --filter @open-foundry/web build` OK. La validación cubrió `ontology_function_packages` reutilizables con capabilities/policies reales, integración de `function_package_id` dentro de actions, `ontology_rules` con evaluación/aplicación real, `machinery insights`, `object views` enriquecidas y simulación end-to-end por objeto con preview de actions, impacto de rules y snapshot de graph.
- Verificaciones post-P6 de conectividad enterprise: `env RUSTFLAGS='-D warnings' cargo check -p data-connector` OK y `cargo test -p data-connector` OK. La validación cubrió registro y heartbeat de `connector_agents`, discover + `auto`/`bulk registration`, `zero_copy` query real, update detection por `source_signature`, políticas de egress y conectores HTTP reales nuevos para `sap` e `iot`.
- Verificaciones post-P6 de datasets/views/filesystem: `env RUSTFLAGS='-D warnings' cargo check -p dataset-service` OK, `cargo test -p dataset-service` OK y `pnpm --filter @open-foundry/web check` OK. La validación cubrió `dataset_views` materializadas con refresh/preview real, journal `dataset_transactions`, uploads con lock transaccional y actualización de `row_count/schema`, además de un filesystem lógico navegable con `current`, `versions`, `branches` y `views`.
- Verificaciones post-P6 de lineage operativo: `env RUSTFLAGS='-D warnings' cargo check -p dataset-service -p workflow-service -p pipeline-service` OK, `cargo test -p dataset-service -p workflow-service -p pipeline-service` OK y `pnpm --filter @open-foundry/web check` OK. La validación cubrió grafo generalizado `dataset/pipeline/workflow`, sync interno de workflow lineage, análisis upstream/downstream, builds downstream desde datasets, propagación de `markings` en nodos/aristas y filtrado por `classification_clearance` en la API/UI de lineage.
- Verificaciones post-P6 del pipeline builder híbrido: `env RUSTFLAGS='-D warnings' cargo check -p pipeline-service` OK, `cargo test -p pipeline-service` OK y `pnpm --filter @open-foundry/web check` OK. La validación cubrió nuevos nodos `spark` y `external` con contrato real de compute remoto HTTP en `pipeline-service`, tests unitarios del contrato request/response, asistente AI operativo dentro de `apps/web/src/routes/pipelines/+page.svelte` y una vista unificada batch/streaming con runtime de topologías en la misma pantalla.
- Verificaciones post-P6 de seguridad transversal: `env RUSTFLAGS='-D warnings' cargo check -p auth-service -p gateway -p audit-service` OK, `cargo test -p auth-service -p gateway -p audit-service` OK y `pnpm --filter @open-foundry/web check` OK. La validación cubrió sesiones `scoped/guest` persistentes, enforcement zero-trust de `method/path` en gateway, callback `OIDC/SAML` dual con metadata/mapping, operaciones `cipher` reales (`hash/sign/verify`), `Sensitive Data Scanner` con redacción y templates de governance/compliance aplicables desde `audit-service`.
- Verificaciones post-P6 de APIs y SDKs multi-lenguaje: `env RUSTFLAGS='-D warnings' cargo check -p of-cli -p auth-service -p dataset-service -p gateway` OK, `cargo test -p of-cli -p auth-service -p dataset-service -p gateway` OK, `pnpm --filter @open-foundry/web check` OK, `cargo run -p of-cli -- docs validate-openapi --input apps/web/static/generated/openapi/openfoundry.json` OK, `cargo run -p of-cli -- docs validate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk` OK, `cargo run -p of-cli -- docs validate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk` OK, `cargo run -p of-cli -- docs validate-sdk-java --input apps/web/static/generated/openapi/openfoundry.json --output sdks/java/openfoundry-sdk` OK, `pnpm --dir apps/web exec tsc -p ../../sdks/typescript/openfoundry-sdk/tsconfig.json --noEmit` OK y `python3 -m compileall sdks/python/openfoundry-sdk` OK. La validación cubrió overlays OpenAPI para `/api/v2/admin/...` y `/api/v2/filesystem/datasets/{dataset_id}`, aliases reales publicados por `auth-service`/`dataset-service` y SDKs oficiales `TypeScript`, `Python` y `Java` generados desde la misma spec con drift check en CI. La compilación Java local queda delegada a `.github/workflows/proto-check.yml` porque esta máquina no tiene `java/javac`.
- Verificaciones post-P6 de Workshop/Slate/developer experience: `env RUSTFLAGS='-D warnings' cargo check -p app-builder-service -p code-repo-service -p of-cli` OK, `cargo test -p app-builder-service -p code-repo-service -p of-cli` OK, `cargo test -p of-cli` OK, `pnpm --filter @open-foundry/web check` OK, `pnpm --filter @open-foundry/web build` OK, `cargo run -p of-cli -- docs generate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk` OK, `cargo run -p of-cli -- docs validate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk` OK y `pnpm --dir apps/web exec tsc -p ../../sdks/typescript/openfoundry-sdk/tsconfig.json --noEmit` OK. La validación cubrió `consumer mode` configurable en Workshop, widget de `scenario/what-if`, widget de agente con ejecución real sobre `ai-service`, export/import Slate real vía `app-builder-service`, workspace/editor in-platform con round-trip manifest, `Quiver` embebido en el workspace de Slate, scaffolds más profundos `TypeScript/React` y `Python` en `code-repo-service`, y helpers React generados en `@open-foundry/sdk/react` con `Provider` + hooks de contexto.
- Verificaciones post-P6 de control panel y multi-org profundo: `cargo test -p auth-service -p nexus-service` OK, `env RUSTFLAGS='-D warnings' cargo check -p auth-service -p nexus-service` OK, `pnpm --filter @open-foundry/web check` OK y `pnpm lint` OK con warnings heredados. La validación cubrió `identity_provider_mappings` persistentes en `control_panel_settings`, provisioning SSO real con `organization/workspace/classification_clearance/default_roles`, políticas de recursos que alimentan `tenant_tier` y `tenant_quotas` para enforcement posterior en gateway, `GET /api/v1/control-panel/upgrade-readiness`, `nexus_spaces` reales con miembros y governance tags, asociación opcional de shares a espacios y lifecycle/admin contacts de peers host/partner en `nexus-service`.
- Verificaciones post-P6 de analytics tipo producto: `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo check -p notebook-service -p gateway` OK, `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 cargo test -p notebook-service -p gateway` OK, `pnpm --filter @open-foundry/web check` OK, `pnpm lint` OK con warnings heredados y `pnpm --filter @open-foundry/web build` OK. La validación cubrió `Contour` real sobre datasets con joins/filtros/agregaciones/drill path/export a dataset/fullscreen, `Quiver` real sobre `ontology-service` con time series/object-set joins/graph navigation/visual functions, `Notepad` persistido en `notebook-service` con presencia/export/indexado AIP y `Fusion Spreadsheet` bidireccional sobre datasets y objetos de ontology.

## Resumen por Dominio

| Dominio | Estado global | Cumple | Parcial | No |
| --- | --- | ---: | ---: | ---: |
| 1. CONECTIVIDAD E INTEGRACIÓN DE DATOS | Parcial alto | 34 | 10 | 7 |
| 2. ONTOLOGY (CAPA SEMÁNTICA Y OPERACIONAL) | Parcial alto | 32 | 2 | 2 |
| 3. CONECTIVIDAD Y DESARROLLO DE MODELOS (MLOps) | Parcial medio | 7 | 13 | 9 |
| 4. DESARROLLO DE CASOS DE USO (APLICACIONES) | Parcial alto | 12 | 6 | 6 |
| 5. ANALYTICS | Parcial alto | 22 | 10 | 8 |
| 6. PRODUCT DELIVERY (DevOps y Marketplace) | Parcial bajo | 1 | 6 | 3 |
| 7. SEGURIDAD Y GOBERNANZA | Parcial alto | 6 | 16 | 0 |
| 8. MANAGEMENT Y ENABLEMENT | Parcial alto | 9 | 2 | 1 |
| 9. DEVELOPER TOOLCHAIN (APIs y SDKs) | Parcial alto | 6 | 8 | 0 |
| 10. INFRAESTRUCTURA Y DEPLOYMENT | Parcial medio | 0 | 3 | 3 |

## Matriz Detallada

### 1.1 Data Connection (Conectores y Fuentes)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.1.1. Conectores nativos (200+) | No | `data-connector` ya cubre `postgresql`, `csv`, `json`, `rest_api`, `salesforce`, `sap` e `iot`, pero la amplitud sigue muy lejos del catálogo 200+ y los módulos `snowflake.rs`, `bigquery.rs`, `kafka.rs`, `s3.rs`, `oracle.rs` o `mssql.rs` siguen sin implementación real. | Ampliar la cobertura de conectores enterprise reales y el catálogo empaquetado. |
| 1.1.2. Conector SAP (Foundry Connector 2.0) | Cumple | `connectors/sap.rs` implementa `test_connection`, discover de entity sets OData, sync real y `zero_copy`; `handlers/connections.rs` ya enruta `sap` y `sync_engine.rs` lo ejecuta con soporte de agent proxy. | Falta endurecer auth/secrets SAP más allá del bearer básico y ampliar variantes RFC/BAPI no OData. |
| 1.1.3. Streaming data sources | Parcial | `connectors/iot.rs` añade feeds HTTP/IIoT reales y discover/zero-copy, mientras `streaming-service` ya cubre push streaming y sinks reales. | Falta completar conectores persistentes tipo Kafka/Kinesis/NATS y una operación streaming-source más amplia desde `data-connector`. |
| 1.1.4. REST API source | Cumple | `rest_api.rs` hace `test_connection` HTTP real, sincroniza payloads JSON reales y el smoke `create_connection -> test_connection -> sync -> preview` quedó verde contra `/health` del gateway. | Queda endurecer incrementalidad, paginación avanzada, secretos y políticas de egress. |
| 1.1.5. Generic source / custom connectors | Parcial | `connectors/rest_api.rs` ya soporta catálogos `resources/catalog_path`, `bulk registration`, `zero_copy` y routing vía `agent_url/agent_id`, lo que deja una base usable de custom HTTP connectors. | Falta empaquetado formal de connectors custom, secrets versionados y SDK/plugin model más fuerte. |
| 1.1.6. IoT / IIoT data sources | Cumple | `connectors/iot.rs` implementa `test_connection`, discover de feeds, sync real y `zero_copy` sobre feeds JSON HTTP/IIoT; `connections.rs` ya enruta `iot` y `sync_engine.rs` lo procesa con update detection. | Queda ampliar más protocolos IoT industriales (MQTT/OPC-UA/Kafka industrial) y operación offline/store-and-forward. |
| 1.1.7. On-premises agent | Parcial | `connector_agents` quedó persistido en la migración `20260425153000_enterprise_connectivity.sql`; `handlers/agents.rs` añade `register/list/heartbeat`, `agent_registry.rs` resuelve `agent_id`, y `http_runtime.rs` soporta proxy HTTP vía `/api/v1/connector-agent/http`. | Falta empaquetar un binary/installer de agent, mTLS/rotación de credenciales y operación completamente autónoma fuera del servicio central. |
| 1.1.8. Virtual tables (zero-copy) | Cumple | `handlers/registrations.rs` expone `POST /api/v1/connections/{id}/virtual-tables/query`, `registration_mode = "zero_copy"` y `query_virtual_table` real en `postgres`, `rest_api`, `salesforce`, `sap`, `iot`, `csv` y `json`. | Falta crecer hacia federación más profunda, pushdown avanzado y permisos/quotas por virtual table. |
| 1.1.9. Auto-registration de tablas | Cumple | `handlers/registrations.rs` expone `POST /api/v1/connections/{id}/discover` y `POST /api/v1/connections/{id}/registrations/auto`, persistiendo `connection_registrations` reales para tablas/feeds/entity sets descubiertos. | Falta enlazar el auto-register con creación automática de datasets/views y políticas más ricas por selector. |
| 1.1.10. Bulk registration | Cumple | `POST /api/v1/connections/{id}/registrations/bulk` registra múltiples selectors de una vez, con `registration_mode`, `target_dataset_id` y metadata por item, persistidos en `connection_registrations`. | Queda endurecer validación masiva transaccional y UX/CLI específicos de operación bulk. |
| 1.1.11. Update detection / versioning | Cumple | `sync_engine.rs` calcula y persiste `source_signature`, compara contra `connection_registrations.last_source_signature`, marca sync `skipped` cuando no hay cambios y mantiene `last_dataset_version`. | Falta añadir estrategias incrementales más inteligentes por fuente en lugar de basarse principalmente en fingerprint/hash del payload. |
| 1.1.12. Export / egress controls | Cumple | `domain/egress.rs` aplica allow/block lists, control de redes privadas e `allow_insecure_http`; `http_runtime.rs` lo hace cumplir para conectores HTTP y también antes del agent proxy. | Queda exponer administración/UI/policies globales más finas y auditar egress a nivel organizacional. |

### 1.2 Pipeline Builder

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.2.1. Interfaz visual drag-and-drop | Parcial | `apps/web/src/routes/pipelines/+page.svelte` ya concentra un builder híbrido con nodos batch reales, sidecar AI y companion streaming runtime, mientras `services/pipeline-service/src/domain/engine/mod.rs` y `runtime.rs` ejecutan nodos reales incluyendo compute remoto. | Falta una edición DAG más gráfica/drag-and-drop de verdad, mejores conectores visuales y una experiencia de canvas más rica. |
| 1.2.2. Transforms batch (PySpark) | Parcial | `services/pipeline-service/src/domain/engine/mod.rs` ya acepta nodos `spark`/`pyspark`, `runtime.rs` orquesta jobs Spark remotos por HTTP con inputs preparados y materialización opcional a dataset, y la UI expone esos nodos en `apps/web/src/routes/pipelines/+page.svelte`. | Falta endurecer perfiles de cluster, runners Spark nativos o más empaquetados y ampliar cobertura e2e más allá del contrato remoto base. |
| 1.2.3. Transforms ligeros (Polars/Python) | Cumple | `services/pipeline-service/src/domain/engine/runtime.rs` ejecuta transforms `python` con datasets reales como entrada/salida, serializa filas al intérprete y materializa resultados; `smoke/scenarios/p2-runtime-critical-path.json` valida el runtime batch real por API. | Falta añadir backend Polars nativo y más cobertura de tests de transformación. |
| 1.2.4. LLM-powered transforms | No | `services/pipeline-service/src/main.rs`, `services/pipeline-service/src/domain/engine/mod.rs`, `apps/web/src/routes/pipelines/+page.svelte` | Completar runtimes reales (Spark/Polars/compute externo), incrementalidad y builder UX. |
| 1.2.5. External compute orchestration | Cumple | `services/pipeline-service/src/domain/engine/runtime.rs` ya orquesta nodos `external` contra endpoints HTTP remotos con `config`, `inputs`, `result_rows` y auth opcional `service_jwt`; `mod.rs` los ejecuta como runtime de primera clase y `apps/web/src/routes/pipelines/+page.svelte` ya permite authoring de estos nodos. | Queda endurecer retries/policies por target, catálogos de runners y observabilidad más profunda por worker externo. |
| 1.2.6. Streaming pipelines | Parcial | `services/streaming-service/src/handlers/topologies.rs` y `domain/engine/processor.rs` ya ejecutan topologías reales con windows, sinks y checkpoints, y el builder ahora muestra un companion streaming runtime en la misma página de pipelines. | Falta una edición streaming más profunda dentro del mismo canvas y una unificación todavía mayor del authoring batch/streaming. |
| 1.2.7. Scheduling e integración de builds | Cumple | `services/pipeline-service/src/domain/executor.rs` persiste `next_run_at`, ejecuta pipelines programados y registra metadata real de build en `execution_context.build`; el smoke P2 valida estados `completed` e incrementalidad. | Queda ampliar la integración con CI externa y dashboards operativos más ricos. |
| 1.2.8. Incremental transforms | Cumple | `services/pipeline-service/src/domain/engine/mod.rs` calcula fingerprints por nodo/datasets y marca `skipped` cuando no cambian inputs; el smoke P2 valida una segunda ejecución con `prior_completed_node_count = 1` y `skipped = true`. | Falta profundizar en invalidación selectiva multi-nodo y estrategias incrementales por conector/formato. |
| 1.2.9. Multi-language pipelines | Cumple | `services/pipeline-service/src/domain/engine/runtime.rs` ejecuta nodos `sql`, `python`, `wasm` y `passthrough` con datasets/materialización reales. | Añadir más runtimes reales y tests end-to-end adicionales del engine. |
| 1.2.10. AI Assist en Pipeline Builder | Cumple | `apps/web/src/routes/pipelines/+page.svelte` integra `askCopilot(...)` dentro del builder, usa el contexto real de datasets enlazados, muestra `suggested_sql`/`pipeline_suggestions` y permite insertar nodos SQL/Spark/External directamente desde la respuesta AI. | Falta endurecer repair/debug automático multi-node y prompts más especializados por dominio/runtime. |

### 1.3 Code Repositories

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.3.1. Web-based IDE | Cumple | `services/code-repo-service/src/domain/git/runtime.rs` ya mantiene repos Git reales con archivos editables/listables y `apps/web/src/routes/code-repos/+page.svelte` expone la superficie web de repos, ramas, commits, archivos y diff. | Falta endurecer la UX de edición/review, paneles laterales y workspaces más especializados por lenguaje. |
| 1.3.2. Control de versiones (Git) | Cumple | `runtime.rs`, `handlers/branches.rs`, `handlers/commits.rs` y `handlers/diff.rs` usan `git init`, `git branch`, `git commit`, `git log` y `git diff` reales; el smoke P4 validó rama, commit con archivos y diff por API. | Falta endurecer merge requests/reviews y operaciones más avanzadas de Git. |
| 1.3.3. CI/CD integrado (ci/foundry-publish) | Cumple | `runtime.rs` ejecuta checks reales para el repo (`cargo check/test --offline` cuando aplica) y `handlers/commits.rs` persiste `ci_runs`; el smoke P4 validó un run real asociado al commit. | Falta crecer hacia pipelines de publish/release, runners externos y matrices más ricas por stack. |
| 1.3.4. Soporte TypeScript v2 | No | Aunque el servicio de repos ya es Git real, no encontré workspaces TypeScript v2 ni toolchains específicos desde `apps/web/src/routes/code-repos/+page.svelte` o `services/code-repo-service/src/domain/git/runtime.rs`. | Faltan scaffolds, CI, ergonomía y tooling específicos para TypeScript v2 dentro del IDE/repos. |
| 1.3.5. Soporte Python | Parcial | El flujo de repos/ramas/commits ya es real y permite almacenar código Python, pero no hay un workspace/runtime Python equivalente al soporte Rust actual en `runtime.rs`. | Faltan plantillas, checks/pipelines y tooling Python de primera clase dentro del servicio de repos. |
| 1.3.6. Plantillas de repositorio | Cumple | `initialize_repository` en `services/code-repo-service/src/domain/git/runtime.rs` crea un scaffold real con `Cargo.toml`, `README.md`, `openfoundry.toml` y `src/lib.rs` sobre un repo Git inicializado. | Queda ampliar el catálogo de plantillas por tipo de paquete y lenguaje. |
| 1.3.7. Libraries side panel | No | La ruta `apps/web/src/routes/code-repos/+page.svelte` cubre exploración básica, pero no encontré un panel lateral dedicado de librerías/dependencias. | Falta construir el side panel de librerías y su integración con plantillas/dependencias. |

### 1.4 Streaming

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.4.1. Stream creation con schema | Cumple | `services/streaming-service/src/handlers/streams.rs` crea streams con schema y binding reales; el smoke P2 crea un stream HTTP con schema explícito y lo consume después en una topología. | Queda ampliar validaciones de schema evolution y catálogos de conectores. |
| 1.4.2. Hot buffer + cold storage archiving | Cumple | `services/streaming-service/src/domain/engine/processor.rs` mantiene eventos calientes en `streaming_events` y archiva eventos procesados a JSONL; el smoke P2 dejó evidencia en `/tmp/of-stream-archive-p21777067824/...jsonl`. | Falta endurecer políticas de retención, replay y storage remoto. |
| 1.4.3. Fault tolerance con checkpoints | Cumple | `streaming_checkpoints` persiste offsets por `topology_id/stream_id`, y el smoke P2 validó una segunda ejecución con `input_events = 0` tras haber procesado `last_sequence_no = 2`. | Falta añadir replay/recovery más avanzado y tests de fallos intermedios. |
| 1.4.4. Job graph visualization | Parcial | `TopologyDefinition` ya modela `nodes`/`edges` y `get_runtime` devuelve snapshot operativo real. | Falta una visualización interactiva/operativa más rica en frontend y tooling de diagnóstico. |
| 1.4.5. Streaming syncs desde fuentes externas | Parcial | Ya existe una fuente HTTP/push real y runtime operativo de topologías/sinks. | Faltan conectores externos persistentes de verdad para Kafka/NATS/Kinesis y su operación completa. |
| 1.4.6. Transform de streams en Pipeline Builder | Parcial | `processor.rs` ejecuta windows, joins y sinks reales; además `apps/web/src/routes/pipelines/+page.svelte` ya integra listado, runtime snapshot y trigger operativo de topologías streaming dentro del mismo builder. | Falta editar la topología streaming completa en el mismo canvas, no solo operarla y acompañarla desde la vista híbrida. |
| 1.4.7. Push manual via API | Cumple | `POST /api/v1/streaming/streams/{id}/events` inserta eventos reales en `streaming_events`; el smoke P2 validó `accepted_events = 2`, `first_sequence_no = 1` y `last_sequence_no = 2`. | Falta añadir cuotas, auth fina y batching/compresión más avanzados. |

### 1.5 Data Lineage

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.5.1. Grafo interactivo de linaje | Cumple | `services/pipeline-service/src/domain/lineage/mod.rs` ahora materializa un grafo generalizado con nodos `dataset/pipeline/workflow` y `apps/web/src/routes/lineage/+page.svelte` lo visualiza con tipos, relaciones, markings y selección operativa. | Queda endurecer layout/UX a escala, añadir más navegación por paths y unificar mejor lineage batch/streaming en una sola superficie. |
| 1.5.2. Data Lineage (datasets) | Cumple | `services/pipeline-service/src/domain/executor.rs` sigue registrando lineage real entre datasets y ahora además `domain/lineage/mod.rs` propaga snapshots/relations `dataset -> pipeline -> dataset`; el smoke P2 validó por gateway la arista `source_dataset -> output_dataset` para el pipeline SQL real, y streaming persiste `streaming_lineage_edges` entre streams y datasets sink. | Queda integrar más profundamente el lineage de streams dentro del mismo grafo/API/UI y aumentar cobertura end-to-end. |
| 1.5.3. Workflow Lineage (GA) | Cumple | `services/workflow-service/src/domain/lineage.rs` extrae y sincroniza lineage de workflows hacia `services/pipeline-service/src/domain/lineage/mod.rs`, que lo persiste como grafo operativo con relaciones `dataset/pipeline/workflow`; `apps/web/src/routes/lineage/+page.svelte` ya lo expone visualmente. | Falta crecer hacia lineage más rico de streams, borrado/reconciliación más sofisticados y más cobertura smoke específica de workflows complejos. |
| 1.5.4. Upstream/downstream impact analysis | Cumple | `GET /api/v1/lineage/datasets/{id}/impact` en `services/pipeline-service/src/handlers/lineage.rs` calcula BFS upstream/downstream real, devuelve paths/markings/build candidates y la UI lo presenta al seleccionar datasets en `apps/web/src/routes/lineage/+page.svelte`. | Queda añadir análisis más avanzados tipo “blast radius” multi-root, filtros por dominio y explicaciones más ricas de paths. |
| 1.5.5. Builds desde Data Lineage | Cumple | `POST /api/v1/lineage/datasets/{id}/builds` dispara pipelines downstream reales desde `pipeline-service` y workflows downstream vía `/internal/workflows/{id}/runs/lineage` en `workflow-service`; la UI de lineage ya permite lanzarlos. | Falta endurecer planificación/ordenación más compleja, rollback/reintento coordinado y soporte a más tipos de build target. |
| 1.5.6. Propagación de markings por linaje | Cumple | `services/pipeline-service/src/domain/lineage/mod.rs` persiste `lineage_nodes/lineage_relations` con `marking` y `effective_marking`, propaga clasificación desde datasets/workflows/pipelines downstream y filtra graph/impact/builds por `classification_clearance`. | Queda extender la misma semántica de propagación a más dominios fuera de lineage y mejorar el recálculo cuando desaparecen dependencias. |

### 1.6 Datasets y Filesystem

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 1.6.1. Dataset con transacciones | Cumple | `upload.rs` ahora usa `sqlx::Transaction`, `SELECT ... FOR UPDATE`, journal en `dataset_transactions` y sincroniza `dataset_versions`, `dataset_schemas`, `row_count` y branch activo de forma consistente; `views.rs` reaplica el mismo patrón en refresh de views. | Queda endurecer más operaciones secundarias como branching con políticas/locks más sofisticados y rollback de side effects externos más allá del storage principal. |
| 1.6.2. Branching de datasets | Cumple | `branches.rs` ya soporta listar, crear, checkout, `merge` y `promote`; además el smoke valida creación de rama, checkout y promoción por API. | Endurecer concurrencia real, locks/transactions y conflictos de contenido más ricos. |
| 1.6.3. Dataset Views | Cumple | `handlers/views.rs`, `models/view.rs`, `domain/runtime.rs` y la migración `20260425173000_dataset_views_transactions.sql` implementan `GET/POST /views`, `POST /views/{id}/refresh` y `GET /views/{id}/preview` sobre SQL real con materialización JSON versionada bajo storage. | Falta añadir dependencias entre views, refresh automático más profundo y pushdown/materialización incremental más avanzada. |
| 1.6.4. Dataset Preview | Cumple | `preview.rs` ahora devuelve muestra real, columnas inferidas, tipos, `total_rows`, errores/warnings y soporte para `branch`/`version`; el smoke valida preview real tras una sync. | Falta ampliar a views/materializaciones y formatos más complejos con profiling más profundo. |
| 1.6.5. Data Health checks | Cumple | `quality.rs` permite crear reglas y perfiles de calidad. | Ampliar reglas avanzadas, alertas continuas y test coverage. |
| 1.6.6. Filesystem navegable | Cumple | `export.rs` ya expone `GET /api/v1/datasets/:id/files` y `/filesystem` con árbol lógico, breadcrumbs y secciones `current`, `versions`, `branches` y `views`, además de listados reales sobre storage para materializaciones/versiones. | Queda crecer desde filesystem dataset-scoped a folders/projects más globales y añadir operaciones mutativas más ricas. |
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
| 2.1.3. Properties con Value Types | Cumple | `services/ontology-service/src/handlers/properties.rs` expone CRUD real de properties con validación tipada, `schema.rs` aplica defaults/requireds y `objects.rs` valida propiedades al crear/editar objetos. | Queda ampliar tipos semánticos avanzados como geospatial/media y reglas más expresivas. |
| 2.1.4. Interfaces / Polimorfismo | Cumple | La migración `20260425003000_p3_semantic_runtime.sql`, `models/interface.rs` y `handlers/interfaces.rs` implementan interfaces, bindings `object_type_interfaces` y reutilización de schema entre tipos. | Falta endurecer herencia/polimorfismo en UI y búsquedas semánticas más avanzadas. |
| 2.1.5. Shared Property Types | Cumple | `interface_properties` + `load_effective_properties` permiten definir propiedades compartidas e incorporarlas al schema efectivo de cada object type; el smoke P3 valida `review_history` heredada por el tipo `case`. | Falta crecer hacia catálogos/versionado de property types y reutilización transversal más rica. |
| 2.1.6. Time-dependent properties | Cumple | `properties` e `interface_properties` ya soportan `time_dependent`, y el smoke P3 valida una property compartida `review_history` marcada como temporal y mutada por una Function real. | Falta modelado temporal más profundo (bitemporalidad, history queries, políticas de retención). |
| 2.1.7. Geo-point properties | Cumple | `type_system.rs` ya acepta `geo_point` con validación tipada (`lat/lon`) y `properties.rs`/`interfaces.rs` permiten declararlo tanto en properties directas como compartidas. | Falta crecer hacia semántica geoespacial más rica y su integración profunda con analytics geoespaciales. |
| 2.1.8. Media references | Cumple | `type_system.rs` ya acepta `media_reference` como `uri/url` tipado, lo que permite modelar referencias de media en object types e interfaces sin recurrir a `json` genérico. | Falta añadir gestión/preview de media más rica y asociaciones a storage/permissions de forma transversal. |
| 2.1.9. Semantic search (unstructured data) | Parcial | `handlers/search.rs`, `domain/search/fulltext.rs`, `domain/search/semantic.rs` y `domain/indexer.rs` ya exponen `/api/v1/ontology/search` con ranking mixto `fulltext + semantic` sobre types, interfaces, links, actions y object instances; además la web lo usa en `/ontology`. | Sigue faltando indexado más profundo de contenido realmente no estructurado, embeddings provider-backed y búsqueda semántica transversal a más dominios. |
| 2.1.10. Digital twin / espejo del mundo real | Cumple | `handlers/objects.rs` ya expone `/api/v1/ontology/types/{type_id}/objects/{obj_id}/view` y `/simulate`, combinando snapshot del objeto, neighbors, graph, matching rules, recent rule runs, timeline y simulación de patch/action sobre el mismo gemelo operativo. | Queda enriquecer escenarios multi-objeto y acoplarlo todavía más con process mining/Gotham. |

### 2.2 Action Types (Kinética del Ontology)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.2.1. Action Types (formularios punto a clic) | Cumple | `handlers/actions.rs` ya soporta CRUD, validación, preview, ejecución simple y batch sobre objetos reales; el smoke P3 crea una action, la valida y la ejecuta por API. | Falta añadir escenarios/branching y endurecer sandboxing/secret management. |
| 2.2.2. Function-backed Actions | Cumple | `handlers/actions.rs` + `domain/function_runtime.rs` ejecutan Functions reales (Python inline u HTTP) con contexto de objeto, object set, linked objects y justificación; el smoke P3 valida mutación real vía Function. | Falta runtime TypeScript, aislamiento más fuerte y límites operativos más finos. |
| 2.2.3. Ontology Edits TypeScript API | Cumple | `domain/function_runtime.rs` ahora expone `sdk.ontology.*` con guardrails de capabilities dentro de Functions TypeScript inline y también dentro de `ontology_function_packages`, soportando `getObject`, `updateObject`, `queryObjects`, `listNeighbors`, `createLink`, `search` y `graph` con enforcement de lectura/escritura. | Queda publicarlo como SDK externo más formal y ampliar la superficie de edición a más dominios de plataforma. |
| 2.2.4. Batch apply actions | Cumple | `POST /api/v1/ontology/actions/{id}/execute-batch` ejecuta una action sobre múltiples objetos, devuelve `succeeded/failed/results` y quedó validado en `smoke/scenarios/p3-semantic-governance-critical-path.json`. | Falta añadir transaccionalidad avanzada, retries y UX de batch apply. |
| 2.2.5. Action validation | Cumple | `validate_action` ahora valida configuración, target access, tipos de input y preview efectivo antes de ejecutar; el smoke P3 valida un preview real para una Function Python. | Falta ampliar a simulaciones/what-if y validaciones más ricas de side effects externos. |
| 2.2.6. Object Storage V2 (escritura inmediata) | Cumple | `apply_object_patch` aplica cambios contra el schema efectivo y devuelve el objeto actualizado inmediatamente; el smoke P3 verifica el objeto mutado tras la ejecución batch. | Falta endurecer garantías transaccionales multi-objeto y rollback cross-side-effects. |
| 2.2.7. Webhook / External system actions | Cumple | `InvokeWebhook` e `InvokeFunction` usan `invoke_http_action` como integración básica. | Añadir retries, secretos seguros y observabilidad. |
| 2.2.8. Permisos granulares por Action | Cumple | `permission_key` ya se hace cumplir en `ensure_action_permission`, con bloqueo verificable en `execute`/`execute-batch`; el smoke P3 demuestra además denegación `403` por aislamiento organizacional. | Falta crecer hacia políticas más compuestas/ABAC global y administración UI de permisos por action. |
| 2.2.9. Scenario / what-if branching | Cumple | `handlers/objects.rs` ahora expone `/api/v1/ontology/types/{type_id}/objects/{obj_id}/simulate`, que admite `properties_patch`, preview de `action_id`, timeline proyectado, matching rules y graph snapshot para explorar `what-if` sin persistir cambios. | Falta crecer hacia escenarios multi-objeto persistibles y ramas compartidas de simulación. |

### 2.3 Functions (Lógica de Negocio)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.3.1. Functions en TypeScript v2 | Cumple | `domain/function_runtime.rs` ya soporta `runtime: "typescript"` y ejecuta Functions inline reales vía `node --experimental-strip-types`, con contrato de contexto, salida estructurada y soporte de mutaciones (`object_patch`, `link`, `delete_object`). | Falta endurecer sandboxing, empaquetado y operación multi-runtime como plataforma Functions independiente. |
| 2.3.2. Functions en Python | Cumple | `domain/function_runtime.rs` ejecuta Functions Python reales con `pyo3`, expone `target_object`, `parameters`, `object_set`, `linked_objects`, `justification` y `context_now`; el smoke P3 valida una Function Python real que muta ontology. | Falta empaquetado/aislamiento más fuerte y un servicio Functions independiente. |
| 2.3.3. Object Set Queries | Cumple | `load_accessible_object_set` alimenta el runtime de Functions y `objects.rs` expone `POST /objects/query`; el smoke P3 valida query real de casos resueltos (`total = 2`). | Falta enriquecer filtros/joins y exponer query semantics más avanzadas. |
| 2.3.4. Link Traversals | Cumple | `load_linked_objects` alimenta Functions y `GET /objects/{id}/neighbors` expone traversal real; el smoke P3 valida navegación desde un caso hacia su analista enlazado. | Falta traversal declarativo más rico, paths multi-hop y soporte UI más potente. |
| 2.3.5. External Functions | Cumple | `FunctionInvocation` soporta tanto runtime Python inline como invocación HTTP externa desde actions. | Falta consolidarlo como plataforma Functions separada con SDK, secrets y deploy/runtime management. |
| 2.3.6. Platform SDK en Functions | Cumple | Las Functions TypeScript inline reciben `context.sdk.ontology.*` y pueden leer/editar ontology real (`getObject`, `updateObject`, `queryObjects`, `listNeighbors`, `createLink`, `search`, `graph`) usando un token interno derivado de los claims del actor. | Queda ampliar ese SDK a más dominios de plataforma y hacerlo reusable/publicable fuera del runtime inline. |
| 2.3.7. LLM en Functions (Language Model Service) | Cumple | `domain/function_runtime.rs` inyecta `context.llm.complete(...)`, que llama a `/api/v1/ai/chat/completions` con auth interna y permite combinar Functions TypeScript con AI real dentro del flujo de ontology. | Falta enriquecer la superficie LLM en Functions con embeddings, tools, guardrails más visibles y políticas/cost tracking más profundas. |

### 2.4 Object Views, Explorer y Vertex

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.4.1. Object Views | Cumple | `/ontology/{id}` ahora incluye `Object View & Simulation` real sobre cada objeto, alimentado por `/objects/{id}/view`, mostrando snapshot, graph neighborhood, matching rules, applicable actions, timeline y machinery history. | Queda especializar más vistas por vertical/caso de uso, pero la capacidad ya dejó de ser básica. |
| 2.4.2. Object Explorer | Cumple | La combinación de `/ontology` con búsqueda semántica y `/ontology/{id}` con `Object Lab`, inspector de objeto, rules, functions y simulación convierte el explorer en una superficie operativa real sobre objetos, actions y relaciones. | Falta endurecer facets/filtros más profundos y operaciones masivas más ricas, pero el explorer ya es de producto base real. |
| 2.4.3. Vertex — System graphs | Cumple | `handlers/search.rs` + `domain/graph.rs` ya exponen `/api/v1/ontology/graph` para schema graph y object neighborhood graph, y `apps/web/src/routes/ontology/graph/+page.svelte` lo visualiza con Cytoscape en modo schema u object-focused. | Falta evolucionarlo hacia simulaciones y análisis de impacto más complejos, pero el system graph ya dejó de ser placeholder. |
| 2.4.4. Vertex — Simulaciones end-to-end | Cumple | `handlers/objects.rs` simula patch + preview de action + evaluación de rules + impactados + timeline + graph, y la UI de `/ontology/{id}` lo expone desde la misma consola de inspección. | Falta profundizar en simulaciones más complejas con múltiples objetos y optimización visual más avanzada del grafo. |

### 2.5 Foundry Rules y Machinery

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.5.1. Foundry Rules | Cumple | `domain/rules.rs`, `handlers/rules.rs` y la migración `20260425233000_functions_rules_runtime.sql` implementan `ontology_rules` con trigger/effect tipados, evaluación real, simulación y aplicación sobre objetos. | Queda crecer hacia políticas y DSLs más expresivas, pero ya existe un rules engine funcional. |
| 2.5.2. Machinery (Process Mining) | Parcial | `ontology_rule_runs`, `machinery_insights` y la UI de `/ontology/{id}` ya ofrecen historial de runs, matched counts, pending schedules y last object/run por regla, lo que deja una base de observabilidad/machinery sobre el runtime semántico. | Falta process mining más profundo, path discovery y análisis temporal/procesal más cercano a un producto de minería real. |
| 2.5.3. Machinery widget de monitoreo | Cumple | `/api/v1/ontology/rules/insights` y la tarjeta `Rules & Machinery` en `/ontology/{id}` exponen un widget operativo con runs, matches, pending schedules y modo de evaluación por regla. | Queda enriquecer visualizaciones y alerting activo, pero el widget ya dejó de ser un hueco. |
| 2.5.4. Dynamic Scheduling | Cumple | `RuleEffectSpec.schedule` permite programar offsets horarios reales sobre properties temporales (`timestamp/date/string`), y `apply_rule` materializa ese schedule como patch persistido sobre el objeto. | Queda acoplarlo a planners más globales y colas/automation cross-service. |

### 2.6 Gotham Integration

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 2.6.1. Type mapping Foundry ↔ Gotham | No | No hay integración Gotham en el código actual | Diseñar interoperabilidad Gotham. |
| 2.6.2. Object Set Service | No | No hay integración Gotham en el código actual | Diseñar interoperabilidad Gotham. |

### 3.1 Model Assets y Modeling Objectives

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.1.1. Modeling Objectives | Cumple | `domain/training/runner.rs` entrena trials reales contra `training_config.records`, calcula métricas (`accuracy/precision/recall/f1/log_loss`) y `handlers/training.rs` usa ese resultado para registrar el mejor trial; el smoke P5 valida un training job real con objetivo `f1`. | Queda crecer hacia objetivos más ricos por dataset/feature store y experiment tracking más profundo. |
| 3.1.2. Model development in-platform | Cumple | `handlers/models.rs`, `handlers/training.rs`, `handlers/deployments.rs` y `handlers/predictions.rs` ya cubren crear modelo, entrenarlo, versionarlo, desplegarlo y predecirlo dentro de la plataforma; el smoke P5 recorre ese flujo completo. | Falta ampliar engines, datasets externos y UX de experimentación más avanzada. |
| 3.1.3. Import de modelos externos | Parcial | `create_model_version` permite registrar manualmente versiones con `schema/artifact_uri/metrics`, pero no hay adapters dedicados para importar formatos/frameworks externos. | Faltan adapters y workflows de import/export reales para frameworks externos. |
| 3.1.4. Batch deployment | Parcial | `ml_batch_predictions` y `create_batch_prediction` ya ejecutan scoring batch sobre deployments reales, pero la orquestación batch sigue siendo mínima. | Faltan destinos batch más ricos, scheduling y runners/targets externos. |
| 3.1.5. Live/online deployment | Cumple | `create_deployment` expone deployments activos y `POST /api/v1/ml/deployments/{id}/predict` ya usa el estado real del modelo versionado; el smoke P5 validó una predicción online positiva con explicabilidad. | Falta endurecer serving online con latencia/scale/observabilidad y endpoints externos dedicados. |
| 3.1.6. Model adapters | No | No encontré adapters específicos para XGBoost/Sklearn/PyTorch/TensorFlow más allá del runtime tabular propio. | Faltan adapters/serializadores para modelos externos y compatibilidad multi-framework. |
| 3.1.7. Versioning y reproducibilidad | Cumple | `ml_model_versions` guarda `hyperparameters`, `metrics`, `artifact_uri` y `schema` con pesos/estadísticas reales del modelo; el smoke P5 capturó `best_model_version_id` y desplegó exactamente esa versión. | Falta reforzar lineage de entrenamiento, artefactos externos y reproducibilidad multi-run más profunda. |
| 3.1.8. Governance y audit trail de modelos | Parcial | El servicio ya persiste metadata útil por training/deployment/version, pero no encontré auditoría/compliance transversal equivalente a otros dominios. | Falta integrar audit trail, approvals y policy enforcement más completos para ML. |
| 3.1.9. Staging y release to production | Parcial | `transition_model_version` soporta mover versiones a `staging/production` y `refresh_model_rollup` refleja el stage actual. | Faltan gates/approvals, validaciones previas y rollout governance más robustos. |
| 3.1.10. ML feedback loops | Parcial | `generate_drift_report` y los deployments mantienen base para retraining/monitoring, aunque el loop sigue siendo ligero. | Falta automatizar feedback loops reales y enlazarlos con datasets/labels observados. |
| 3.1.11. MLflow integration | No | No encontré integración MLflow en el repo actual. | Faltan conectores e interoperabilidad con MLflow u otros trackers externos. |
| 3.1.12. Marketplace de modelos (DevOps) | Parcial | Hay registry/versionado/deployment en `ml-service`, pero no un marketplace de modelos plenamente conectado a distribución/instalación. | Falta empaquetado/distribución de modelos y conexión más profunda con Marketplace/DevOps. |
| 3.1.13. Compute Modules (containers serverless) | No | No encontré módulos de compute serverless/containerizado para entrenamiento o serving ML. | Faltan compute modules dedicados y runners externos. |

### 3.2 AIP — Language Model Service

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.2.1. Interfaz unificada multi-LLM | Cumple | `domain/llm/runtime.rs` ya soporta proveedores HTTP reales para `chat_completions` OpenAI-compatible, `messages` tipo Anthropic y `chat/embeddings` tipo Ollama; `handlers/chat.rs` enruta por `route_rules`, y el smoke P5 validó un provider OpenAI-compatible vivo. | Queda endurecer retries/fallbacks, health tracking persistente y cobertura para más modos/proveedores. |
| 3.2.2. LLM en redes privadas | No | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.3. Multimodal / Vision-Language Models | No | `services/ai-service/src/main.rs`, `domain/llm/gateway.rs`, `domain/evaluation.rs`, `apps/web/src/routes/ai/+page.svelte` | Conectar proveedores reales, embeddings, gobernanza de coste y multimodalidad. |
| 3.2.4. LLM cost governance | Parcial | `ai_providers` ya modela `cost_tier`, `max_output_tokens`, pesos y routing, y `handlers/chat.rs`/`runtime.rs` usan esos límites al llamar al proveedor. | Falta persistir usage/costes por request y exponer reporting/cuotas de coste reales. |
| 3.2.5. Evaluations (benchmarking LLMs) | Parcial | `evaluation.rs`, cache/guardrails y el smoke P5 ya validan comportamiento real sobre provider live, pero no hay un benchmark suite de evaluación comparativa completo. | Faltan datasets de evaluación, scoring comparativo entre modelos y reporting más formal. |

### 3.3 AIP Agent Studio

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 3.3.1. AIP Agents con herramientas del Ontology | Parcial | `handlers/agents.rs` y `domain/agents/executor.rs` ya ejecutan tools HTTP reales y combinan tool output con retrieval/provider real, pero todavía no existe un paquete de tools ontology-native de primera clase. | Falta endurecer agents sobre ontology/actions seguras y ampliar el catálogo de tools nativas de plataforma. |
| 3.3.2. Deploy interno (Workshop widget) | No | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.3. Deploy externo (OSDK / APIs) | Parcial | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.4. Agents como Functions (para Automate) | No | `services/ai-service/src/handlers/agents.rs`, `services/ai-service/src/domain/agents/*` | Pasar de agentes simulados a ejecución real de herramientas y despliegue. |
| 3.3.5. Tool use / Function calling | Cumple | `ai_tools.execution_config` y `domain/agents/executor.rs` permiten invocar tools HTTP reales con `auth_mode=forward_bearer`; el smoke P5 ejecutó un tool real contra `/api/v1/ml/deployments/{id}/predict` y usó ese resultado en la respuesta final del agent. | Falta añadir function-calling más rico, tool selection más inteligente y governance/sandboxing más profundo. |
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
| 4.1.1. Punto a clic (drag-and-drop) | Cumple | `apps/web/src/routes/apps/+page.svelte` ya opera un builder real con canvas drag-and-drop, templates, palette, preview WYSIWYG, inspector y publicación; `app-builder-service` persiste apps/pages/versiones y el runtime sigue siendo operable en `/apps/runtime/[slug]`. | Queda endurecer embeds más ricos y flujos colaborativos, pero el builder punto-a-clic ya es real. |
| 4.1.2. Pro-code customizations | Cumple | `AppSettings.custom_css`, `builder_experience = slate`, el export/import `/api/v1/apps/{id}/slate-package` y el editor in-platform de `apps/+page.svelte` ya permiten pasar de Workshop a un starter React/SDK real, editarlo dentro de la plataforma y reinyectarlo usando `.openfoundry/workshop.json`. | Queda endurecer colaboración multiusuario y añadir más runtimes/frameworks además de React. |
| 4.1.3. Widget library (continuamente actualizada) | Cumple | `models/widget_type.rs` define un catálogo de widgets. | Expandir la librería y eliminar componentes UI aún vacíos. |
| 4.1.4. AIP Interactive widget (agent embed) | Cumple | `models/widget_type.rs` añade `agent`, `apps/web/src/routes/apps/+page.svelte` permite configurarlo con agentes reales y `AppWidgetRenderer.svelte` ejecuta `executeAgent(...)` contra `ai-service`, devolviendo respuesta final y traces. | Queda endurecer auth/guest access de ese widget en portales públicos y mejorar streaming/chat continuo. |
| 4.1.5. Scenario / what-if en Workshop | Cumple | `models/widget_type.rs` añade `scenario`, `createWidgetFromCatalog` lo inicializa con `set_parameters`, `AppWidgetRenderer.svelte` publica parámetros runtime reales y `AppRenderer.svelte` los propaga para parametrizar el resto del runtime. | Falta extenderlo a simulaciones multi-step más complejas y guardar escenarios versionados. |
| 4.1.6. Embedded Quiver dashboards | Cumple | `AppSettings.slate.quiver_embed`, `apps/web/src/routes/apps/+page.svelte` y `apps/web/src/routes/quiver/+page.svelte` ya permiten incrustar una vista real de Quiver dentro del workspace Slate usando `iframe` + query params tipados sobre ontology analytics vivos. | Queda llevar ese embed a más superficies runtime/widget y endurecer presets más ricos de configuración. |
| 4.1.7. Embedded Map | Cumple | Hay `map widget` y un `geospatial-service` separado. | Endurecer bindings geoespaciales, seguridad y casos avanzados. |
| 4.1.8. Consumer mode (usuarios externos B2C/B2B) | Parcial | `AppSettings.consumer_mode` ya modela branding, subtitle, CTA y `allow_guest_access`; `AppRenderer.svelte` materializa un runtime consumer-facing real sobre rutas públicas `/api/v1/apps/public/{slug}` y `/apps/runtime/[slug]`. | Falta cerrar provisioning/invitations y una historia más fuerte de usuarios externos persistentes, aunque el modo de portal ya existe. |

### 4.2 Slate (Pro-code App Builder)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.2.1. HTML / CSS / JavaScript custom apps | Cumple | `app-builder-service/src/domain/slate.rs` genera un package React/TypeScript real con `package.json`, `tsconfig.json`, `src/main.tsx`, `src/App.tsx`, theme tokens y `.openfoundry/workshop.json`; `apps/+page.svelte` lo expone y además lo convierte en workspace editable in-platform. | Queda evolucionar a más frameworks/runtimes además de React. |
| 4.2.2. Integración con Ontology layer | Parcial | El runtime Workshop ya consume ontology bindings reales, el workspace Slate puede embeder `Quiver` sobre object types reales y el starter nace desde la misma app/configuración, pero el package generado aún no trae object views/actions ontology-specific como starter de primera clase. | Faltan starters/proxies más opinionated para object views, actions y relaciones semánticas en Slate. |
| 4.2.3. Acceso directo a datasets | Cumple | El package Slate generado usa `@open-foundry/sdk/react` + `datasetDatasetListdatasets()` como seed real, y el builder mantiene widgets con bindings dataset/query que luego pueden evolucionar a la app pro-code. | Queda ampliar a más ejemplos de datasets/views/filesystem dentro del starter. |
| 4.2.4. Drag-and-drop + código | Cumple | El flujo actual ya permite diseñar en Workshop, exportar un starter Slate real, editar archivos dentro del workspace en `apps/+page.svelte` y reimportarlos vía `POST /api/v1/apps/{id}/slate-package`, cerrando un round-trip operativo Workshop <-> Slate. | Queda endurecer merge/conflict handling y colaboración concurrente sobre ese round-trip. |

### 4.3 OSDK React Applications

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 4.3.1. React UI con OSDK como backend | Cumple | `tools/of-cli/src/openapi.rs` ahora genera `sdks/typescript/openfoundry-sdk/src/react.ts` con `OpenFoundryProvider`, hooks de contexto y query/mutation mejorados; `package.json` exporta `./react`, y `app-builder-service/src/domain/slate.rs` + `code-repo-service/src/domain/git/runtime.rs` consumen esa capa en starters React reales. | Queda publicar el paquete y añadir hooks domain-specific más profundos, pero la capa React ya es real. |
| 4.3.2. TypeScript bindings type-safe | Cumple | `tools/of-cli/src/openapi.rs` genera `sdks/typescript/openfoundry-sdk/src/index.ts`, `package.json` y `tsconfig.json`; además `validate-sdk-typescript` y `tsc --noEmit` validan el SDK generado. | Queda añadir pipeline de publicación y mejorar el mapeo tipado de casos complejos/refinados. |
| 4.3.3. Soporte NPM, Pip/Conda, Maven | Parcial | `TypeScript`, `Python` y `Java` ya tienen artefactos oficiales generados/validados en `sdks/`; además `code-repo-service` ahora scaffoldea `typescript-react` y `python` con `index.html/.env.example`, workspace files, comandos de dev/preview y estructura más profunda para repos nuevos. | Sigue faltando distribución/publicación oficial y una historia explícita de Conda. |
| 4.3.4. Developer Console | Parcial | `developers/+page.svelte` y `SdkToolkit.svelte` ya centralizan OpenAPI, SDKs multi-lenguaje, React hooks, Terraform y cookbook de Slate/CLI. | Falta un playground más interactivo con credenciales/snippets ejecutables y publicación guiada. |
| 4.3.5. VS Code Workspaces in-platform | Parcial | `apps/web/src/routes/apps/+page.svelte` ahora expone un workspace/editor in-platform para Slate con selección de repo, edición de archivos, comandos de runtime y round-trip hacia `app-builder-service`; no es VS Code completo, pero ya existe una base operativa de workspace web. | Falta convertirlo en una experiencia más cercana a VS Code con tree, tabs, terminal, diff/merge y colaboración real. |
| 4.3.6. Palantir extension for VS Code | No | No encontré una extensión VS Code propia dentro del repo. | Falta empaquetar y mantener una extensión oficial con auth, SDK y flujos de plataforma. |
| 4.3.7. Palantir MCP (Model Context Protocol) | No | No encontré servidor MCP/platform MCP dedicado en el repo. | Falta exponer un MCP oficial para capacidades de plataforma. |
| 4.3.8. Ontology MCP | No | No encontré servidor MCP específico para ontology-service. | Falta exponer un MCP oficial y seguro para ontology/actions/object queries. |

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
| 5.1.1. Exploración top-down visual | Cumple | `apps/web/src/routes/contour/+page.svelte` construye una experiencia dedicada de exploración visual con charts, tabla de resultados, paths y métricas derivadas sobre datasets reales cargados vía `dataset-service`. | Queda endurecer escalabilidad para datasets mucho mayores y persistencia multiusuario de analysis boards. |
| 5.1.2. Transform boards (joins, filtros, agregaciones) | Cumple | `apps/web/src/routes/contour/+page.svelte` soporta dataset principal + join opcional, join keys, filtros search/date, dimensiones, métricas y agregaciones, con materialización del resultado. | Falta pasar de transform board browser-side a execution/persistence más profunda del lado servidor. |
| 5.1.3. Display boards (gráficos, tablas) | Cumple | `Contour` renderiza charts y tabla materializable con `EChartView.svelte`, integrados sobre resultados reales de dataset. | Queda ampliar catálogo de visualizaciones y layouts guardables. |
| 5.1.4. Paths y secuencias de análisis | Cumple | `apps/web/src/routes/contour/+page.svelte` mantiene `analysisPath` y drill breadcrumbs explícitos (`dataset`, `join`, `search`, `drill`) durante la exploración. | Falta persistir esos paths como objetos/versiones reutilizables entre usuarios. |
| 5.1.5. Parámetros de análisis | Cumple | `Contour` expone parámetros de análisis para search, rango temporal, dimensiones, métricas, agregación y join. | Queda añadir parámetros compartibles/versionados y bindings más ricos. |
| 5.1.6. Dashboards con chart-to-chart filtering | Cumple | `Contour` conecta el chart principal con el board secundario y la tabla mediante `selectedCategory`, dando drill/filtering entre visualizaciones. | Falta extender el cross-filtering a más de dos boards y a filtros persistidos. |
| 5.1.7. Export a dataset (materialización) | Cumple | `apps/web/src/routes/contour/+page.svelte` crea un dataset real vía `createDataset(...)` y persiste el resultado materializado con `uploadData(...)`. | Queda añadir naming/policies/branching más ricos para exports de análisis. |
| 5.1.8. Export PDF | Parcial | `report-service` ya soporta `generator_kind = pdf` y Contour cubre export materializado + fullscreen, pero no existe todavía un export PDF nativo desde la propia vista `Contour`. | Falta disparar `report-service`/print-to-pdf desde Contour como flujo integrado. |
| 5.1.9. Fullscreen presentation view | Cumple | `apps/web/src/routes/contour/+page.svelte` ofrece modo `fullscreen` para presentation/review de la analysis board. | Queda endurecer sharing/presentation presets y navegación tipo slideshow. |

### 5.2 Quiver (Time Series y Ontology Analytics)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.2.1. Análisis de time series | Cumple | `apps/web/src/routes/quiver/+page.svelte` construye series temporales reales sobre `ontology-service`, agregando por `dateField` y `metricField` a partir de objetos persistidos. | Queda endurecer time windows avanzadas y análisis temporal especializado. |
| 5.2.2. Point-and-click sin código | Cumple | `Quiver` ya permite escoger object types, joins, date/metric/group fields y filtros sin escribir código. | Falta persistencia multiusuario más fuerte y authoring más guiado. |
| 5.2.3. Navegación de relaciones entre object types | Cumple | `Quiver` consume `getOntologyGraph(...)` y expone overview de nodos/aristas y related nodes del tipo seleccionado. | Queda profundizar en navegación interactiva nodo-a-nodo y relation drilling completo. |
| 5.2.4. Joins entre object sets | Cumple | `apps/web/src/routes/quiver/+page.svelte` soporta join simple entre object sets vía `joinField`/`secondaryJoinField`, fusionando propiedades en la misma lente analítica. | Falta soportar joins más complejos y semántica relacional más rica que equality join simple. |
| 5.2.5. Visual functions (bloques de lógica reutilizables) | Parcial | `Quiver` guarda y reaplica presets reutilizables (`visualFunctions`) en `localStorage`, cubriendo una base usable de lenses reutilizables. | Falta persistir esas funciones en backend, compartirlas entre usuarios y versionarlas. |
| 5.2.6. Dashboards interactivos y paramétricos | Cumple | La ruta de dashboards y componentes como `DashboardGrid.svelte` y `FilterBar.svelte` cubren filtrado/composición básica. | Conectarlo a object sets/series temporales reales y a semántica Quiver completa. |
| 5.2.7. Embed en Workshop, Object Views, Carbon | Parcial | `Quiver` ya existe como superficie dedicada y `Notepad` permite referenciar embeds de tipo `quiver`, pero no hay integración nativa todavía en Workshop/Object Views/Carbon. | Faltan embeds live de primera clase en Workshop/Object Views y packaging curado. |
| 5.2.8. Vega plots | No | `apps/web/src/routes/quiver/+page.svelte` usa `EChartView.svelte`, no Vega/Vega-Lite. | Añadir soporte real de Vega/Vega-Lite y specs compartibles. |

### 5.3 Map (Geospatial Analysis)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.3.1. Análisis geoespacial y temporal | Cumple | `geospatial-service` expone capas/features reales, `report-service` ya consume esas capas junto con datasets reales y el smoke P6 valida query geoespacial + report live con mapa. | Completar GIS/raster ingestion y validarlo con datos productivos. |
| 5.3.2. Track analysis (movimiento histórico) | Parcial | `services/geospatial-service/src/main.rs`, `features.rs`, `tiles.rs`, y `apps/web/src/routes/geospatial/+page.svelte` | Endurecer GIS/raster/time-series. |
| 5.3.3. Raster imagery y capas GIS | Parcial | `services/geospatial-service/src/main.rs`, `features.rs`, `tiles.rs`, y `apps/web/src/routes/geospatial/+page.svelte` | Endurecer GIS/raster/time-series. |
| 5.3.4. Color/style por valor de dato | Cumple | `models/style.rs` y las respuestas de tiles cubren estilos básicos/vector tiles. | Añadir estilos más ricos y edición visual. |
| 5.3.5. Combinación con time series y sensores | Parcial | `geospatial-service` y `report-service` ya combinan datasets tabulares reales con capas geoespaciales reales; el smoke P6 valida esa unión en el report preview. | Falta profundizar en time-series/sensores y análisis temporal especializado. |
| 5.3.6. Standalone o embebido en Workshop | Cumple | El servicio geoespacial es independiente y hay integración embebible vía widgets/mapas. | Terminar el acoplamiento Workshop/apps y permisos. |

### 5.4 Notepad (Collaborative Documents)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.4.1. Editor de texto enriquecido colaborativo | Parcial | `services/notebook-service/src/handlers/notepad.rs` persiste documentos/presencia y `apps/web/src/routes/notepad/[id]/+page.svelte` ofrece editor markdown con polling de presencia colaborativa real. | Falta WYSIWYG rich text de verdad, coedición más fuerte y CRDT/websocket si se quiere paridad alta. |
| 5.4.2. Embed de widgets de Contour, Quiver, etc. | Parcial | `Notepad` soporta embeds persistidos de tipo `contour`, `quiver`, `report` y `fusion` dentro del documento y los refleja en export/preview. | Falta convertir esos embeds en widgets live realmente interactivos dentro del documento. |
| 5.4.3. Templates de Notepad | Cumple | `apps/web/src/routes/notepad/+page.svelte` ofrece plantillas reales (`executive-brief`, `investigation`, `operating-review`) con contenido y embeds starter. | Queda ampliar la librería de templates y su gobierno/publicación. |
| 5.4.4. Export / print de documentos | Cumple | `services/notebook-service/src/domain/notepad.rs` genera export HTML real y `apps/web/src/routes/notepad/[id]/+page.svelte` habilita preview + print. | Queda ampliar a PDF/docx nativos y distribución programada. |
| 5.4.5. Indexado por AIP Assist | Cumple | `apps/web/src/routes/notepad/[id]/+page.svelte` integra `listKnowledgeBases(...)` + `createKnowledgeDocument(...)` y marca `last_indexed_at` al indexar el documento en AIP. | Falta automatizar reindexado incremental y búsqueda contextual desde el propio editor. |
| 5.4.6. Marketplace support | No | `Notepad` ya existe como producto propio en `notebook-service`/web, pero no hay package/install/distribution vía `marketplace-service`. | Añadir packaging/publicación de templates/document apps en Marketplace. |

### 5.5 Fusion (Spreadsheet Bidireccional)

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 5.5.1. Spreadsheet editable sincronizado con dataset | Cumple | `apps/web/src/lib/components/fusion/FusionSpreadsheet.svelte` carga datasets reales vía `dataset-service`, permite editar celdas y persiste el sheet completo como nueva versión mediante `uploadData(...)`. | Queda endurecer edición concurrente, paginación/virtualización y semántica transaccional más fina por celda. |
| 5.5.2. Query de datos del Ontology en spreadsheet | Cumple | El mismo `FusionSpreadsheet.svelte` consulta object sets reales desde `ontology-service`, edita propiedades inline y persiste cada fila con `updateObject(...)`. | Falta ampliar fórmulas, joins interactivos y experiencia spreadsheet más profunda sobre ontology. |

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
| 6.2.1. Storefront de productos | Cumple | Marketplace cubre overview/browse/publish/reviews/installs, tiene ruta web dedicada y ahora las instalaciones de tipo `app_template` pueden activar/publicar una app real vía `domain/activation.rs`. | Queda ampliar installs reales más allá de `app_template`, recomendaciones y operación fleet/multi-space. |
| 6.2.2. Guided installation | Parcial | `install.rs`, `domain/activation.rs`, `browse.rs` y la ruta web ya cubren preview de dependencias/steps e instalación con activación real mínima para `app_template`. | Faltan recomendaciones contextuales, starter packs más ricos y gestión multi-space/fleet. |
| 6.2.3. Recommended products | No | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |
| 6.2.4. Starter packs / ejemplos | Parcial | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |
| 6.2.5. Instalaciones multi-space | Parcial | `services/marketplace-service/src/main.rs`, `browse.rs`, `install.rs`, `publish.rs`, y ruta web | Completar recomendaciones, starter packs y gestión multi-space. |

### 7.1 Control de Acceso

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.1.1. Role-based access control (RBAC) | Cumple | RBAC real en `rbac.rs`, `role_mgmt.rs` y `group_mgmt.rs`. | Complementarlo con mandatory controls, markings y aislamiento multi-org. |
| 7.1.2. Markings (mandatory access controls) | Parcial | `services/ontology-service/src/domain/access.rs` valida markings (`public/confidential/pii`), `objects.rs` los persiste en objetos y `ensure_object_access` los hace cumplir; el smoke P3 valida objetos `confidential` y acceso controlado. | Falta extender la enforcement/propagación a toda la plataforma y al lineage. |
| 7.1.3. Propagación de markings por linaje | Parcial | `services/pipeline-service/src/domain/lineage/mod.rs` ya propaga `marking/effective_marking` por el grafo `dataset/pipeline/workflow` y usa `classification_clearance` de claims para filtrar graph/impact/builds de lineage. | Falta llevar el mismo modelo mandatory a más superficies de plataforma, no solo al subsistema de lineage y ontology. |
| 7.1.4. Classification-based access controls (CBAC) | Parcial | `ensure_object_access` compara el marking del objeto con `classification_clearance` en claims y bloquea acceso insuficiente. | Falta generalizar el modelo CBAC fuera de ontology-service y propagación automática por más dominios. |
| 7.1.5. Scoped sessions | Parcial | `libs/auth-middleware/src/claims.rs` modela `session_scope`, `services/auth-service/src/handlers/sessions.rs` emite/revoca sesiones persistidas en `scoped_sessions`, y `services/gateway/src/proxy/service_router.rs` bloquea `method/path` fuera del scope antes de proxyear. | Falta introspección/revocación inmediata platform-wide, una UI de administración más rica y controles por recurso todavía más finos. |
| 7.1.6. Organization-level isolation | Parcial | `object_instances.organization_id` se rellena desde claims y `ensure_object_access` bloquea objetos de otra organización; el smoke P3 valida un `403` real para ejecución cross-org. | Falta extender el aislamiento a más servicios/dominios y sharing controlado entre organizaciones. |
| 7.1.7. Guest access cross-organization | Parcial | `create_guest_session` emite credenciales guest con `guest_email`, allowlists de `org/path/method/subject` y clearance; gateway y `audit-service/src/domain/security.rs` hacen cumplir ese scope en vistas/exportes auditables. | Falta invitation lifecycle/UX para invitados, sharing explícito entre organizaciones y más superficies read-only preparadas para consumo externo. |
| 7.1.8. Restricted views | Parcial | Ontology ya filtra por `ensure_object_access`; lineage filtra graph/impact/builds por `classification_clearance`; y `audit-service` filtra eventos, reports y exports por clearance, `subject_id` y org allowlists derivadas de claims/sesión. | Falta llevar el mismo patrón mandatory a dashboards, reportes de negocio y más superficies UI transversales. |
| 7.1.9. Consumer mode (external users) | Parcial | Las guest sessions read-only y subject-scoped ya permiten acceso externo acotado por gateway, claims y restricciones de organización/classification. | Falta un portal/branding de consumer mode, provisioning self-service e invitation flows más completos. |

### 7.2 Autenticación y Cifrado

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.2.1. Single Sign-On (SSO / SAML 2.0) | Parcial | `services/auth-service/src/handlers/sso.rs` ya soporta providers `oidc` y `saml`, resuelve metadata por URL, construye `AuthnRequest`, procesa callback `SAMLResponse/RelayState` y mapea atributos a `organization_id`, `classification_clearance` y `workspace`; la web ya consume ambos callbacks en `apps/web/src/routes/auth/callback/+page.svelte`. | Falta validación criptográfica más dura de assertions/certificados, bindings enterprise adicionales y posture más fuerte de hardening SAML. |
| 7.2.2. Multi-factor authentication (MFA) | Cumple | MFA implementado en `mfa.rs`. | Añadir más factores, políticas basadas en riesgo y tests. |
| 7.2.3. OAuth 2.0 (client credentials, auth code) | Parcial | `oauth.rs` y `sso.rs` implementan `authorization_code`/userinfo/state reales para OIDC, y el plano programático ya se complementa con API keys y sesiones scoping/guest. | Falta `client_credentials` estándar y posture más completo de provider management/policies OAuth. |
| 7.2.4. Encryption in transit y at rest | Parcial | `jwt.rs` y `domain/security.rs` ya cubren JWT firmados, refresh/API keys hashed y operaciones criptográficas reales vía `/api/v1/auth/cipher/*`. | Falta TLS/mTLS gestionado por la plataforma, envelope/KMS para secretos y cifrado persistente más homogéneo entre servicios. |
| 7.2.5. Zero-trust security architecture | Parcial | `services/gateway/src/proxy/service_router.rs` aplica `enforce_zero_trust_scope`, bloquea `method/path` fuera de `session_scope` y propaga contexto auth; luego `audit-service`, ontology y lineage aplican clearance/org/subject restrictions reales. | Falta mTLS/workload identity, introspección/revocación inmediata y enforcement homogéneo en todos los servicios de la plataforma. |

### 7.3 Governance y Privacidad

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 7.3.1. Audit logging completo | Cumple | Además del `audit-service` y el gateway, `ontology-service` ahora emite eventos cross-service reales por `emit_action_audit_event`; el smoke P3 valida eventos `ontology.action.execute` con metadata y justificación. | Queda ampliar cobertura a más servicios y reforzar integridad/retención global. |
| 7.3.2. Approvals (change management) | Parcial | Hay approvals en workflows y, en ontology, actions con `confirmation_required` + auditoría/justificación verificable. | Falta un framework de approvals más uniforme y multi-servicio, con estados/roles de aprobación más ricos. |
| 7.3.3. Checkpoint (justification prompts) | Parcial | `ensure_confirmation_justification` exige justificación para actions marcadas con `confirmation_required`; el smoke P3 valida el `400` cuando falta. | Falta extender checkpoints/justifications a otras superficies de cambio y políticas globales. |
| 7.3.4. Cipher (cryptographic operations) | Cumple | `services/auth-service/src/handlers/security_ops.rs` expone `/api/v1/auth/cipher/hash`, `/sign` y `/verify`, respaldado por SHA-256/HMAC reales en `services/auth-service/src/domain/security.rs`, con tests verdes. | Queda conectar KMS/HSM, rotación de claves y políticas crypto más fuertes. |
| 7.3.5. Sensitive Data Scanner (SDS) | Cumple | `services/audit-service/src/domain/sds.rs` detecta/redacta email, SSN, tarjetas, API keys y bearer tokens, y `POST /api/v1/audit/sds/scan` lo publica de forma real con test unitario. | Falta llevarlo a escaneos batch/background, diccionarios custom y hooks sobre datasets/files. |
| 7.3.6. Data Lifetime / retention policies | Parcial | `audit_policies.retention_days`, `audit_events.retention_until`, reportes de compliance y los governance templates aplicables ya materializan retención/hold reales en `audit-service`. | Falta enforcement automático (sweeper/archival) y cobertura más allá del dominio audit. |
| 7.3.7. Compliances: HIPAA, GDPR, ITAR | Parcial | `audit-service` ya genera evidence packs `hipaa`, `gdpr` e `itar`, además de flujos GDPR (`export/erase`) y templates aplicables `gdpr-default`, `hipaa-baseline` e `itar-export-control`. | Falta control mapping más profundo, evidence automation multi-servicio y una postura certificable más fuerte. |
| 7.3.8. Project templates para governance estándar | Cumple | `GET /api/v1/audit/governance/templates` lista templates reales y `POST /api/v1/audit/governance/templates/{slug}/apply` materializa políticas en `audit_policies`. | Queda ampliar el catálogo con más templates sectoriales y una UI guiada de aplicación. |

### 8.1 Control Panel y Administración

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.1.1. Control Panel centralizado | Cumple | `auth-service` persiste `control_panel_settings`, expone `GET/PUT /api/v1/control-panel`, la web tiene la ruta `apps/web/src/routes/control-panel/+page.svelte` y el smoke P6 valida lectura/escritura reales vía gateway. | Queda crecerlo con más dominios administrativos, pero ya dejó de ser solo base parcial. |
| 8.1.2. Enrollment vs Organization permissions | Cumple | `control_panel_settings.identity_provider_mappings` ya modela org/workspace/roles por `provider_slug`, `apps/web/src/routes/control-panel/+page.svelte` los administra en JSON y `handlers/sso.rs` los aplica durante el provisioning SSO para poblar `organization_id`, `workspace` y `classification_clearance` reales sobre el usuario materializado. | Queda endurecer más casos de lifecycle externo, pero el mapping enrollment/org ya es operativo. |
| 8.1.3. Resource Management | Cumple | `control_panel_settings.resource_management_policies` persiste quotas reales (`max_query_limit`, `max_pipeline_workers`, `requests_per_minute`, etc.), `handlers/sso.rs` las transforma en `tenant_tier` + `tenant_quotas` y el gateway ya usa esas claims vía `TenantContext` para clamps y rate limiting efectivos. | Falta más observabilidad/forecasting, pero la administración de recursos dejó de ser solo flags estáticos. |
| 8.1.4. Upgrade Assistant | Cumple | `control_panel_settings.upgrade_assistant` ya modela versiones, preflight checks, stages y rollback; además `auth-service` expone `GET /api/v1/control-panel/upgrade-readiness` con validación viva sobre proveedores SSO, políticas ABAC y scoped sessions, y la UI lo presenta en `/control-panel`. | Queda sofisticar checks y dependencias entre servicios, pero el asistente guiado ya es real y verificable. |
| 8.1.5. Identity provider mapping (SAML org assignment) | Cumple | `handlers/sso.rs` combina claim mapping del provider con `identity_provider_mappings` del control panel, valida dominios permitidos, resuelve `organization_id`/workspace/clearance y asigna roles por nombre en login SAML/OIDC. | Falta añadir más plantillas de IdP, pero la asignación SAML/OIDC por org ya funciona de verdad. |
| 8.1.6. Custom platform branding | Cumple | `control_panel_settings.default_app_branding` ya se gestiona desde `auth-service` y desde la nueva ruta web `/control-panel`; el smoke P6 valida actualización real de branding (`display_name`, colores y `show_powered_by`). | Queda propagar branding dinámico a más superficies publicadas del frontend/runtime. |

### 8.2 Enablement y Documentación

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.2.1. AIP Assist (platform-wide chatbot) | Parcial | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |
| 8.2.2. Custom documentation in-platform | Parcial | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |
| 8.2.3. Walkthroughs (tutoriales interactivos) | No | copilot y knowledge bases en `ai-service`, con superficies parciales en web | Extender copilot/docs y walkthroughs a toda la plataforma. |

### 8.3 Multi-Organization Ecosystems

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 8.3.1. Private + shared spaces | Cumple | `nexus_spaces` persiste espacios `private/shared`, `handlers/spaces.rs` expone `GET/POST/PATCH /api/v1/nexus/spaces`, la web añade `SpaceManager.svelte` y los shares pueden ligarse a `provider_space_id` / `consumer_space_id` con validación de membresía en `domain/governance.rs`. | Queda endurecer más políticas y vistas agregadas, pero los espacios multi-org ya existen como primitive real. |
| 8.3.2. Data sharing controlado entre orgs | Cumple | `services/nexus-service/src/domain/governance.rs` endurece contratos/shares/federated queries; el smoke P6 valida rechazo con consumer pendiente, share activo controlado, grant ligado al consumer y bloqueo de SQL mutante. | Queda ampliar políticas y observabilidad, pero el sharing controlado ya es real y verificable. |
| 8.3.3. Host organization + partners | Cumple | `nexus_peers` ahora modela `organization_type`, `lifecycle_stage` y `admin_contacts`; `handlers/peers.rs` los crea/actualiza, `authenticate_peer` promueve lifecycle a `active`, `NexusOverview` suma métricas de spaces y la UI enseña host/partner lifecycle y contactos operativos. | Queda crecer hacia fleet/marketplace más ricos, pero la administración de host + partners ya está implementada. |

### 9.1 APIs REST

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 9.1.1. Foundry Platform API (v1 y v2) | Parcial | `tools/of-cli/src/openapi.rs` ya versiona la spec con overlays REST para `/api/v2/admin/...` y `/api/v2/filesystem/datasets/{dataset_id}`; `services/gateway/src/proxy/service_router.rs` enruta ambos prefijos y `auth-service`/`dataset-service` publican aliases v2 reales. | Falta extender la normalización `v2` al resto de microservicios, cerrar garantías de compatibilidad interversión y sumar más tests end-to-end de contratos. |
| 9.1.2. Datasets API | Parcial | `dataset-service` ya cubre CRUD, upload transaccional, preview rico, schema, versions, branching (`merge/promote`), `views`, `transactions` y filesystem lógico `/files`/`/filesystem`, todo accesible vía gateway; además ahora publica `GET /api/v2/filesystem/datasets/{id}` y el cliente TS en `apps/web/src/lib/api/datasets.ts` refleja estas superficies. | Homogeneizar mejor contratos entre endpoints dataset/filesystem, ampliar más `v2` y reforzar tests de integración específicos. |
| 9.1.3. Ontologies API (Objects, Links, Actions) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.4. Orchestration API (Builds, Jobs, Schedules) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.5. Streams API (real-time, second latency) | Parcial | rutas `/api/v1/...` distribuidas en microservicios y gateway | Homogeneizar APIs, añadir v2/filesystem/admin y más tests. |
| 9.1.6. Connectivity API (external systems) | Parcial | `connections.rs` y `sync_ops.rs` ya exponen creación, `test_connection` real, sync jobs persistentes, retries y estados; los conectores PostgreSQL/CSV/JSON/REST API/Salesforce están implementados en los caminos principales. | Queda ampliar cobertura enterprise, incrementalidad, descubrimiento de fuentes y contratos más homogéneos entre conectores. |
| 9.1.7. Filesystem API (folders, projects) | Parcial | `dataset-service` expone `/api/v1/datasets/{id}/filesystem` y ahora también `/api/v2/filesystem/datasets/{id}`; la superficie quedó documentada en `apps/web/static/generated/openapi/openfoundry.json` y consumible desde los SDKs `TypeScript`, `Python` y `Java`. | Sigue siendo un filesystem dataset-scoped; faltan folders/projects globales, operaciones mutativas y una administración más amplia fuera del caso dataset. |
| 9.1.8. SQL Queries API | Cumple | `query-service` expone `/execute` y `/explain`. | Añadir federación, drivers externos y observabilidad. |
| 9.1.9. Admin API (Users, Groups, Markings, Orgs) | Parcial | `auth-service/src/main.rs` ya publica aliases `/api/v2/admin/users`, `/roles`, `/groups`, `/permissions`, `/policies` y `/control-panel`; `gateway` los enruta y la OpenAPI/SDKs los exponen como contrato formal multi-lenguaje. | Falta ampliar el bloque admin hacia orgs/markings más completos, endurecer compatibilidad de contratos y aumentar la cobertura de integración. |

### 9.2 SDKs

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 9.2.1. Foundry Platform SDK (Python) | Cumple | `tools/of-cli/src/openapi.rs` y `tools/of-cli/src/main.rs` generan/validan `sdks/python/openfoundry-sdk` con `pyproject.toml`, `README.md`, `openfoundry_sdk/client.py` y `models.py`; además `python3 -m compileall sdks/python/openfoundry-sdk` pasa y la spec incluye ya los overlays `v2` admin/filesystem. | Queda publicar el paquete y mejorar ergonomía idiomática adicional (auth helpers, async/streaming y ejemplos más amplios). |
| 9.2.2. OSDK (TypeScript/NPM) | Cumple | `tools/of-cli/src/openapi.rs` y `tools/of-cli/src/main.rs` generan/validan `sdks/typescript/openfoundry-sdk`, `.github/workflows/proto-check.yml` detecta drift y el SDK pasa `tsc --noEmit`. | Queda publicar el paquete, añadir helpers más ergonómicos y ampliar cobertura idiomática/framework-specific. |
| 9.2.3. OSDK (Python/Pip) | Cumple | El artefacto `sdks/python/openfoundry-sdk` ya existe como paquete Python instalable con `pyproject.toml`, deriva de la misma OpenAPI versionada y queda cubierto por `validate-sdk-python` + `compileall` en el flujo de verificación. | Falta publicación externa, ejemplos más ricos y helpers idiomáticos adicionales. |
| 9.2.4. OSDK (Java/Maven) | Cumple | `tools/of-cli` genera/valida `sdks/java/openfoundry-sdk` con `pom.xml` y `src/main/java/com/openfoundry/sdk/OpenFoundryClient.java`; `.github/workflows/proto-check.yml` levanta Java 17 y compila el SDK generado para detectar drift. | Queda enriquecer DTOs/serialización Java y publicar el artefacto en un registry Maven. |
| 9.2.5. OpenAPI spec (any language) | Cumple | `openapi.rs` genera la spec versionada en `apps/web/static/generated/openapi/openfoundry.json`, ya enriquecida con overlays REST `v2`, y ahora sirve como fuente única de los SDKs oficiales `TypeScript`, `Python` y `Java`. | Queda publicar/versionar externamente la spec y formalizar mejor las garantías de compatibilidad entre versiones. |

### 10. INFRAESTRUCTURA Y DEPLOYMENT

| Item | Estado | Evidencia | Gap concreto |
| --- | --- | --- | --- |
| 10.1. SaaS multi-cloud (AWS, Azure, GCP, OCI) | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |
| 10.2. On-premises / air-gapped deployment | Parcial | Además de `docker-compose`, el chart Helm se reescribió con `existingSecret`, wiring interno entre servicios, ingress, network policy, PDB y overlays `dev/staging/prod`; la validación quedó automatizada en `.github/workflows/helm-check.yml`. | Falta cerrar secretos/offline registries/conectores privados y una historia air-gapped más fuerte. |
| 10.3. Apollo (CI/CD autónomo) | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |
| 10.4. Kubernetes autoscaling build system | Parcial | El chart Helm ahora define HPA/KEDA con overlays por entorno y `helm-check.yml` renderiza/linta base, staging y prod para evitar drift. | Falta demostrar autoscaling real bajo carga y extenderlo a más servicios/build runtimes. |
| 10.5. High availability / autoscaling compute mesh | Parcial | El chart añade `serviceAccount`, probes, `topologySpreadConstraints`, PDB, ingress y network policy, reforzando una base HA/autoscaling más seria. | Sigue faltando validación de reparto real de carga, chaos/failover y malla compute completa. |
| 10.6. Geo-restricted enrollments | No | `infra/docker-compose.yml`, chart Helm y provider Terraform | Falta una historia real de SaaS multi-cloud/Apollo/deploy enterprise restringido. |

## Notas Finales

- Lo más sólido hoy: auth base (RBAC/MFA/SSO), CRUD y versionado básico de datasets, ontology base, workflows/notificaciones, query engine y notebooks.
- Lo más débil: infraestructura enterprise demostrable, BI/workspaces de ciencia de datos más completos, fleet/marketplace avanzado y el hardening transversal fuera de los golden paths ya cerrados.
- También pesan en la evaluación los placeholders y archivos vacíos detectados durante la revisión, porque indican que varias capacidades del checklist todavía no están cerradas.
