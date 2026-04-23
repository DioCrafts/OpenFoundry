# Palantir Foundry: Checklist Completa de Características y Servicios

> **Propósito de este documento:** Referencia exhaustiva de todas las capacidades de Palantir Foundry para validar si una implementación open source las cubre. Cada ítem incluye una casilla `[ ]` para marcar como cumplido.

***

## Resumen Ejecutivo

Palantir Foundry es una plataforma de operaciones de datos de nivel empresarial construida sobre una arquitectura de más de 300+ microservicios. Su diferenciador clave es el **Palantir Ontology**: una capa operacional que conecta activos digitales (datasets, modelos, pipelines) con sus contrapartes del mundo real, habilitando tanto análisis convencional como flujos de trabajo con IA. La plataforma se organiza en ocho grandes dominios de capacidad: Data Connectivity, Model Connectivity, Ontology Building, Use Case Development, Analytics, Product Delivery, Security & Governance, y Management & Enablement.[1][2][3][4]

***

## 1. CONECTIVIDAD E INTEGRACIÓN DE DATOS

### 1.1 Data Connection (Conectores y Fuentes)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.1.1 | Conectores nativos (200+) | Soporte out-of-the-box para sistemas enterprise como SAP, Salesforce, Oracle, etc.[5] | [ ] |
| 1.1.2 | Conector SAP (Foundry Connector 2.0) | Conector dedicado para SAP con actualizaciones regulares (v2.35.0 SP32)[6] | [ ] |
| 1.1.3 | Streaming data sources | Soporte para Kafka, Kinesis y otras fuentes de streaming con baja latencia[7] | [ ] |
| 1.1.4 | REST API source | Conector genérico para cualquier REST API con autenticación configurable[8] | [ ] |
| 1.1.5 | Generic source / custom connectors | Framework para conectores personalizados vía genéric source o REST API source[9] | [ ] |
| 1.1.6 | IoT / IIoT data sources | Integración con sistemas industriales y sensores IoT/IIoT[10] | [ ] |
| 1.1.7 | On-premises agent | Agente para conectar con fuentes en redes privadas o on-premises[11] | [ ] |
| 1.1.8 | Virtual tables (zero-copy) | Acceso in-place a tablas en Databricks, BigQuery, Snowflake sin mover datos[12] | [ ] |
| 1.1.9 | Auto-registration de tablas | Registro automático periódico de todas las tablas accesibles de una fuente[12] | [ ] |
| 1.1.10 | Bulk registration | Registro masivo de múltiples virtual tables a la vez[12] | [ ] |
| 1.1.11 | Update detection / versioning | Detección de cambios en fuentes externas (Delta, Iceberg) para builds incrementales[12] | [ ] |
| 1.1.12 | Export / egress controls | Controles de egress y políticas para exportar datos hacia sistemas externos[8] | [ ] |

### 1.2 Pipeline Builder

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.2.1 | Interfaz visual drag-and-drop | Constructor de pipelines punto a clic sin código[4] | [ ] |
| 1.2.2 | Transforms batch (PySpark) | Transformaciones Spark en Python para pipelines batch a gran escala[13] | [ ] |
| 1.2.3 | Transforms ligeros (Polars/Python) | Lightweight transforms composables con Spark; hasta 10x mejora en consumo[14] | [ ] |
| 1.2.4 | LLM-powered transforms | Clasificación, análisis de sentimiento, resumen, extracción de entidades, traducción via LLMs[1] | [ ] |
| 1.2.5 | External compute orchestration | Orquestación de compute externo (ej. Spark clusters externos) junto con pipelines nativos[15] | [ ] |
| 1.2.6 | Streaming pipelines | Pipelines de streaming con latencia < 15 segundos hasta el Ontology[16] | [ ] |
| 1.2.7 | Scheduling e integración de builds | Programación y orquestación de pipelines complejos multi-tecnología[15] | [ ] |
| 1.2.8 | Incremental transforms | Transforms incrementales que procesan solo datos nuevos/cambiados[12] | [ ] |
| 1.2.9 | Multi-language pipelines | Pipelines que combinan diferentes lenguajes y motores de compute[14] | [ ] |
| 1.2.10 | AI Assist en Pipeline Builder | Asistente de IA integrado que sugiere transformaciones y acciones[1] | [ ] |

### 1.3 Code Repositories

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.3.1 | Web-based IDE | Entorno de autoría de código en browser con soporte Git[4] | [ ] |
| 1.3.2 | Control de versiones (Git) | Branching, merging, pull requests, protected branches[17] | [ ] |
| 1.3.3 | CI/CD integrado (ci/foundry-publish) | Pipeline de publicación continua integrado en el flujo de PR[17] | [ ] |
| 1.3.4 | Soporte TypeScript v2 | Funciones en TypeScript v2 con autenticación first-class[18] | [ ] |
| 1.3.5 | Soporte Python | Funciones y transforms en Python con todas las librerías del ecosistema[18] | [ ] |
| 1.3.6 | Plantillas de repositorio | Templates predefinidos (Model Training, Functions, etc.)[19] | [ ] |
| 1.3.7 | Libraries side panel | Gestión de dependencias/librerías directamente desde el IDE[18] | [ ] |

