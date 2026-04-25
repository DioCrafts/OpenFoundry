# Remaining Gaps

Este documento resume lo que **sigue pendiente de verdad** después del cierre mínimo de `P0` a `P6`, cruzando [`backlog.md`](./backlog.md) con el estado más reciente de [`checklist_status.md`](./checklist_status.md).

## Foto actual

- Total evaluado: `244` ítems
- `Cumple`: `129`
- `Parcial`: `76`
- `No`: `39`
- Pendiente total (`Parcial + No`): `115`

Nota: esta foto sale del resumen vigente de [`checklist_status.md`](./checklist_status.md#resumen-por-dominio). La “foto rápida” de [`backlog.md`](./backlog.md#foto-rápida) ya quedó desactualizada.

## 1. Pendiente funcional real

### Prioridad alta

- Seguridad transversal y zero-trust más profundos.
  Base: ya existen `RBAC`, `MFA`, sesiones `scoped/guest`, enforcement por `method/path`, `SAML/OIDC`, `cipher`, `SDS` y parte de `markings`.
  Lo pendiente real: revocación/introspección inmediata, `OAuth client_credentials`, `mTLS/KMS`, retención automática, propagación homogénea de `markings/CBAC` y enforcement transversal fuera de los dominios ya cubiertos.
  Referencias: [`checklist_status.md`](./checklist_status.md#71-control-de-acceso), [`checklist_status.md`](./checklist_status.md#72-autenticación-y-cifrado), [`checklist_status.md`](./checklist_status.md#73-governance-y-privacidad)

- APIs y SDKs endurecidos más allá del caso base.
  Base: ya hay OpenAPI versionada, overlays `v2`, aliases `admin/filesystem` y SDKs `TypeScript`, `Python` y `Java`.
  Lo pendiente real: extender `v2` al resto de dominios, cerrar compatibilidad contractual, ampliar `Admin API` y `Filesystem API`, publicar SDKs en registries y reforzar tests end-to-end de contratos.
  Referencias: [`checklist_status.md`](./checklist_status.md#91-apis-rest), [`checklist_status.md`](./checklist_status.md#92-sdks)

- Conectividad enterprise ampliada y endurecida.
  Base: ya quedaron cubiertos `sap`, `iot`, `discover`, `auto-registration`, `bulk registration`, `zero_copy`, `update detection`, `egress controls` y un `on-prem agent` base.
  Lo pendiente real: catálogo de conectores mucho más amplio (`Snowflake`, `BigQuery`, `Kafka`, `S3`, etc.), agent empaquetado/autónomo con `mTLS`, mejores secrets/policies y streaming connectors persistentes de verdad.
  Referencias: [`checklist_status.md`](./checklist_status.md#11-data-connection-conectores-y-fuentes), [`checklist_status.md`](./checklist_status.md#14-streaming)

- Infraestructura y deployment enterprise demostrables.
  Base: Helm/CI ya existen y hay una base operativa real.
  Lo pendiente real: `air-gapped`, multi-cloud, HA/failover, autoscaling bajo carga, compute mesh más completo y posture enterprise de despliegue.
  Referencias: [`checklist_status.md`](./checklist_status.md#10-infraestructura-y-deployment)

- AI/ML enterprise todavía lejos de cierre.
  Base: P5 cerró provider real, RAG base, tools HTTP, training/deployment/prediction reales.
  Lo pendiente real: `private LLM`, multimodalidad, cost governance persistente, evaluaciones/benchmarking más fuertes, model adapters, `MLflow`, compute modules, rollout governance y feedback loops más reales.
  Referencias: [`checklist_status.md`](./checklist_status.md#31-model-assets-y-modeling-objectives), [`checklist_status.md`](./checklist_status.md#32-aip--language-model-service), [`checklist_status.md`](./checklist_status.md#33-aip-agent-studio), [`checklist_status.md`](./checklist_status.md#34-aip-logic-y-automate)

### Prioridad media

- Datasets, filesystem y semántica transaccional más fuertes.
  Base: `views`, preview, branching, quality, journal transaccional y filesystem lógico ya son reales.
  Lo pendiente real: dependencias/auto-refresh entre views, branching más fuerte, filesystem global tipo folders/projects y operaciones mutativas más ricas.
  Referencias: [`checklist_status.md`](./checklist_status.md#16-datasets-y-filesystem)

- Pipeline builder y streaming todavía por endurecer.
  Base: el runtime batch/streaming ya es real y el builder híbrido existe.
  Lo pendiente real: canvas drag-and-drop más serio, edición streaming completa en la misma superficie, mejores runtimes Spark/compute y `LLM-powered transforms`.
  Referencias: [`checklist_status.md`](./checklist_status.md#12-pipeline-builder), [`checklist_status.md`](./checklist_status.md#14-streaming)

- Lineage más profundo y más transversal.
  Base: ya hay lineage `dataset/pipeline/workflow`, impacto upstream/downstream, builds desde lineage y propagación base de `markings`.
  Lo pendiente real: integrar mejor streams, recalcular propagación en más escenarios, endurecer UX/pathing y extender el modelo mandatory a más superficies.
  Referencias: [`checklist_status.md`](./checklist_status.md#15-data-lineage), [`checklist_status.md`](./checklist_status.md#71-control-de-acceso)

- Ontology residual después del uplift de plataforma.
  Base: ya quedaron cubiertos `function packages` reutilizables con policies/capabilities, `rules` reales con evaluación/aplicación, `machinery insights`, `object views` enriquecidas, simulación end-to-end por objeto y un explorer mucho más operativo.
  Lo pendiente real: process mining más profundo, sandboxing más fuerte especialmente para Python, explorer con más facets/operaciones masivas, simulaciones multi-objeto persistibles y Gotham.
  Referencias: [`checklist_status.md`](./checklist_status.md#21-ontology-manager--tipos-semánticos), [`checklist_status.md`](./checklist_status.md#22-action-types-kinética-del-ontology), [`checklist_status.md`](./checklist_status.md#23-functions-lógica-de-negocio), [`checklist_status.md`](./checklist_status.md#24-object-views-explorer-y-vertex), [`checklist_status.md`](./checklist_status.md#25-foundry-rules-y-machinery)

- Product delivery, Marketplace y fleet management siguen muy verdes.
  Base: ya hay packaging/instalación mínimos reales y activación base de productos.
  Lo pendiente real: release channels completos, fleet management, maintenance windows, recommended products, starter packs más ricos e instalaciones multi-space.
  Referencias: [`checklist_status.md`](./checklist_status.md#61-foundry-devops), [`checklist_status.md`](./checklist_status.md#62-marketplace)

- Control panel y multi-org todavía necesitan más profundidad operativa.
  Base: ya existen `identity_provider_mappings`, quotas/policies, `upgrade_readiness`, `private/shared spaces` y governance multi-org base.
  Lo pendiente real: propagación/revocación cross-service más fuerte, lifecycle ecosistémico más automatizado y administración más rica a escala.
  Referencias: [`checklist_status.md`](./checklist_status.md#81-control-panel-y-administración), [`checklist_status.md`](./checklist_status.md#83-multi-organization-ecosystems)

### Prioridad media-baja

- Analytics tipo producto: bastante mejor, pero no cerrado.
  Base: `Contour`, `Quiver`, `Notepad` y `Fusion` ya existen con flujos reales.
  Lo pendiente real: PDF nativo desde Contour, `Vega/Vega-Lite` en Quiver, rich text/coedición más fuerte en Notepad, embeds más vivos y una UX spreadsheet más profunda.
  Referencias: [`checklist_status.md`](./checklist_status.md#51-contour-top-down-analysis), [`checklist_status.md`](./checklist_status.md#52-quiver-time-series-y-ontology-analytics), [`checklist_status.md`](./checklist_status.md#54-notepad-collaborative-documents), [`checklist_status.md`](./checklist_status.md#55-fusion-spreadsheet-bidireccional)

- Workspaces de ciencia de datos e integraciones BI externas.
  Base: hay base útil en notebooks y analytics.
  Lo pendiente real: `JupyterLab` más fuerte, LLMs en notebooks, `RStudio`, legacy workbook más completo y conectores `Tableau`, `Power BI`, `ODBC/JDBC` y Python SDK para BI.
  Referencias: [`checklist_status.md`](./checklist_status.md#56-code-workspaces-y-code-workbook), [`checklist_status.md`](./checklist_status.md#57-integraciones-bi-externas)

- Workshop / Slate / developer experience quedan ya en fase residual.
  Base: consumer mode, scenarios, agent widget, export/import Slate, workspace/editor in-platform, Quiver embebido, scaffolds más profundos `TypeScript/React` y `Python`, y React SDK con `Provider` + hooks de contexto ya están en pie.
  Lo pendiente real: colaboración multiusuario, tree/tabs/terminal más tipo IDE, publicación oficial de SDKs, más runtimes/frameworks y una ergonomía todavía más fuerte alrededor de snippets/playgrounds.
  Referencias: [`checklist_status.md`](./checklist_status.md#41-workshop-no-code--low-code-app-builder), [`checklist_status.md`](./checklist_status.md#42-slate-pro-code-app-builder), [`checklist_status.md`](./checklist_status.md#43-osdk-react-applications)

- Enablement y documentación in-product todavía parciales.
  Base: hay copilot, KB y superficies parciales.
  Lo pendiente real: walkthroughs interactivos, documentación custom en plataforma y asistencia más transversal.
  Referencias: [`checklist_status.md`](./checklist_status.md#82-documentación-y-onboarding)

### Explícitamente no priorizado por ahora

- `1.7 HyperAuto (SDDI)`
- `2.6 Gotham Integration`
- `10.3 Apollo`
- `10.6 Geo-restricted enrollments`

Estas líneas siguen abiertas, pero no deberían adelantarse al hardening de seguridad, APIs, conectividad, infra y ML/AI.

## 2. Pendiente documental

- Alinear la foto rápida de [`backlog.md`](./backlog.md#foto-rápida) con el estado actual de [`checklist_status.md`](./checklist_status.md#resumen-por-dominio).
  Hoy el backlog sigue mostrando una foto vieja.

- Añadir una nota ejecutiva al principio de [`checklist_status.md`](./checklist_status.md).
  Conviene dejar explícito desde arriba que el golden path ya es real, pero el checklist total sigue lejos del cierre completo.

- Explicar mejor que “cerrado” en el roadmap significa “cerrado en alcance mínimo”.
  Ahora se entiende al leer varios documentos juntos, pero no queda suficientemente visible de un vistazo.
  Referencias: [`backlog.md`](./backlog.md), [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md)

- Marcar `P0` explícitamente como cerrado en [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md).
  `P1` a `P6` sí aparecen claramente como fases cerradas; `P0` quedó más ambiguo.

- Reescribir la sección de “próximos 90 días” del roadmap.
  Ya no refleja el estado real post-P6.
  Referencia: [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md)

- Añadir una fase nueva post-P6 o un bloque explícito de “hardening y amplitud”.
  El roadmap actual termina antes de reflejar la deuda que todavía queda viva.
  Referencias: [`backlog.md`](./backlog.md), [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md)

- Normalizar la nomenclatura de smoke tests y verificaciones por fase.
  Hay referencias válidas pero potencialmente confusas entre escenarios `p0`, `p1`, `post-P6`, etc.
  Referencias: [`checklist_status.md`](./checklist_status.md#método), [`roadmap_checklist_gaps.md`](./roadmap_checklist_gaps.md)

## Criterio práctico de cierre

Podemos considerar este bloque razonablemente saneado cuando:

- los `No` bajen de forma clara en conectividad, ML/AI, product delivery e infraestructura;
- los `Parcial` del golden path pasen mayoritariamente a `Cumple`;
- el roadmap deje de terminar en “alcance mínimo” y refleje una fase explícita de hardening posterior;
- backlog, roadmap y checklist vuelvan a contar la misma historia sin contradicciones.
