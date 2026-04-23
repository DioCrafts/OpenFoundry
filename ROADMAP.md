<div align="center">

# 🗺️ OpenFoundry Roadmap

### From zero to full Palantir Foundry parity — open source.

*Last updated: April 2026*

</div>

---

## Overview

OpenFoundry aims to deliver **25 core capabilities** that match Palantir Foundry — all open-source, self-hosted, and community-driven. This roadmap outlines our phased approach to get there.

### Status Legend

| Icon | Meaning |
|------|---------|
| ✅ | Done — feature shipped and usable |
| 🚧 | In Progress — actively being built |
| 📐 | Designed — architecture defined, implementation pending |
| 🔲 | Planned — scoped but not yet started |
| 💡 | Exploring — researching approaches |

### Priority Legend

| Tag | Meaning |
|-----|---------|
| 🔴 **Critical** | Core platform value — blocks adoption |
| 🟡 **High** | Key differentiator — needed for production use |
| 🟠 **Medium** | Important — completes the platform story |
| 🟢 **Low** | Nice to have — enhances ecosystem |

---

## 📊 Parity Tracker: 24/25 Foundry Components

| # | Foundry Component | OpenFoundry Service | Status | Target Phase |
|---|---|---|---|---|
| 1 | Ontology | `ontology-service` | ✅ Done | Phase 1 |
| 2 | Transforms / Pipeline Builder | `pipeline-service` | ✅ Done | Phase 1 |
| 3 | Data Connections | `data-connector` | ✅ Done | Phase 1 |
| 4 | Contour (Visual Analytics) | `query-service` | ✅ Done | Phase 1 |
| 5 | Dataset Management & Versioning | `dataset-service` | ✅ Done | Phase 1 |
| 6 | Data Lineage | `pipeline-service/lineage` | ✅ Done | Phase 1 |
| 7 | Notebooks / Code Workbooks | `notebook-service` | ✅ Done | Phase 1 |
| 8 | Quiver (Dashboards) | Frontend components | ✅ Done | Phase 2 |
| 9 | Object Explorer | `ontology-service` | ✅ Done | Phase 1 |
| 10 | Auth / RBAC / SSO | `auth-service` | 🚧 In Progress | Phase 2 |
| 11 | Workflows / Actions | `workflow-service` | ✅ Done | Phase 2 |
| 12 | Notifications | `notification-service` | ✅ Done | Phase 2 |
| 13 | Data Catalog | `dataset-service/catalog` | ✅ Done | Phase 2 |
| 14 | Data Quality | `dataset-service/quality` | ✅ Done | Phase 2 |
| 15 | Slate/Workshop (App Builder) | `app-builder-service` | ✅ Done | Phase 3 |
| 16 | ML / Model Management | `ml-service` | ✅ Done | Phase 3 |
| 17 | AIP (GenAI / LLM / Copilot) | `ai-service` | ✅ Done | Phase 3 |
| 18 | Reports | `report-service` | ✅ Done | Phase 4 |
| 19 | Fusion (Entity Resolution) | `fusion-service` | ✅ Done | Phase 4 |
| 20 | Code Repositories (Git) | `code-repo-service` | ✅ Done | Phase 4 |
| 21 | Marketplace | `marketplace-service` | ✅ Done | Phase 4 |
| 22 | Streaming (Real-time) | `streaming-service` | ✅ Done | Phase 4 |
| 23 | Geospatial / Maps | `geospatial-service` | ✅ Done | Phase 4 |
| 24 | Audit & Compliance | `audit-service` | ✅ Done | Phase 4 |
| 25 | Nexus (Cross-org Sharing) | `nexus-service` | ✅ Done | Phase 5 |

Current repo audit: 24 components are shipped. Enterprise auth remains the only partial component because OIDC is implemented, while SAML sign-in flow is still pending.

---

## Phase 1 — Foundation 🏗️

> **Goal:** A working platform where you can connect data, explore it, build pipelines, and define an ontology.
>
> **Priority:** 🔴 Critical — nothing works without this.

### Milestone 1.1 — Platform Bootstrap