### 1.4 Streaming

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.4.1 | Stream creation con schema | Definición de streams con schema tipado (String, Double, Timestamp, etc.)[20] | [ ] |
| 1.4.2 | Hot buffer + cold storage archiving | Datos del stream archivados a cold storage cada pocos minutos como dataset estándar[21] | [ ] |
| 1.4.3 | Fault tolerance con checkpoints | Tolerancia a fallos con checkpoints periódicos para restart sin reprocesar[21] | [ ] |
| 1.4.4 | Job graph visualization | Representación visual del pipeline de streaming (job graph)[21] | [ ] |
| 1.4.5 | Streaming syncs desde fuentes externas | Ingesta de Kafka, Kinesis y otras plataformas de streaming[7] | [ ] |
| 1.4.6 | Transform de streams en Pipeline Builder | Transformación de streams en tiempo real directamente en Pipeline Builder[20] | [ ] |
| 1.4.7 | Push manual via API | Ingesta de records vía curl/OAuth para testing y producción[20] | [ ] |

### 1.5 Data Lineage

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.5.1 | Grafo interactivo de linaje | Vista gráfica de cómo fluyen los datos a través de la plataforma[22] | [ ] |
| 1.5.2 | Data Lineage (datasets) | Trazabilidad completa de origen y transformaciones de cada dataset[23] | [ ] |
| 1.5.3 | Workflow Lineage (GA) | Linaje de ejecución: cómo interactúan objetos, acciones, apps y automatizaciones[24] | [ ] |
| 1.5.4 | Upstream/downstream impact analysis | Ver qué datasets se verán afectados antes de hacer cambios[24] | [ ] |
| 1.5.5 | Builds desde Data Lineage | Trigger de builds directamente desde la vista de linaje[25] | [ ] |
| 1.5.6 | Propagación de markings por linaje | Los markings de seguridad se heredan automáticamente a datasets derivados[26] | [ ] |

### 1.6 Datasets y Filesystem

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.6.1 | Dataset con transacciones | Historial de transacciones (snapshot, append, delete) sobre cada dataset[27] | [ ] |
| 1.6.2 | Branching de datasets | Branches por entorno (dev/prod) con el mismo patrón que Git[27] | [ ] |
| 1.6.3 | Dataset Views | Vistas de datasets sin duplicar datos[27] | [ ] |
| 1.6.4 | Dataset Preview | Visualización de contenido e historial/metadatos de un dataset[4] | [ ] |
| 1.6.5 | Data Health checks | Definición y monitoreo de health checks sobre la calidad de datasets[4] | [ ] |
| 1.6.6 | Filesystem navegable | Sistema de ficheros jerárquico (proyectos, carpetas, archivos)[27] | [ ] |
| 1.6.7 | Linter / anti-patterns detector | Análisis del enrollment para detectar anti-patrones y optimizar recursos[4] | [ ] |

### 1.7 HyperAuto (SDDI)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 1.7.1 | Generación automática de pipelines ERP | Genera pipelines end-to-end sobre sistemas ERP sin desarrollo manual[4] | [ ] |
| 1.7.2 | Generación de Ontology desde ERP | Crea el Ontology a partir de sistemas enterprise automáticamente[4] | [ ] |

***

## 2. ONTOLOGY (CAPA SEMÁNTICA Y OPERACIONAL)

### 2.1 Ontology Manager — Tipos Semánticos

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.1.1 | Object Types | Definición de tipos de objeto con propiedades tipadas y metadata rico[3] | [ ] |
| 2.1.2 | Link Types | Relaciones entre tipos de objeto (uni/bidireccionales)[3] | [ ] |
| 2.1.3 | Properties con Value Types | Tipos de valor con restricciones y contexto adicional embebido[1] | [ ] |
| 2.1.4 | Interfaces / Polimorfismo | Interfaces que describen la forma de un object type para polimorfismo[3] | [ ] |
| 2.1.5 | Shared Property Types | Propiedades compartidas entre múltiples object types[28] | [ ] |
| 2.1.6 | Time-dependent properties | Propiedades que varían en el tiempo (time series nativas)[29] | [ ] |
| 2.1.7 | Geo-point properties | Soporte nativo para coordenadas geoespaciales en object types[28] | [ ] |
| 2.1.8 | Media references | Referencias a imágenes, vídeos y otros medios en objetos[1] | [ ] |
| 2.1.9 | Semantic search (unstructured data) | Búsqueda semántica para desbloquear datos no estructurados[1] | [ ] |
| 2.1.10 | Digital twin / espejo del mundo real | El Ontology como gemelo digital de la organización[3] | [ ] |

