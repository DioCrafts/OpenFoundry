# Indexing and materialization

Indexing is how raw platform inputs become queryable operational entities.

Materialization is how those entities, or the sets built from them, become reusable and stable enough to power applications, workflows, and analytics.

## Conceptual pipeline

In ontology terms, the path usually looks like this:

1. ingest data from connectors, datasets, or streams
2. normalize and transform it
3. map it onto object and link schemas
4. index it for low-latency reads
5. materialize reusable slices when needed

Even when a platform does not have one single indexing service, these five concerns still have to exist somewhere.

## OpenFoundry mapping

The current repository suggests the indexing path is distributed across:

- `services/data-connector`
- `services/dataset-service`
- `services/pipeline-service`
- `services/streaming-service`
- `services/ontology-service/src/handlers/funnel.rs`
- `services/ontology-service/src/domain/indexer.rs`

The important change is that OpenFoundry now exposes an explicit ontology-facing batch orchestrator.

The funnel surface in `ontology-service` lets builders define:

- a source dataset
- an optional upstream pipeline
- property mappings into an object type
- executable funnel runs with batch upsert into `object_instances`

That gives the platform a named ingestion-to-ontology path instead of leaving the full responsibility implicitly split across generic dataset and pipeline services.

## Batch and streaming

OpenFoundry already has separate service signals for both:

- batch-oriented dataset and pipeline work, now coordinated by ontology funnel sources and runs
- streaming-oriented ingestion

That matters because ontology indexing rarely stays purely batch forever. Operational use cases eventually demand some mix of:

- slower authoritative refreshes
- faster event-driven updates

## Search-oriented indexing

The ontology search path already depends on indexed documents built from ontology-visible objects.

`services/ontology-service/src/domain/search/mod.rs` calls into an indexer that builds search documents scoped to:

- object type
- search kind
- caller claims

This means indexing is not only about storage. It is also about shaping data so search and application surfaces can consume it efficiently.

## Materialization in the current repo

The clearest materialization signal today is in object sets.

`services/ontology-service/src/models/object_set.rs` already includes:

- `materialized_snapshot`
- `materialized_at`
- `materialized_row_count`

That is important because it shows the platform already distinguishes between:

- a logical object-set definition
- a stored, reusable realization of that definition

This is exactly the kind of distinction a serious ontology platform needs for repeatable application behavior.

## Indexing health and observability

OpenFoundry now also exposes a dedicated monitoring surface for ontology batch indexing through the funnel abstraction.

At the API level, `ontology-service` can now report:

- global funnel health across visible sources
- per-source health summaries
- success, failure, and warning counts
- rows read and object upsert volume
- recency signals such as `healthy`, `degraded`, `failing`, `stale`, `paused`, and `never_run`

This is important because ontology indexing health is not the same thing as generic infrastructure uptime.

Builders usually need to answer more operationally specific questions:

- which object feeds are stale?
- which ingestion sources are failing repeatedly?
- which feeds are completing but with row-level errors?
- how much ontology write activity is each batch source driving?

## Why materialization matters

Materialization becomes useful when:

- an application needs stable results instead of recomputing everything live
- an analyst wants a reusable what-if slice
- downstream systems need a frozen handoff
- expensive joins or traversals should not be recomputed on every read

## What still appears missing

Compared with a more complete ontology indexing architecture, the current repo still seems partial in a few areas:

- streaming ingestion is still not modeled with the same explicit ontology-facing orchestration as batch
- no clearly modeled batch-versus-stream policy per object type
- no explicit hydration or index lifecycle stages
- no documented persistent merge process combining datasource refreshes with user edits
- streaming indexing still lacks the same dedicated health surface that batch funnel sources now have

## Recommended direction

The next useful evolution for OpenFoundry would be:

1. formalize how object types are fed from datasets and streams
2. make index builds observable
3. document when object sets are evaluated live versus materialized
4. separate search indexing from object-storage semantics where needed
5. connect indexing behavior to edit durability and conflict handling

## Related pages

- [Ontology architecture](/ontology-building/ontology-architecture/)
- [Object edits and conflict resolution](/ontology-building/object-edits-and-conflict-resolution)
- [Object sets and search](/ontology-building/object-sets-and-search)