- [x] **Rust workspace setup** — Cargo workspace, shared crates compile, `just` recipes
- [x] **Protobuf generation** — `buf` pipeline generating Rust (tonic) and TypeScript clients
- [x] **Shared libraries** — `core-models`, `auth-middleware`, `event-bus`, `storage-abstraction`
- [x] **Gateway service** — Axum HTTP server, gRPC-Web proxy, request ID propagation, CORS
- [x] **Auth service (basic)** — JWT issue/validate, local user registration, session management
- [x] **Docker Compose dev stack** — PostgreSQL, Redis, NATS, MinIO running with one command
- [x] **SvelteKit shell** — App layout, sidebar, top bar, routing, auth flow, design system (base UI components)
- [x] **CI pipeline** — GitHub Actions: lint (clippy + eslint), test, build, proto-check

### Milestone 1.2 — Data Layer

- [x] **Dataset service** — CRUD, Parquet read/write, schema management, basic versioning
- [x] **Data connectors (first wave)** — PostgreSQL, MySQL, CSV, Parquet, JSON, S3, REST API
- [x] **Query service** — DataFusion integration, SQL execution, result pagination, saved queries
- [x] **Frontend: Dataset Explorer** — Data preview table, schema viewer, upload flow
- [x] **Frontend: SQL Workbench** — Monaco SQL editor, query execution, results table

### Milestone 1.3 — Ontology & Pipelines

- [x] **Ontology service** — Object types, properties, link types, CRUD, type validation
- [x] **Pipeline service (basic)** — DAG definition, SQL transforms, sequential execution
- [x] **Data lineage** — Dataset-level lineage tracking, lineage graph queries
- [x] **Frontend: Ontology Explorer** — Type editor, object explorer, graph view (Cytoscape.js)
- [x] **Frontend: Pipeline Builder** — DAG canvas (Svelvet), node palette, transform editor
- [x] **Frontend: Lineage View** — Interactive lineage graph

### Milestone 1.4 — Notebooks

- [x] **Notebook service** — Notebook CRUD, cell model, session management
- [x] **Python kernel** — PyO3-based Python execution, variable state, output capture
- [x] **SQL kernel** — Route SQL cells to query-service
- [x] **Frontend: Notebook Editor** — Cell editor (Monaco), cell outputs, kernel selector/status

**Phase 1 exit criteria:**
> A user can connect a Postgres database, explore tables, write SQL queries, build a simple pipeline with SQL transforms, define ontology object types backed by datasets, and run Python notebooks.

---

## Phase 2 — Core Platform 🧱

> **Goal:** Production-grade auth, dashboards, workflows, data quality, and catalog. The platform becomes usable for real teams.
>
> **Priority:** 🔴 Critical + 🟠 Medium features that complete the core loop.

### Milestone 2.1 — Enterprise Auth

- [x] **RBAC** — Roles, permissions, row-level security
- [x] **ABAC** — Attribute-based policies
- [ ] **SSO** — OAuth2/OIDC provider integration, SAML (OIDC implemented; SAML sign-in flow pending)
- [x] **MFA** — TOTP-based multi-factor authentication
- [x] **API keys** — Programmatic access management
- [x] **Frontend: User/Role management** — Settings pages for users, roles, groups

### Milestone 2.2 — Dashboards (Quiver)

- [x] **Dashboard grid layout** — Responsive drag-and-drop grid
- [x] **Chart widget** — ECharts integration: bar, line, area, pie, scatter, etc.
- [x] **Table widget** — Paginated, sortable, filterable data tables
- [x] **KPI widget** — Single metric cards with sparklines
- [x] **Filter bar** — Global filters propagated to all widgets
- [x] **Date range filter** — Relative and absolute date selection
- [x] **Dashboard CRUD** — Create, edit, duplicate, share dashboards

### Milestone 2.3 — Data Catalog & Quality

- [x] **Data catalog** — Search by name/tag/owner, dataset tagging, ownership assignment
- [x] **Auto-profiling** — Column statistics, distributions, null rates, uniqueness
- [x] **Quality rules** — Null checks, range validation, regex, custom SQL rules
- [x] **Quality scoring** — Per-dataset quality score, trend tracking
- [x] **Quality alerts** — Notifications on quality degradation
- [x] **Frontend: Catalog search** — Full-text search in dataset explorer
- [x] **Frontend: Quality dashboard** — Quality scores, profiling report, rule management

