# Roadmap de Cierre de Gaps Críticos

Este documento prioriza los gaps más importantes detectados en [`checklist_status.md`](./checklist_status.md) y propone un orden razonable para cerrarlos. La idea no es “cubrir más superficie”, sino convertir la base actual en una plataforma creíble, testeable y menos dependiente de simulaciones.

## Principios de priorización

- Cerrar primero lo que desbloquea varios dominios a la vez.
- Sustituir simulaciones en el core antes de añadir nuevas features visibles.
- No escalar superficie de producto sin tests, CI y observabilidad mínimas.
- Priorizar flujos end-to-end reales sobre módulos aislados o demos parciales.
- Posponer capabilities satélite hasta que datos, runtime, seguridad y APIs estén estabilizados.

## Orden recomendado

| Prioridad | Fase | Qué se cierra | Secciones del checklist más afectadas |
| --- | --- | --- | --- |
| P0 | Fundación de ingeniería | Tests, CI, contratos API, observabilidad básica, rate limiting | Transversal, 7.3, 9.1, 9.2 |
| P1 | Core de datos real | Conectores, syncs, datasets, preview, branching usable | 1.1, 1.6, 9.1.2, 9.1.6 |
| P2 | Runtime de pipelines y streaming | Ejecución real, incrementalidad, scheduling, streaming con checkpoints, lineage útil | 1.2, 1.4, 1.5 |
| P3 | Ontology + Functions + Governance | Modelo semántico útil, actions seguras, runtime de functions, auditoría y aislamiento | 2.1, 2.2, 2.3, 2.4, 7.1, 7.2, 7.3 |
| P4 | APIs, SDKs y plataforma de desarrollo | APIs estables, OpenAPI publicado, SDKs, repos/CI reales, app-builder endurecido | 1.3, 4.*, 6.*, 9.* |
| P5 | AI/ML no sintético | LLM gateway real, agents ejecutables, RAG real, training/inference reales | 3.*, 8.2 |
| P6 | Analytics y enterprise hardening | Dashboards maduros, multi-org, control panel, despliegue enterprise | 5.*, 8.1, 8.3, 10.* |

## Fase P0: Fundación de ingeniería

### Por qué va primero

Sin esto, cualquier avance en conectividad, runtime o seguridad se apoya en una base frágil. Hoy el repo tiene señales claras de baja confianza operativa: tests casi inexistentes en Rust, mezcla de Vitest con specs E2E y varias superficies críticas sin verificación automática fuerte.

### Entregables críticos

- Pipeline CI real para el monorepo: `cargo fmt/check/test`, `pnpm check`, `pnpm test`, y E2E separados.
- Corregir la separación entre tests unitarios, integración y Playwright.
- Smoke tests end-to-end para el camino crítico:
  ingestar fuente -> dataset -> pipeline -> query -> audit.
- Versionado y validación de OpenAPI generada.
- Observabilidad base en servicios core:
  logs estructurados, health endpoints consistentes, errores tipados.
- Implementar `rate_limit.rs` en gateway y añadir pruebas.

### Definition of Done

- Cada PR relevante ejecuta CI real y falla si rompe el flujo principal.
- Existe al menos un test de integración por cada flujo crítico entre servicios.
- Web tests y E2E están desacoplados y verdes por separado.
- El gateway ya no tiene middlewares críticos en placeholder.

## Fase P1: Core de datos real

### Objetivo

Convertir la parte de conectividad y datasets en una base usable de verdad. Si esta fase no existe, casi todo lo demás sigue siendo una demo encima de datos sintéticos o poco fiables.

Estado a 2026-04-24: cerrado en el alcance mínimo de la fase. Ya hay conectores reales para PostgreSQL, REST API, CSV/JSON y Salesforce; `test_connection` real; sync jobs asíncronos con persistencia, retries y scheduling; preview enriquecido; branching con `merge/promote`; filesystem mínimo; y smoke end-to-end validado contra servicios vivos. Lo pendiente de aquí en adelante pasa a hardening incremental: más conectores enterprise, auto-registration, incrementalidad y dataset views.

### Entregables críticos

- Implementar conectores reales y bien soportados al menos para:
  PostgreSQL, REST API, archivos CSV/JSON y un conector enterprise prioritario.
- `test_connection` real con validación de credenciales, permisos y latencia básica.
- Sync engine asíncrono con persistencia, retries, estados y scheduling.
- Mejorar preview de datasets con muestra real, esquema, tipos, volumen y errores.
- Endurecer branching de datasets:
  merge/promote, conflictos básicos y reglas de concurrencia.
- Añadir filesystem navegable mínimo para datasets/proyectos si va a ser superficie pública.

### Definition of Done

- Una fuente externa real puede conectarse, validarse, sincronizarse y dejar evidencia auditable.
- Dataset resultante puede previsualizarse, versionarse y reusarse sin pasos manuales ocultos.
- Los conectores core tienen tests de integración y manejo de errores reales.

## Fase P2: Runtime de pipelines y streaming

### Objetivo

Sustituir la ejecución “parcial o simulada” por un runtime creíble para batch y streaming. Esta fase es la que convierte OpenFoundry en plataforma de procesamiento y no solo en un catálogo de servicios.

### Entregables críticos

- Runtime real para `sql`, `python` y `wasm`, con contratos claros de inputs/outputs.
- Scheduling y tracking de builds con estados reales.
- Incrementalidad mínima:
  detección de cambios, re-ejecución selectiva y metadata de build.