### 2.2 Action Types (Kinética del Ontology)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.2.1 | Action Types (formularios punto a clic) | Configuración de acciones con interfaz visual sin código[1] | [ ] |
| 2.2.2 | Function-backed Actions | Acciones con lógica arbitraria definida en Functions (TypeScript/Python)[30] | [ ] |
| 2.2.3 | Ontology Edits TypeScript API | API tipada para crear/editar/borrar objetos y links desde Functions[30] | [ ] |
| 2.2.4 | Batch apply actions | Aplicar hasta 20 (o más) acciones del mismo tipo en una sola llamada[31] | [ ] |
| 2.2.5 | Action validation | Validar parámetros de una acción antes de ejecutarla[32] | [ ] |
| 2.2.6 | Object Storage V2 (escritura inmediata) | Ediciones visibles inmediatamente tras completar la acción[31] | [ ] |
| 2.2.7 | Webhook / External system actions | Acciones que orquestan cambios en sistemas externos[1] | [ ] |
| 2.2.8 | Permisos granulares por Action | Control de quién puede ejecutar cada acción bajo qué condiciones[1] | [ ] |
| 2.2.9 | Scenario / what-if branching | Proyectar consecuencias de un cambio en un branch del Ontology (sandbox)[1] | [ ] |

### 2.3 Functions (Lógica de Negocio)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.3.1 | Functions en TypeScript v2 | Lógica de negocio tipada en TypeScript con soporte de autenticación[18] | [ ] |
| 2.3.2 | Functions en Python | Funciones en Python con acceso al Ontology y plataforma SDK[18] | [ ] |
| 2.3.3 | Object Set Queries | Consultas sobre conjuntos de objetos (filtros, agregaciones)[30] | [ ] |
| 2.3.4 | Link Traversals | Navegación por las relaciones entre objetos del Ontology[30] | [ ] |
| 2.3.5 | External Functions | Llamadas a sistemas externos en tiempo real desde acciones y workflows[1] | [ ] |
| 2.3.6 | Platform SDK en Functions | Acceso a APIs de la plataforma (schedules, media sets, etc.) desde Functions[18] | [ ] |
| 2.3.7 | LLM en Functions (Language Model Service) | Llamadas a LLMs directamente desde Functions para lógica IA[18] | [ ] |

### 2.4 Object Views, Explorer y Vertex

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.4.1 | Object Views | Representación canónica / UI estándar para mostrar un tipo de objeto[4] | [ ] |
| 2.4.2 | Object Explorer | Exploración y búsqueda visual del Ontology completo[4] | [ ] |
| 2.4.3 | Vertex — System graphs | Grafos de objetos relacionados para análisis de sistemas[4] | [ ] |
| 2.4.4 | Vertex — Simulaciones end-to-end | Ejecución de simulaciones usando modelos sobre los grafos del sistema[4] | [ ] |

### 2.5 Foundry Rules y Machinery

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.5.1 | Foundry Rules | Definición y aplicación de reglas de negocio complejas sobre datasets, objetos y series temporales[4] | [ ] |
| 2.5.2 | Machinery (Process Mining) | Comprensión y gestión de procesos, identificación de comportamientos no deseados[4] | [ ] |
| 2.5.3 | Machinery widget de monitoreo | Widget de análisis y monitoreo en tiempo real de procesos Machinery[15] | [ ] |
| 2.5.4 | Dynamic Scheduling | Optimización de scheduling con ML y gestión de restricciones del Ontology[4] | [ ] |

### 2.6 Gotham Integration

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 2.6.1 | Type mapping Foundry ↔ Gotham | Representación unificada del Ontology entre Foundry y Gotham[28] | [ ] |
| 2.6.2 | Object Set Service | Servicio backend que soporta búsqueda, filtrado y agregación de objetos para Gotham[28] | [ ] |

***

## 3. CONECTIVIDAD Y DESARROLLO DE MODELOS (MLOps)

