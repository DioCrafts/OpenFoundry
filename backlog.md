# Backlog Post-P6

Este backlog resume lo que sigue pendiente después del cierre mínimo de `P0` a `P6` en [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md), cruzado con la evidencia y gaps aún abiertos en [`checklist_status.md`](./checklist_status.md).

La lectura correcta del estado actual es:

- El roadmap crítico mínimo quedó cubierto en alcance base.
- El checklist completo todavía no está cerrado.
- El trabajo que queda ya no es “salir de la simulación” en el golden path principal, sino endurecer, ampliar cobertura y completar superficie de producto.

## Foto rápida

- Total checklist evaluado: `244` ítems
- `Cumple`: `102`
- `Parcial`: `83`
- `No`: `59`
- Pendientes totales (`Parcial + No`): `142`

## 1. Pendiente real

Estas son deudas de producto, plataforma o hardening que siguen abiertas de verdad.

### 1.1 Prioridad alta

- Hardening de conectividad enterprise tras el salto de base.
  Base: ya quedaron cubiertos `sap`, `iot`, `discover`, `auto-registration`, `bulk registration`, `zero_copy`, update detection por `source_signature`, egress controls y un on-prem agent base con `register/heartbeat/proxy`. Lo que sigue aquí ya es amplitud y endurecimiento: más conectores enterprise (Snowflake/BigQuery/Kafka/S3...), agent empaquetado/autónomo con mTLS, pushdown/federación zero-copy más profunda y mejor gestión de secretos/policies.
  Referencias: [`checklist_status.md`](./checklist_status.md#11-data-connection-conectores-y-fuentes)

- Hardening de datasets tras el salto a views/filesystem/transacciones.
  Base: ya quedaron cubiertos `Dataset Views` materializadas, preview de views, filesystem lógico navegable (`current`, `versions`, `branches`, `views`), journal `dataset_transactions` y uploads con lock transaccional + actualización de schema/row_count. Lo que sigue aquí ya es amplitud: branching con semántica transaccional todavía más fuerte, auto-refresh/dependencias de views, filesystem global tipo folders/projects y operaciones mutativas más ricas.
  Referencias: [`checklist_status.md`](./checklist_status.md#16-datasets-y-filesystem)

- Hardening posterior del pipeline builder híbrido.
  Base: ya quedaron cubiertos nodos `spark` y `external` con compute remoto HTTP real, AI assist operativo dentro del builder y una vista batch/streaming unificada con runtime de topologías. Lo que sigue aquí ya es hardening: canvas drag-and-drop más fuerte, edición streaming completa en la misma superficie, runtimes nativos adicionales (Spark/Polars más empaquetados) y observabilidad/repair más profundos.
  Referencias: [`checklist_status.md`](./checklist_status.md#12-pipeline-builder), [`checklist_status.md`](./checklist_status.md#14-streaming)

- Hardening posterior del lineage operativo.
  Base: ya hay grafo `dataset/pipeline/workflow`, `Workflow Lineage`, análisis upstream/downstream, builds downstream y propagación de `markings` con filtrado por clearance. Lo que queda aquí ya es amplitud y endurecimiento: integrar mejor streams en el mismo grafo, recalcular propagación al borrar dependencias, ampliar smoke coverage y enriquecer UX/pathing.
  Referencias: [`checklist_status.md`](./checklist_status.md#15-data-lineage), [`checklist_status.md`](./checklist_status.md#71-control-de-acceso)

- Hardening posterior de seguridad transversal.
  Base: ya quedaron cubiertos `scoped sessions`, `guest sessions`, enforcement zero-trust `method/path` en gateway, callback dual `OIDC/SAML`, `cipher` API real, `Sensitive Data Scanner` y templates de governance/compliance aplicables. Lo que sigue aquí ya es hardening serio: validación criptográfica SAML más fuerte, `OAuth client_credentials`, introspección/revocación inmediata de sesiones, mTLS/KMS, retención automática y propagación mandatory más homogénea por toda la plataforma.
  Referencias: [`checklist_status.md`](./checklist_status.md#71-control-de-acceso), [`checklist_status.md`](./checklist_status.md#72-autenticación-y-cifrado), [`checklist_status.md`](./checklist_status.md#73-governance-y-privacidad)

- Hardening posterior de APIs y SDKs multi-lenguaje.
  Base: ya quedaron cubiertos la OpenAPI versionada con overlays REST `v2`, aliases formales para `admin/filesystem` y SDKs oficiales `TypeScript`, `Python` y `Java` generados desde la misma spec y validados en CI. Lo que sigue aquí ya es endurecimiento: ampliar `v2` al resto de dominios, publicar paquetes en sus registries, enriquecer el SDK Java con modelos/serialización más fuertes y crecer la superficie admin/filesystem más allá del caso actual.
  Referencias: [`checklist_status.md`](./checklist_status.md#91-apis-rest), [`checklist_status.md`](./checklist_status.md#92-sdks)

### 1.2 Prioridad media

- Ontology hardening restante tras el cierre base de Functions/Explorer.
  Base: ya quedaron cubiertos `geo_point`, `media_reference`, búsqueda `fulltext + semantic`, runtime inline `typescript`, Platform SDK en Functions, `llm.complete(...)` y un `Vertex`/Explorer base real. Lo que sigue pendiente aquí ya es hardening y amplitud: digital twin más profundo, simulaciones end-to-end, Rules/Machinery, sandboxing/packaging más fuerte para Functions y un explorer/vertex todavía más cercano al producto final.
  Referencias: [`checklist_status.md`](./checklist_status.md#21-ontology-manager--tipos-semánticos), [`checklist_status.md`](./checklist_status.md#22-action-types-kinética-del-ontology), [`checklist_status.md`](./checklist_status.md#23-functions-lógica-de-negocio), [`checklist_status.md`](./checklist_status.md#24-object-views-explorer-y-vertex)

- AI/ML enterprise hardening.
  Base: P5 cerró el camino crítico, pero siguen pendientes private LLM, multimodalidad, cost governance persistente, benchmark suites, adapters de modelos externos, MLflow, compute modules, rollout governance y feedback loops más reales.
  Referencias: [`checklist_status.md`](./checklist_status.md#31-model-assets-y-modeling-objectives), [`checklist_status.md`](./checklist_status.md#32-aip--language-model-service), [`checklist_status.md`](./checklist_status.md#33-aip-agent-studio), [`checklist_status.md`](./checklist_status.md#34-aip-logic-y-automate)

- Hardening posterior de Workshop / Slate / developer experience.
  Base: ya quedaron cubiertos `consumer mode` configurable, `scenario/what-if`, widget de agente con ejecución real, export Slate React real desde `app-builder-service`, presets/scaffolds `TypeScript/React` y `Python` en `code-repo-service`, y hooks React generados en `@open-foundry/sdk/react`. Lo que sigue aquí ya es hardening y amplitud: Quiver embebido, round-trip Workshop <-> Slate, ontology examples más profundos en Slate, portales externos con lifecycle más fuerte y workspaces/editor in-platform más cercanos a producto final.
  Referencias: [`checklist_status.md`](./checklist_status.md#41-workshop-no-code--low-code-app-builder), [`checklist_status.md`](./checklist_status.md#42-slate-pro-code-app-builder), [`checklist_status.md`](./checklist_status.md#43-osdk-react-applications)

- Marketplace y DevOps más completos.
  Base: `app_template` ya se activa de verdad, pero siguen abiertos recommended products, multi-space/fleet, packaging más serio, release channels completos y maintenance windows.
  Referencias: [`checklist_status.md`](./checklist_status.md#61-foundry-devops), [`checklist_status.md`](./checklist_status.md#62-marketplace)

- Hardening posterior de control panel y multi-org.
  Base: ya quedaron cubiertos `identity_provider_mappings` reales con provisioning SSO, `resource_management_policies` que alimentan `tenant_tier`/`tenant_quotas`, `upgrade_readiness` con checks vivos, `private/shared spaces` persistentes y lifecycle/admin contacts de peers host/partner. Lo que sigue aquí ya es endurecimiento: más policies ecosistémicas, revocación/propagación cross-service más profunda, automation de partner lifecycle y vistas/fleet operativas más ricas.
  Referencias: [`checklist_status.md`](./checklist_status.md#81-control-panel-y-administración), [`checklist_status.md`](./checklist_status.md#83-multi-organization-ecosystems)

- Infra enterprise más demostrable.
  Base: Helm quedó más serio, pero todavía no hay validación fuerte de air-gapped, multi-cloud, autoscaling real bajo carga, HA/failover y compute mesh completo.
  Referencias: [`checklist_status.md`](./checklist_status.md#10-infraestructura-y-deployment)

### 1.3 Prioridad media-baja

- Analytics maduros tipo producto.
  Base: ya quedaron cubiertos `Contour` real sobre datasets con joins/filtros/agregaciones/drill/export/fullscreen, `Quiver` real sobre ontology con time series/object-set joins/graph navigation/visual functions, `Notepad` persistido con presencia/export/indexado AIP y `Fusion Spreadsheet` bidireccional sobre datasets y objetos. Lo que sigue aquí ya es hardening: PDFs nativos desde Contour, Vega en Quiver, rich text/coedición más fuerte en Notepad y una UX spreadsheet más profunda.
  Referencias: [`checklist_status.md`](./checklist_status.md#51-contour-top-down-analysis), [`checklist_status.md`](./checklist_status.md#52-quiver-time-series-y-ontology-analytics), [`checklist_status.md`](./checklist_status.md#54-notepad-collaborative-documents), [`checklist_status.md`](./checklist_status.md#55-fusion-spreadsheet-bidireccional)

- BI connectors y workspaces de ciencia de datos más completos.
  Base: faltan Tableau, Power BI, ODBC/JDBC, Python SDK para BI, RStudio y notebooks con LLM mejor integrados.
  Referencias: [`checklist_status.md`](./checklist_status.md#56-code-workspaces-y-code-workbook), [`checklist_status.md`](./checklist_status.md#57-integraciones-bi-externas)

## 2. Pendiente documental

Estas no son carencias funcionales del producto, sino incoherencias, desalineaciones o documentos que conviene actualizar para que el estado del repo se entienda mejor.

- Marcar `P0` explícitamente como cerrado en `roadmap_checklist_gaps.md`, igual que `P1` a `P6`.
  Hoy `P0` tiene entregables y `Definition of Done`, pero no un estado de cierre equivalente.
  Referencia: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md#fase-p0-fundación-de-ingeniería)

- Reescribir la sección “Ruta sugerida para los próximos 90 días”.
  Ahora mismo sigue diciendo “Cerrar P0”, “Ejecutar P1” y “P2 ejecutado; arrancar P3”, aunque esas fases ya aparecen como cerradas en el mismo documento.
  Referencia: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md#ruta-sugerida-para-los-próximos-90-días)

- Añadir una nueva fase post-P6 o un bloque explícito de “hardening y amplitud”.
  El roadmap actual termina en P6, pero el propio texto deja claro que queda bastante backlog posterior.
  Referencias: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md#fase-p6-analytics-y-hardening-enterprise), [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md#qué-no-priorizar-todavía)

- Explicar mejor que “cerrado” en el roadmap significa “cerrado en alcance mínimo”, no “cumplimiento completo del checklist”.
  Ahora eso se entiende al leer con cuidado, pero convendría dejarlo más explícito al principio del documento.
  Referencia: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md)

- Normalizar la nomenclatura de smokes por fase.
  En `checklist_status.md`, la verificación “P1 posterior” referencia `smoke/scenarios/p0-critical-path.json`, lo que puede confundir aunque el flujo esté bien.
  Referencia: [`checklist_status.md`](./checklist_status.md#método)

- Añadir una nota ejecutiva al principio de `checklist_status.md`.
  Conviene explicitar desde arriba que el repo ya cubre un golden path real, pero que el checklist total sigue mayoritariamente en `Parcial/No`.
  Referencias: [`checklist_status.md`](./checklist_status.md#resumen-por-dominio), [`checklist_status.md`](./checklist_status.md#notas-finales)

## 3. Pendiente explícitamente no priorizado

Estas líneas existen en el checklist, pero no conviene meterlas por delante del hardening post-P6.

- `1.7 HyperAuto (SDDI)`
- `2.6 Gotham Integration`
- `10.3 Apollo`
- `10.6 Geo-restricted enrollments`
- `4.3.5+` extensiones IDE/MCP complejas mientras no haya SDKs y APIs más maduras

Referencia: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md#qué-no-priorizar-todavía)

## 4. Orden recomendado después de P6

Si hay que seguir cerrando deuda con buen retorno, este sería el orden recomendado:

1. Seguridad transversal, APIs/SDKs y conectividad enterprise.
2. Dataset views/filesystem, lineage operativo y pipeline builder más fuerte.
3. Ontology/Functions como plataforma y hardening AI/ML enterprise.
4. Marketplace/fleet, control panel avanzado y multi-org más completo.
5. BI/connectores externos y workspaces de ciencia de datos más completos.
6. Líneas explícitamente no priorizadas.

## 5. Criterio práctico de cierre

Podemos considerar este backlog razonablemente saneado cuando se cumplan estas condiciones:

- El porcentaje de `No` baja de forma visible en conectividad, seguridad, APIs/SDKs e infraestructura.
- Los ítems `Parcial` del golden path pasan a `Cumple` en su mayoría.
- El roadmap deja de estar “cerrado en alcance mínimo” y pasa a reflejar una fase explícita de hardening posterior.
- El repo puede defender no solo un flujo demo real, sino también operación, extensibilidad y gobierno de plataforma con menos huecos notorios.