### Milestone 2.4 — Workflows & Notifications

- [x] **Workflow service** — Workflow definitions, step execution, conditional branching
- [x] **Triggers** — Cron, event-driven, manual, webhook triggers
- [x] **Human-in-the-loop** — Approval steps, approval queue
- [x] **Notification service** — Email (SMTP/SES), Slack, MS Teams webhooks
- [x] **In-app notifications** — WebSocket-based real-time notifications
- [x] **User preferences** — Per-user channel and frequency preferences
- [x] **Frontend: Workflow builder** — Visual workflow canvas, step config, trigger config
- [x] **Frontend: Notification bell** — In-app notification center

### Milestone 2.5 — Pipeline Enhancements

- [x] **Python transforms** — PyO3-based Python transform execution
- [x] **WASM sandbox** — Sandboxed WASM transforms for user-submitted code
- [x] **Column-level lineage** — Track lineage at the column level through transforms
- [x] **Pipeline scheduling** — Cron-based pipeline scheduling
- [x] **Retry & failure handling** — Configurable retry policies, partial re-execution
- [x] **Dataset branching** — Git-like branches for datasets, branch selector in UI

**Phase 2 exit criteria:**
> Teams can collaborate with proper auth/RBAC, build dashboards over their data, set up data quality monitoring, automate workflows with approvals, and receive notifications.

---

## Phase 3 — Intelligence 🧠

> **Goal:** ML, AI, and app building capabilities. This is where OpenFoundry becomes a true decision-making platform.
>
> **Priority:** 🔴 Critical — these are the features that make Foundry *Foundry*.

### Milestone 3.1 — App Builder (Slate/Workshop)

- [x] **App builder service** — App definitions, page layouts, widget catalog
- [x] **Widget system** — Table, form, chart, map, text, image, button, container
- [x] **Data binding** — Bind widgets to ontology objects, datasets, or queries
- [x] **Event handlers** — onClick → execute action, navigate, filter, etc.
- [x] **App theming** — Colors, fonts, branding customization
- [x] **Publish & deploy** — Version and publish apps, embedding support (iframe)
- [x] **App templates** — Starter templates for common use cases
- [x] **Frontend: WYSIWYG editor** — Drag-and-drop canvas, property inspector, live preview
- [x] **Frontend: App runtime** — Render published apps for end users

### Milestone 3.2 — ML Studio

- [x] **Experiment tracking** — Log runs with params, metrics, and artifacts
- [x] **Model registry** — Register models, manage versions (staging → production)
- [x] **Feature store** — Feature definitions, online serving (Redis), offline batch computation
- [x] **Training orchestration** — Submit training jobs, hyperparameter tuning
- [x] **Model serving** — Real-time inference endpoints, batch predictions
- [x] **A/B testing** — Traffic splitting between model versions
- [x] **Drift monitoring** — Data and concept drift detection, auto-retraining triggers
- [x] **Frontend: ML Studio** — Experiment list, run comparison, model registry, deployment panel

### Milestone 3.3 — AI Platform (AIP)

- [x] **LLM gateway** — Multi-provider routing (OpenAI, Anthropic, Ollama/local), load balancing, fallback
- [x] **Prompt management** — Versioned prompt templates, variable interpolation
- [x] **RAG pipeline** — Document chunking, embedding generation, semantic retrieval + reranking
- [x] **Knowledge bases** — Index datasets and ontology into vector store (Qdrant)
- [x] **AI agents** — Plan → Act → Observe loop, tool calling, task decomposition
- [x] **Platform copilot** — Natural language → SQL, pipeline suggestions, ontology help
- [x] **Guardrails** — Output validation, PII detection, toxicity filtering
- [x] **Semantic caching** — Cache LLM responses by semantic similarity
- [x] **Frontend: Copilot panel** — Floating drawer, conversational UI
- [x] **Frontend: Agent builder** — Visual agent configuration, tool registry
- [x] **Frontend: Knowledge manager** — Upload docs, manage knowledge bases

**Phase 3 exit criteria:**
> Users can build operational apps without code, train and deploy ML models, use AI agents and a platform copilot to accelerate their work, and build RAG pipelines over their data.

