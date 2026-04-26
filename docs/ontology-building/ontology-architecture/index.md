# Ontology architecture

The ontology architecture is the part of OpenFoundry that turns many independent platform services into one operational system.

Without this layer, the repo is still a capable data and workflow platform. With this layer, it can become a governed semantic operating system.

## Control plane and data plane

A useful mental model is to separate the ontology into two cooperating planes.

### Control plane

The control plane defines:

- object types
- properties
- interfaces
- link types
- action types
- function packages
- object-set definitions

### Data plane

The data plane executes:

- ingestion
- indexing
- search
- object reads
- object writes
- permission shaping
- AI-assisted retrieval

OpenFoundry already has pieces of both planes, but they are distributed across several services rather than concentrated into one named subsystem.

## The six main concerns in the current repo

### 1. Metadata authority

`services/ontology-service` is the clearest semantic control-plane service in the repository.

It already exposes managed resources for:

- object types
- properties
- interfaces
- shared property types
- links
- actions
- function packages
- object sets

### 2. Ingestion and indexing path

OpenFoundry now exposes a more explicit ontology ingestion path for batch indexing.

The ontology-facing orchestration sits in:

- `services/ontology-service/src/handlers/funnel.rs`
- `services/ontology-service/src/models/funnel.rs`

And it coordinates the lower-level platform services behind it:

- `services/data-connector`
- `services/dataset-service`
- `services/pipeline-service`
- `services/streaming-service`

This matters because the platform no longer depends only on an implicit combination of datasets and pipelines. It now has a named batch funnel abstraction that maps external rows into ontology object types and records funnel runs.

The same area is also becoming the natural observability surface for ontology ingestion, because funnel sources and runs can now expose health summaries instead of leaving builders to infer indexing health only from generic service logs.

### 3. Query and read layer

Ontology reads naturally span:

- `services/query-service`
- `services/ontology-service`
- `services/geospatial-service`
- `services/gateway`

This is the layer that should eventually serve object loading, graph traversal, search, and application-facing query patterns.

### 4. Mutation and decision layer

Controlled writes already map well to:

- `services/ontology-service`
- `services/workflow-service`
- `services/notification-service`
- `services/audit-service`

This is where action execution, workflow handoffs, and operational state change meet.

### 5. Governance layer

A usable ontology must be permission-aware and reviewable.

In the current repo, this concern maps primarily to:

- `services/auth-service`
- `services/audit-service`
- `libs/auth-middleware`

### 6. Intelligence and retrieval layer

The programmable and AI-aware layer is implied by:

- `services/ai-service`
- `services/ml-service`
- `services/notebook-service`
- `libs/vector-store`
- `tools/of-cli`

This is the layer that can make ontology entities retrievable, explainable, and actionable in AI-assisted workflows.

## What is already concrete

The ontology architecture is not only aspirational in this repo.

There are already concrete REST surfaces for:

- metadata CRUD
- action validation and execution
- function validation and simulation
- object search
- object graph access
- object sets with policy and materialization

That makes the architecture real enough to document as a platform slice, not only as a future design note.

## What still needs consolidation

The current architecture still looks distributed rather than fully converged.

The main gaps are:

- streaming indexing still lacks the same explicit ontology-facing orchestration that batch now has
- no visible persistent merge layer for datasource state plus user edits
- no unified contract surface in protobuf for some ontology resources
- no clearly separated storage backends for semantic reads versus writeback history

That is not necessarily a weakness for an early-stage system, but it does mean that several concepts still exist more as patterns across services than as fully named platform subsystems.

## Recommended architecture direction

If OpenFoundry continues toward a stronger ontology platform, the next architectural milestones should be:

1. strengthen metadata contracts
2. formalize indexing and materialization
3. unify query and search semantics
4. make object edits durable and conflict-aware
5. expose permission-aware semantic retrieval everywhere

## Related pages

- [Indexing and materialization](/ontology-building/indexing-and-materialization)
- [Object edits and conflict resolution](/ontology-building/object-edits-and-conflict-resolution)
- [Action types](/ontology-building/action-types)
- [Functions](/ontology-building/functions)