### 3.1 Model Assets y Modeling Objectives

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 3.1.1 | Modeling Objectives | Gestión del ciclo de vida completo de un problema ML (colaboración, documentación, deployment)[33] | [ ] |
| 3.1.2 | Model development in-platform | Entrenamiento de modelos con scikit-learn, TensorFlow, OR-tools, etc.[33] | [ ] |
| 3.1.3 | Import de modelos externos | Integración de modelos entrenados fuera de la plataforma (containers, librerías, APIs)[33] | [ ] |
| 3.1.4 | Batch deployment | Despliegue batch: transform que corre inferencia sobre un dataset de entrada[34] | [ ] |
| 3.1.5 | Live/online deployment | Despliegue en tiempo real de modelos envueltos en Functions para Workshop[35] | [ ] |
| 3.1.6 | Model adapters | Abstracción de la lógica del modelo independientemente del framework[1] | [ ] |
| 3.1.7 | Versioning y reproducibilidad | Cada modelo está vinculado al código, datos y entorno de entrenamiento[19] | [ ] |
| 3.1.8 | Governance y audit trail de modelos | Registro gobernado de cómo se produjo cada modelo[33] | [ ] |
| 3.1.9 | Staging y release to production | Flujo de release: staging → production con número de versión[34] | [ ] |
| 3.1.10 | ML feedback loops | Loops de feedback desde datos de producción, outcomes y acciones de usuario[33] | [ ] |
| 3.1.11 | MLflow integration | Integración con MLflow para tracking de métricas, image logging y vistas de parámetros[36] | [ ] |
| 3.1.12 | Marketplace de modelos (DevOps) | Packaging y distribución de modelos como productos via Marketplace[37] | [ ] |
| 3.1.13 | Compute Modules (containers serverless) | Despliegue de containers Docker serverless con escalado horizontal[4] | [ ] |

### 3.2 AIP — Language Model Service

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 3.2.1 | Interfaz unificada multi-LLM | Interfaz unificada para GPT-4, Claude, Gemini, Grok, y otros modelos[38] | [ ] |
| 3.2.2 | LLM en redes privadas | Modelos LLM operando dentro de redes privadas del cliente[39] | [ ] |
| 3.2.3 | Multimodal / Vision-Language Models | Soporte para modelos de visión y aplicaciones móviles[1] | [ ] |
| 3.2.4 | LLM cost governance | Tracking de consumo y costes de los LLMs[23] | [ ] |
| 3.2.5 | Evaluations (benchmarking LLMs) | Suite de evaluación para medir y comparar rendimiento de modelos LLM[40] | [ ] |

### 3.3 AIP Agent Studio

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 3.3.1 | AIP Agents con herramientas del Ontology | Agentes LLM con acceso a datos y herramientas del Ontology[41] | [ ] |
| 3.3.2 | Deploy interno (Workshop widget) | Agentes deployables en Workshop via AIP Interactive widget[41] | [ ] |
| 3.3.3 | Deploy externo (OSDK / APIs) | Agentes accesibles desde aplicaciones externas via OSDK y APIs[41] | [ ] |
| 3.3.4 | Agents como Functions (para Automate) | Publicación de agentes como funciones para usar en Automate[41] | [ ] |
| 3.3.5 | Tool use / Function calling | Los LLMs pueden invocar herramientas y trazabilidad de la secuencia de llamadas[38] | [ ] |
| 3.3.6 | AI FDE (natural language platform ops) | Operar Foundry en lenguaje natural para todos los usuarios[15] | [ ] |

### 3.4 AIP Logic y Automate

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 3.4.1 | AIP Logic (LLM decision logic builder) | Entorno de desarrollo de lógica de decisión con LLMs[42] | [ ] |
| 3.4.2 | Automate (event-driven triggers) | Automatizaciones disparadas por cambios en el Ontology[4] | [ ] |
| 3.4.3 | Automate — notificaciones | Envío de notificaciones cuando se cumplen condiciones sobre los datos[4] | [ ] |
| 3.4.4 | Automate — submit Actions automáticas | Ejecución automática de acciones cuando se cumplen condiciones[4] | [ ] |
| 3.4.5 | Proposal-based pattern (human-in-the-loop) | Agentes que generan propuestas para revisión humana antes de aplicar cambios[1] | [ ] |

***

## 4. DESARROLLO DE CASOS DE USO (APLICACIONES)

### 4.1 Workshop (No-code / Low-code App Builder)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 4.1.1 | Punto a clic (drag-and-drop) | Constructor de aplicaciones visual sin código[4] | [ ] |
| 4.1.2 | Pro-code customizations | Soporte para personalizaciones en código dentro del builder[4] | [ ] |
| 4.1.3 | Widget library (continuamente actualizada) | Biblioteca de widgets prediseñados integrada[43] | [ ] |
| 4.1.4 | AIP Interactive widget (agent embed) | Widget para embeber agentes AIP en aplicaciones Workshop[41] | [ ] |
| 4.1.5 | Scenario / what-if en Workshop | Soporte nativo de escenarios para workflows "¿Qué pasaría si...?"[1] | [ ] |
| 4.1.6 | Embedded Quiver dashboards | Incrustar dashboards de Quiver dentro de apps Workshop[44] | [ ] |
| 4.1.7 | Embedded Map | Incrustar componente de mapa geoespacial[45] | [ ] |
| 4.1.8 | Consumer mode (usuarios externos B2C/B2B) | Modo seguro para entregar apps a usuarios externos sin acceso a la plataforma[46] | [ ] |