---

## Phase 4 — Platform Completeness 🔒

> **Goal:** Every remaining Foundry capability. Entity resolution, streaming, geospatial, code repos, marketplace, reports, and audit.
>
> **Priority:** 🟡 High — completes the platform for enterprise adoption.

### Milestone 4.1 — Entity Resolution (Fusion)

- [x] **Match rules** — Deterministic rules (exact, fuzzy, phonetic)
- [x] **ML-based matching** — Gradient boosted classifier for probabilistic matching
- [x] **Blocking strategies** — LSH, sorted neighborhood, key-based blocking
- [x] **String comparators** — Jaro-Winkler, Levenshtein, Soundex, metaphone
- [x] **Graph resolution** — Transitive closure for entity clusters
- [x] **Golden record** — Survivorship rules, merge strategies
- [x] **Human-in-the-loop** — Review queue for uncertain matches
- [x] **Frontend: Match rule builder, cluster viewer, manual review**

### Milestone 4.2 — Real-time Streaming

- [x] **Stream definitions** — Named streams with schemas
- [x] **Processing topology** — DAG-based stream processing
- [x] **Windowing** — Tumbling, sliding, and session windows
- [x] **Stream joins** — Stream-stream and stream-table joins
- [x] **Complex event processing** — Pattern matching on event sequences
- [x] **State backend** — RocksDB-based state store
- [x] **Connectors** — Kafka source, NATS source, HTTP webhook source, WebSocket sink, dataset sink
- [x] **Backpressure** — Flow control to prevent overload
- [x] **Frontend: Topology editor, stream monitor, live data tail**

### Milestone 4.3 — Reports & Geospatial

- [x] **Report service** — Report definitions, scheduled generation, distribution
- [x] **Generators** — PDF (typst), Excel (rust_xlsxwriter), CSV, HTML, PPTX
- [x] **Distribution** — Email, S3, Slack, webhook delivery
- [x] **Geospatial service** — Spatial queries (within, intersects, nearest, buffer)
- [x] **Vector tiles** — MVT tile server, H3 hex aggregation
- [x] **Geocoding** — Address ↔ coordinates
- [x] **Spatial clustering** — DBSCAN, K-means
- [x] **Routing** — Shortest path, isochrones
- [x] **Frontend: Report designer, preview, schedule manager**
- [x] **Frontend: MapLibre GL map, layer panel, heatmap, clustering, routing**

### Milestone 4.4 — Code Repos & Marketplace

- [x] **Code repo service** — Git object storage (gitoxide), branches, commits
- [x] **Merge requests** — Code review workflow, inline comments, approvals
- [x] **CI integration** — Trigger pipeline builds on push
- [x] **Code search** — Tantivy-indexed full-text code search
- [x] **Marketplace service** — Package registry, versioning, dependency resolution
- [x] **Package types** — Connectors, transforms, widgets, app templates, ML models, AI agents
- [x] **Discovery** — Search, categories, ratings & reviews
- [x] **One-click install** — Install packages into workspace
- [x] **Frontend: File browser, diff viewer, MR workflow**
- [x] **Frontend: Marketplace browser, publish wizard**

### Milestone 4.5 — Audit & Compliance

- [x] **Audit service** — Immutable append-only audit log
- [x] **Event collection** — Auto-capture from all services via NATS
- [x] **GDPR support** — Right to erasure, data portability
- [x] **Compliance reports** — SOC2, ISO 27001, HIPAA export formats
- [x] **Anomaly detection** — Alert on unusual access patterns
- [x] **Data classification** — PII, confidential, public labels
- [x] **Retention policies** — Configurable TTL for audit events
- [x] **Frontend: Audit log viewer, compliance dashboard, policy manager**

**Phase 4 exit criteria:**
> The platform has full feature parity with Palantir Foundry for all 24 of 25 components, suitable for enterprise production use.

---

## Phase 5 — Ecosystem 🌐

> **Goal:** Cross-organization data sharing, plugin SDK, and community ecosystem.
>
> **Priority:** 🟠 Medium — the network-effect layer.

### Milestone 5.1 — Nexus (Cross-org Data Sharing)