- Streaming real con al menos una fuente soportada de verdad, checkpoints y recuperación.
- Hot buffer + archivado cold storage funcional, aunque sea con alcance reducido.
- Push API para ingesta manual o programática.
- Lineage automático entre datasets, pipelines y streams.

### Definition of Done

- Existe un caso batch y un caso streaming que funcionan de extremo a extremo sin simulación.
- Los fallos dejan estados trazables y permiten reintento o recuperación.
- El lineage refleja dependencias reales, no solo metadata decorativa.

## Fase P3: Ontology, Functions y Governance

### Objetivo

Dar valor operativo real a la capa semántica. Hoy hay base útil en object types, links y actions simples, pero falta profundidad de modelo, runtime de negocio y enforcement.

### Entregables críticos

- Extender ontology con interfaces, shared properties y tipos dependientes del tiempo.
- Añadir acciones con permisos reales, batch apply y mejores validaciones.
- Crear runtime de Functions real para TypeScript y/o Python.
- Soportar object queries y traversals desde Functions.
- Endurecer Object Views / Explorer para objetos y relaciones reales.
- Cerrar mínimos de gobernanza:
  audit cross-service, approvals, markings básicos, aislamiento por organización.
- Completar SSO/OAuth/MFA con pruebas de flujo y políticas mínimas.

### Definition of Done

- Un usuario autorizado puede ejecutar una action/function sobre objetos reales con auditoría completa.
- Un usuario no autorizado queda bloqueado de forma verificable.
- La ontology ya soporta un caso de uso de negocio real, no solo CRUD de tipos.

## Fase P4: APIs, SDKs y plataforma de desarrollo

### Objetivo

Después de estabilizar el core, toca convertirlo en plataforma consumible por developers y equipos de producto.

### Entregables críticos

- APIs REST coherentes y documentadas para datasets, ontology, orchestration y admin.
- OpenAPI publicada como artefacto versionado.
- SDK oficial mínimo en TypeScript o Python generado desde OpenAPI y validado.
- Reescribir la parte sintética de code repositories:
  commits, diffs, repos y CI deben apoyarse en Git y ejecución reales.
- Endurecer Workshop:
  widget library mantenible, embeds útiles y menos componentes vacíos.
- Conectar Marketplace install con activación/despliegue real, aunque sea básico.

### Definition of Done

- Un desarrollador externo puede consumir una API estable o un SDK oficial y construir un flujo simple.
- El servicio de code repos deja de fabricar commits/diffs sintéticos en los caminos principales.
- Marketplace instala algo que realmente queda activado en la plataforma.

## Fase P5: AI/ML real

### Objetivo

Mover AI/ML desde simulación a producto útil. Esta fase debe llegar después del core porque depende de datos, APIs, auth, audit y runtimes ya maduros.

### Entregables críticos

- LLM gateway conectado a proveedores reales y con configuración por modelo.
- Embeddings y knowledge retrieval reales; no solo vectores deterministas locales.
- Agents con tool execution real sobre APIs/actions seguras.
- Evaluations y cost governance básicas.
- Entrenamiento e inferencia reales en `ml-service`.
- Registry/deployment de modelos con estados y versionado utilizables.

### Definition of Done

- Un caso RAG y un caso de agent execution funcionan con proveedores reales.
- Un modelo ML puede entrenarse o registrarse, desplegarse y predecir sin rutas sintéticas.
- AI y ML dejan trazabilidad, costes y errores observables.

## Fase P6: Analytics y hardening enterprise

### Objetivo

Terminar la capa de producto visible y endurecer el despliegue enterprise una vez que la base ya es confiable.

### Entregables críticos

- Madurar dashboards, analytics y geospatial sobre datasets/ontology reales.
- Priorizar Quiver/Contour antes que Notepad/Fusion si el foco es plataforma analítica.
- Construir Control Panel útil para branding, upgrades y administración.
- Endurecer multi-org sharing y Nexus.
- Validar despliegue self-hosted serio:
  Helm, autoscaling, HA, on-prem y operación restringida.

### Definition of Done

- Existe una experiencia analítica usable sobre datos reales.
- Hay historia de despliegue reproducible y operable fuera del entorno local.
- Multi-org y administración dejan de ser “base parcial”.

## Qué no priorizar todavía

Estas líneas existen en el checklist, pero hoy no deberían ir delante del core:

- `1.7 HyperAuto (SDDI)`
- `2.6 Gotham Integration`
- `5.4 Notepad`
- `5.5 Fusion`
- `10.3 Apollo`
- `10.6 Geo-restricted enrollments`
- `4.3.5+` extensiones IDE/MCP complejas mientras no haya SDKs/API estables

## Ruta sugerida para los próximos 90 días

### Mes 1

- Cerrar P0 completo.
- Elegir 1 caso de uso vertical que sirva como golden path.
- Definir contrato API de datasets, pipelines y audit.

### Mes 2

- Ejecutar P1.
- Tener al menos un conector real funcionando en producción interna.
- Endurecer preview/versionado de datasets.

### Mes 3

- Ejecutar P2 y arrancar P3.
- Sacar del modo simulación el flujo batch principal.
- Dejar preparado el primer caso streaming recuperable.

## Métrica de éxito global

El roadmap va bien si, al final de P3-P4, ya puedes demostrar este flujo sin simulación:

1. Conectar una fuente real.
2. Sincronizarla a un dataset versionado.
3. Procesarla con un pipeline real.
4. Exponerla vía query/API.
5. Operarla desde ontology/actions con permisos y auditoría.
6. Consumirla desde una app o SDK estable.

Si ese flujo no existe todavía, abrir más frentes de AI, analytics o marketplace probablemente solo añada complejidad sin cerrar los gaps estructurales.