### 4.2 Slate (Pro-code App Builder)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 4.2.1 | HTML / CSS / JavaScript custom apps | Aplicaciones completamente personalizadas con tecnologías web estándar[47] | [ ] |
| 4.2.2 | Integración con Ontology layer | Acceso nativo al Ontology desde aplicaciones Slate[47] | [ ] |
| 4.2.3 | Acceso directo a datasets | Slate puede interactuar directamente con datasets además del Ontology[48] | [ ] |
| 4.2.4 | Drag-and-drop + código | Interfaz mixta: visual para estructura, código para personalización[47] | [ ] |

### 4.3 OSDK React Applications

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 4.3.1 | React UI con OSDK como backend | Aplicaciones React usando Foundry como backend via OSDK[4] | [ ] |
| 4.3.2 | TypeScript bindings type-safe | Bindings TypeScript con type-safety para el Ontology[49] | [ ] |
| 4.3.3 | Soporte NPM, Pip/Conda, Maven | SDK disponible para TypeScript, Python, Java y OpenAPI spec[49] | [ ] |
| 4.3.4 | Developer Console | Portal para crear y gestionar aplicaciones OSDK[49] | [ ] |
| 4.3.5 | VS Code Workspaces in-platform | IDE VS Code integrado con soporte nativo para Python transforms y OSDK React[4] | [ ] |
| 4.3.6 | Palantir extension for VS Code | Extensión VS Code con acceso a repositorios, builds y AI coding assistant[4] | [ ] |
| 4.3.7 | Palantir MCP (Model Context Protocol) | Conecta IDEs de IA externas con el Ontology y herramientas de Foundry[4] | [ ] |
| 4.3.8 | Ontology MCP | Expone recursos del Ontology como herramientas MCP para agentes externos[4] | [ ] |

### 4.4 Workflow Building

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 4.4.1 | Automate (ver sección AIP) | — | [ ] |
| 4.4.2 | Solution Designer | Herramienta interactiva para diseñar arquitecturas de soluciones[4] | [ ] |
| 4.4.3 | Carbon (curated workspaces) | Combina apps y recursos en workspaces curados para usuarios finales[4] | [ ] |
| 4.4.4 | Use Case app | App de gestión de casos de uso con Workshop, Slate y Quiver integrados[50] | [ ] |

***

## 5. ANALYTICS

### 5.1 Contour (Top-down Analysis)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.1.1 | Exploración top-down visual | Análisis punto a clic sobre datasets a escala[51] | [ ] |
| 5.1.2 | Transform boards (joins, filtros, agregaciones) | Transformaciones step-by-step con recálculo automático downstream[52] | [ ] |
| 5.1.3 | Display boards (gráficos, tablas) | Visualizaciones de resultados: histogramas, scatter, tablas, etc.[52] | [ ] |
| 5.1.4 | Paths y secuencias de análisis | Análisis organizados en paths con boards en serie[51] | [ ] |
| 5.1.5 | Parámetros de análisis | Parámetros para cambiar vistas (rangos de fechas, IDs, localizaciones)[51] | [ ] |
| 5.1.6 | Dashboards con chart-to-chart filtering | Dashboards interactivos con filtrado cruzado entre gráficos[51] | [ ] |
| 5.1.7 | Export a dataset (materialización) | Guardar resultados de Contour como nuevo dataset en Foundry[51] | [ ] |
| 5.1.8 | Export PDF | Exportación de dashboards a PDF[51] | [ ] |
| 5.1.9 | Fullscreen presentation view | Modo presentación en pantalla completa para dashboards[51] | [ ] |

### 5.2 Quiver (Time Series y Ontology Analytics)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.2.1 | Análisis de time series | Visualización y cálculo de KPIs sobre datos temporales[29] | [ ] |
| 5.2.2 | Point-and-click sin código | Transformaciones y análisis sin escribir código[44] | [ ] |
| 5.2.3 | Navegación de relaciones entre object types | Traversal y análisis de objetos relacionados del Ontology[44] | [ ] |
| 5.2.4 | Joins entre object sets | Agregación y análisis sobre múltiples object sets[44] | [ ] |
| 5.2.5 | Visual functions (bloques de lógica reutilizables) | Funciones visuales reutilizables para simplificar análisis[44] | [ ] |
| 5.2.6 | Dashboards interactivos y paramétricos | Dashboards con filtros dinámicos publicables para stakeholders[44] | [ ] |
| 5.2.7 | Embed en Workshop, Object Views, Carbon | Incrustar análisis Quiver en otros contextos de la plataforma[44] | [ ] |
| 5.2.8 | Vega plots | Soporte para gráficos Vega avanzados[44] | [ ] |