- [x] **Peer management** — Register and authenticate partner organizations
- [x] **Data sharing contracts** — Define what's shared, with whom, under what terms
- [x] **Federated queries** — Query shared data without copying it
- [x] **Selective replication** — Replicate subsets of data to consumer orgs
- [x] **E2E encryption** — Encrypted data in transit and at rest for shared datasets
- [x] **Cross-org audit trail** — Audit bridge between organizations
- [x] **Schema compatibility** — Validate schema compatibility across orgs
- [x] **Frontend: Peer list, share wizard, contract manager, shared data browser**

### Milestone 5.2 — Developer Ecosystem

- [x] **Plugin SDK** — Rust + WASM SDK for building custom connectors, transforms, widgets
- [x] **CLI tool** — `of` CLI for project management, deployment, and scripting
- [x] **REST API docs** — Full OpenAPI spec auto-generated from proto
- [x] **Developer portal** — Interactive API explorer, tutorials, cookbooks
- [x] **Terraform provider** — Manage OpenFoundry resources as IaC
- [x] **GitHub/GitLab integration** — External Git sync, CI/CD triggers
- [x] **Frontend: Developers portal with API explorer, SDK toolkit, Terraform panel, and repository integration manager**

### Milestone 5.3 — Performance & Scale

- [x] **Distributed query execution** — Multi-node DataFusion queries
- [x] **Distributed pipeline execution** — Parallel transform execution across workers
- [x] **Auto-scaling** — HPA/KEDA-based scaling per service
- [x] **Multi-tenancy** — Logical tenant isolation, resource quotas
- [x] **Global CDN** — Tile server and static asset caching at the edge
- [x] **Benchmark suite** — Reproducible benchmarks for all critical paths

**Phase 5 exit criteria:**
> Organizations can share data securely across boundaries, third-party developers can extend the platform, and the system scales to enterprise workloads.

---

## 📅 Indicative Timeline

> ⚠️ These are **estimates**, not commitments. Open source moves at the speed of contributors.

```
2026 Q2-Q3    Phase 1 — Foundation
              ████████████████████████████░░░░░░░░░░░░░░░░░░░

2026 Q3-Q4    Phase 2 — Core Platform
              ░░░░░░░░░░░░████████████████████████░░░░░░░░░░░

2027 Q1-Q2    Phase 3 — Intelligence (ML, AI, App Builder)
              ░░░░░░░░░░░░░░░░░░░░░░░░████████████████░░░░░░

2027 Q2-Q3    Phase 4 — Platform Completeness
              ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████████░░

2027 Q3+      Phase 5 — Ecosystem
              ░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░████████
```

---

## 🎯 How We Prioritize

1. **User value first** — Does this unlock a workflow that wasn't possible before?
2. **Foundation before features** — Auth, data layer, and ontology must be solid before ML/AI.
3. **Horizontal before vertical** — Basic versions of many features > perfect version of one.
4. **Community signal** — GitHub issues with 👍 reactions influence priority.
5. **Contributor interest** — If someone wants to build it, we help them ship it.

---

## 🤝 Help Us Get There Faster

Every contribution accelerates the roadmap. Here's where help is most needed:

| Phase | Area | What's Needed |
|---|---|---|
| **Phase 1** | Data connectors | Implement `trait DataConnector` for new sources |
| **Phase 1** | Frontend | SvelteKit pages, Tailwind components |
| **Phase 2** | Dashboard widgets | New chart types, custom widgets |
| **Phase 2** | Quality rules | Custom data quality rule implementations |
| **Phase 3** | LLM providers | Adapters for Gemini, Mistral, Cohere, etc. |
| **Phase 4** | Geospatial | PostGIS integration, spatial algorithms |
| **Phase 4** | Report generators | PDF, Excel, PPTX template engines |
| **Phase 2** | Enterprise SSO | Wire SAML sign-in flow, provider validation, and end-to-end login testing |

**Want to contribute?** Check the [issues labeled `help wanted`](https://github.com/open-foundry/open-foundry/labels/help%20wanted) or comment on this roadmap's [tracking issue](#).

---

<div align="center">

*This roadmap is a living document. It evolves with community feedback and contributions.*

**[Discuss the roadmap →](https://github.com/open-foundry/open-foundry/discussions)**

</div>