### 5.3 Map (Geospatial Analysis)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.3.1 | Análisis geoespacial y temporal | Canvas geoespacial con datos del Ontology sobre capas de mapa[45] | [ ] |
| 5.3.2 | Track analysis (movimiento histórico) | Visualización de trayectorias y movimiento de activos móviles en el tiempo[45] | [ ] |
| 5.3.3 | Raster imagery y capas GIS | Soporte para imagery satelital y capas geoespaciales externas[45] | [ ] |
| 5.3.4 | Color/style por valor de dato | Visualización diferenciada con colores y estilos basados en propiedades del objeto[45] | [ ] |
| 5.3.5 | Combinación con time series y sensores | Análisis conjunto de datos IoT, series temporales y eventos sobre el mapa[45] | [ ] |
| 5.3.6 | Standalone o embebido en Workshop | Usable como análisis independiente o componente en Workshop[45] | [ ] |

### 5.4 Notepad (Collaborative Documents)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.4.1 | Editor de texto enriquecido colaborativo | Documentos colaborativos con formato rico (texto, imágenes)[53] | [ ] |
| 5.4.2 | Embed de widgets de Contour, Quiver, etc. | Integración de gráficos y tablas de otras apps directamente en el documento[53] | [ ] |
| 5.4.3 | Templates de Notepad | Plantillas como blueprints para generar nuevos documentos[53] | [ ] |
| 5.4.4 | Export / print de documentos | Exportación e impresión de documentos Notepad[53] | [ ] |
| 5.4.5 | Indexado por AIP Assist | Los documentos pueden ser indexados y consultados via AIP Assist (RAG)[54] | [ ] |
| 5.4.6 | Marketplace support | Incluir documentos y templates Notepad en productos Marketplace[55] | [ ] |

### 5.5 Fusion (Spreadsheet Bidireccional)

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.5.1 | Spreadsheet editable sincronizado con dataset | Sync bidireccional: editar en spreadsheet → actualiza dataset en Foundry[4] | [ ] |
| 5.5.2 | Query de datos del Ontology en spreadsheet | Mostrar y consultar datos del Ontology en formato de hoja de cálculo[4] | [ ] |

### 5.6 Code Workspaces y Code Workbook

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.6.1 | JupyterLab® integrado | Entorno Jupyter nativo en la plataforma[4] | [ ] |
| 5.6.2 | RStudio® Workbench integrado | RStudio nativo para workflows de estadística y R[4] | [ ] |
| 5.6.3 | LLMs en notebooks | Acceso a modelos LLM directamente desde notebooks[42] | [ ] |
| 5.6.4 | Code Workbook (legacy) | Entorno web de análisis en código con workflows de data science[4] | [ ] |

### 5.7 Integraciones BI Externas

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 5.7.1 | Conector Tableau | Conector dedicado para Tableau[1] | [ ] |
| 5.7.2 | Conector Power BI | Conector dedicado para Power BI[1] | [ ] |
| 5.7.3 | ODBC/JDBC drivers | Drivers estándar para conectar herramientas SQL externas[1] | [ ] |
| 5.7.4 | Python SDK | SDK Python para acceso programático a datos y Ontology[56] | [ ] |
| 5.7.5 | REST API (Foundry API) | API REST con OAuth 2.0 para construir aplicaciones sobre la plataforma[57] | [ ] |

***

## 6. PRODUCT DELIVERY (DevOps y Marketplace)

### 6.1 Foundry DevOps

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 6.1.1 | Packaging de productos | Crear "productos" con colecciones de recursos (pipelines, Ontology, apps, modelos)[58] | [ ] |
| 6.1.2 | Release channels / versioning | Etiquetar versiones del producto con canales de release[58] | [ ] |
| 6.1.3 | Gestión de instalaciones (fleet) | Administrar una flota de instalaciones con upgrades automáticos[58] | [ ] |
| 6.1.4 | Maintenance windows | Configuración de ventanas de mantenimiento para actualizaciones[46] | [ ] |
| 6.1.5 | Foundry Branching (beta) | Branching a nivel de enrollment para desarrollo de features[59] | [ ] |

### 6.2 Marketplace

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 6.2.1 | Storefront de productos | Descubrimiento e instalación de productos publicados[4] | [ ] |
| 6.2.2 | Guided installation | Instalación guiada con configuración personalizable[4] | [ ] |
| 6.2.3 | Recommended products | Recomendaciones de productos relacionados[4] | [ ] |
| 6.2.4 | Starter packs / ejemplos | Workflow starter packs para arranque rápido[4] | [ ] |
| 6.2.5 | Instalaciones multi-space | Instalación del mismo producto en múltiples spaces[46] | [ ] |

***

## 7. SEGURIDAD Y GOBERNANZA

### 7.1 Control de Acceso

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 7.1.1 | Role-based access control (RBAC) | Control de acceso discrecional basado en roles[60] | [ ] |
| 7.1.2 | Markings (mandatory access controls) | Controles mandatorios que viajan con los datos independientemente de la ubicación[61][62] | [ ] |
| 7.1.3 | Propagación de markings por linaje | Los markings se heredan automáticamente a todos los recursos derivados[26] | [ ] |
| 7.1.4 | Classification-based access controls (CBAC) | Controles de clasificación para información gubernamental sensible[63] | [ ] |
| 7.1.5 | Scoped sessions | Usuarios pueden seleccionar un subconjunto de markings por sesión[62] | [ ] |
| 7.1.6 | Organization-level isolation | Silos estrictos entre organizaciones dentro de un mismo enrollment[64] | [ ] |
| 7.1.7 | Guest access cross-organization | Acceso de invitado controlado entre organizaciones[65] | [ ] |
| 7.1.8 | Restricted views | Vistas granulares que limitan columnas/filas visibles[66] | [ ] |
| 7.1.9 | Consumer mode (external users) | Entrega segura de apps a usuarios externos sin acceso a la plataforma[46] | [ ] |

### 7.2 Autenticación y Cifrado

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 7.2.1 | Single Sign-On (SSO / SAML 2.0) | Integración con identity providers via SAML[67] | [ ] |
| 7.2.2 | Multi-factor authentication (MFA) | Autenticación de múltiples factores[60] | [ ] |
| 7.2.3 | OAuth 2.0 (client credentials, auth code) | Flujos OAuth para server-to-server y autenticación interactiva[68] | [ ] |
| 7.2.4 | Encryption in transit y at rest | Cifrado completo de datos en tránsito y en reposo[39] | [ ] |
| 7.2.5 | Zero-trust security architecture | Infraestructura zero-trust con node cycling agresivo[2] | [ ] |

### 7.3 Governance y Privacidad

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 7.3.1 | Audit logging completo | Registro de todas las interacciones del sistema y decisiones[23] | [ ] |
| 7.3.2 | Approvals (change management) | Flujo de aprobación para cambios sensibles (propuestas, acceso, ontology)[4] | [ ] |
| 7.3.3 | Checkpoint (justification prompts) | Prompts de justificación para interacciones con datos sensibles[4] | [ ] |
| 7.3.4 | Cipher (cryptographic operations) | Servicio de cifrado/descifrado/hashing con gestión de algoritmos y claves[4] | [ ] |
| 7.3.5 | Sensitive Data Scanner (SDS) | Descubrimiento automático de datos sensibles con acciones de remediación[4] | [ ] |
| 7.3.6 | Data Lifetime / retention policies | Políticas de retención lineage-aware con deletion gobernado[4] | [ ] |
| 7.3.7 | Compliances: HIPAA, GDPR, ITAR | Alineación con marcos regulatorios globales[4] | [ ] |
| 7.3.8 | Project templates para governance estándar | Templates que estandarizan la gobernanza a escala[66] | [ ] |

***

## 8. MANAGEMENT Y ENABLEMENT

### 8.1 Control Panel y Administración

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 8.1.1 | Control Panel centralizado | Suite completa de governance, gestión de recursos y administración de seguridad[4] | [ ] |
| 8.1.2 | Enrollment vs Organization permissions | Permisos gestionados en dos niveles independientes[69] | [ ] |
| 8.1.3 | Resource Management | Insights granulares sobre utilización de recursos del enrollment[4] | [ ] |
| 8.1.4 | Upgrade Assistant | Herramienta para gestionar actualizaciones de la plataforma[4] | [ ] |
| 8.1.5 | Identity provider mapping (SAML org assignment) | Asignación automática de usuarios a organizaciones via SAML[65] | [ ] |
| 8.1.6 | Custom platform branding | Personalización de la experiencia de plataforma con branding organizacional[4] | [ ] |

### 8.2 Enablement y Documentación

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 8.2.1 | AIP Assist (platform-wide chatbot) | Asistente IA disponible en toda la plataforma para navegar y generar valor[70] | [ ] |
| 8.2.2 | Custom documentation in-platform | Creación y gestión de documentación interna indexada por AIP Assist (RAG)[54] | [ ] |
| 8.2.3 | Walkthroughs (tutoriales interactivos) | Tutoriales step-by-step customizables para onboarding de usuarios[4] | [ ] |

### 8.3 Multi-Organization Ecosystems

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 8.3.1 | Private + shared spaces | Construcción de ecosistemas con espacios privados y compartidos[71] | [ ] |
| 8.3.2 | Data sharing controlado entre orgs | Compartición de datos con controles de auditoría y marcings mandatorios[71] | [ ] |
| 8.3.3 | Host organization + partners | Modelo de ecosistema con organización host y partners[71] | [ ] |

***

## 9. DEVELOPER TOOLCHAIN (APIs y SDKs)

### 9.1 APIs REST

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 9.1.1 | Foundry Platform API (v1 y v2) | API REST completa para interactuar con la plataforma (datasets, ontology, filesystem, etc.)[57] | [ ] |
| 9.1.2 | Datasets API | CRUD completo sobre datasets, branches, transacciones, archivos[72] | [ ] |
| 9.1.3 | Ontologies API (Objects, Links, Actions) | API para consultar objetos, navegar links, aplicar acciones[32] | [ ] |
| 9.1.4 | Orchestration API (Builds, Jobs, Schedules) | API para gestionar pipelines y schedules programáticamente[6] | [ ] |
| 9.1.5 | Streams API (real-time, second latency) | API para análisis y procesamiento de datos en tiempo real[6] | [ ] |
| 9.1.6 | Connectivity API (external systems) | API para gestionar conexiones a sistemas externos[6] | [ ] |
| 9.1.7 | Filesystem API (folders, projects) | API para gestionar el filesystem de recursos[6] | [ ] |
| 9.1.8 | SQL Queries API | API para ejecutar queries SQL sobre datasets[6] | [ ] |
| 9.1.9 | Admin API (Users, Groups, Markings, Orgs) | API para gestión administrativa de usuarios, grupos y markings[6] | [ ] |

### 9.2 SDKs

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 9.2.1 | Foundry Platform SDK (Python) | SDK Python oficial sobre la Foundry API[56] | [ ] |
| 9.2.2 | OSDK (TypeScript/NPM) | SDK del Ontology para TypeScript/React[49] | [ ] |
| 9.2.3 | OSDK (Python/Pip) | SDK del Ontology para Python[49] | [ ] |
| 9.2.4 | OSDK (Java/Maven) | SDK del Ontology para Java[49] | [ ] |
| 9.2.5 | OpenAPI spec (any language) | Especificación OpenAPI para integrar cualquier lenguaje[49] | [ ] |

***

## 10. INFRAESTRUCTURA Y DEPLOYMENT

| # | Característica | Descripción | ✅ Cumplido |
|---|---|---|---|
| 10.1 | SaaS multi-cloud (AWS, Azure, GCP, OCI) | Despliegue como SaaS sobre múltiples clouds[11] | [ ] |
| 10.2 | On-premises / air-gapped deployment | Soporte para entornos air-gapped con agent on-premises[11] | [ ] |
| 10.3 | Apollo (CI/CD autónomo) | Plataforma de entrega continua que gestiona 300+ microservicios automáticamente[2] | [ ] |
| 10.4 | Kubernetes autoscaling build system | Sistema de builds basado en Kubernetes con autoescalado[1] | [ ] |
| 10.5 | High availability / autoscaling compute mesh | Compute mesh altamente disponible con autoescalado[2] | [ ] |
| 10.6 | Geo-restricted enrollments | Soporte para restricciones geográficas de datos y compute[59] | [ ] |

***

## Resumen de Áreas por Dominio

| Dominio | Total Ítems | Complejidad |
|---|---|---|
| 1. Data Connectivity e Integración | ~45 | Alta |
| 2. Ontology | ~35 | Muy alta |
| 3. ML / AIP | ~30 | Alta |
| 4. Aplicaciones (Workshop/Slate/OSDK) | ~30 | Media-Alta |
| 5. Analytics (Contour/Quiver/Map/etc.) | ~35 | Media |
| 6. Product Delivery (DevOps/Marketplace) | ~10 | Media |
| 7. Seguridad y Gobernanza | ~25 | Alta |
| 8. Management y Enablement | ~10 | Media |
| 9. APIs y SDKs | ~15 | Alta |
| 10. Infraestructura | ~6 | Muy alta |
| **TOTAL** | **~241 características** | |

***

## Notas de Priorización para una Implementación Open Source

Las características se pueden priorizar en tres capas para una implementación open source:

**Capa 1 — Core (Bloqueante para viabilidad):**
- Ontology (Objects, Links, Actions, Functions)
- Dataset / Filesystem con branching y transacciones
- Pipeline Builder batch (PySpark/Python)
- RBAC + Markings básicos
- REST API + OSDK básico

**Capa 2 — Funcional (Para uso real):**
- Streaming pipelines
- Model deployment (batch y live)
- Workshop / app builder básico
- Contour y Quiver básicos
- Audit logging y data lineage

**Capa 3 — Avanzada (Enterprise / diferenciadora):**
- AIP Agent Studio y LLM integration
- HyperAuto (ERP auto-generation)
- DevOps y Marketplace
- CBAC, Cipher, Sensitive Data Scanner
- Multi-organization ecosystems
- Apollo (autonomous CI/CD)