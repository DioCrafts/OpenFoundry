# Semantic search

Semantic search is the part of the ontology stack that helps users retrieve meaning, not just keywords.

In a mature platform, it becomes most valuable when the retrieved text is tied back to governed objects, links, workflows, and permissions.

## Two search surfaces already visible in OpenFoundry

The repository already suggests two complementary search paths.

### 1. Ontology search

`services/ontology-service/src/domain/search/mod.rs` already combines:

- full-text scoring
- semantic candidate recall
- provider-backed semantic reranking
- fusion strategies (`rrf` and `weighted`)
- title bonus logic
- ranking and truncation

This is still not a full vector-native ontology search engine, but it is now a real hybrid retrieval path instead of simple keyword matching plus a lightweight local similarity hint.

### 2. Knowledge-base retrieval in `ai-service`

`services/ai-service/src/handlers/knowledge.rs` and the `rag` domain show a second, more RAG-oriented path:

- knowledge base creation
- document ingestion
- embedding provider selection
- chunk indexing
- embedding-based retrieval

This makes the current platform shape especially interesting: one search surface is close to ontology objects, and another is close to AI knowledge workflows.

## Vector properties and KNN on ontology objects

OpenFoundry now also exposes a direct KNN surface over ontology object types when a property is explicitly modeled as `vector`.

The shape is straightforward:

- object properties can use the `vector` property type
- object instances can store numeric arrays in those properties
- `ontology-service` exposes `POST /api/v1/ontology/types/{type_id}/objects/knn`
- callers can query by `query_vector` or by `anchor_object_id`
- the service supports `cosine`, `dot_product`, and `euclidean` metrics

This is different from the hybrid text search path. Hybrid search starts from text and embeddings. KNN starts from a vector-valued object property and returns nearest ontology objects of the same type.

It is also visible beyond raw HTTP:

- the web client exposes a typed `knnObjects(...)` helper
- the function runtime SDK exposes `sdk.ontology.knnObjects(...)` in TypeScript and `sdk.ontology.knn_objects(...)` in Python

That matters because the capability is no longer just an internal primitive. It is part of the ontology application surface.

## End-to-end semantic workflow

A practical semantic-search architecture usually looks like this:

1. ingest documents or long-form text
2. normalize and chunk the content
3. create embeddings
4. retrieve candidate chunks
5. rank them against the user query
6. connect them back to operational entities
7. enforce visibility and policy before showing results

OpenFoundry already has parts of this flow in place, but distributed across services.

## OpenFoundry mapping

The most relevant repository signals are:

- `services/ontology-service/src/domain/search/mod.rs`
- `services/ontology-service/src/domain/search/semantic.rs`
- `services/ontology-service/src/handlers/search.rs`
- `services/ai-service/src/handlers/knowledge.rs`
- `services/ai-service/src/domain/rag/*`
- `libs/vector-store`

This suggests the following conceptual split:

- ontology-service owns user-facing semantic retrieval over ontology-shaped content
- ai-service owns document-oriented embedding and retrieval workflows
- vector-store can become the lower-level storage abstraction for future ANN or vector-native indexing

## What the current implementation already does well

The repo already shows several good design instincts:

- semantic search is optional per request
- ranking combines lexical and semantic relevance instead of treating them as mutually exclusive
- hybrid search can use provider-backed embeddings when `ontology-service` is configured with `search_embedding_provider=provider:<uuid>`
- ontology objects can now carry explicit `vector` properties for nearest-neighbor retrieval
- KNN is available as a first-class object query path, not only as a lower-level library concern
- knowledge bases track embedding providers explicitly
- retrieval is already modeled as a reusable domain concern

These are all strong signs that search is being treated as a platform capability rather than as a UI-only feature.

## Design guidance for OpenFoundry

If this area keeps evolving, the most useful path is:

1. Treat chunking as a first-class ingestion step, not a UI concern.
2. Keep semantic retrieval permission-aware from the start.
3. Distinguish clearly between object search and document search.
4. Prefer hybrid ranking over purely lexical or purely semantic ranking.
5. Ground search results in object views, workflows, or actions whenever possible.
6. Expose the same retrieval logic to functions, apps, and agents so the platform has one search brain instead of several disconnected ones.

## Current gaps

Compared with a more complete ontology-semantic platform, the current repository still appears partial in these areas:

- no clear multimodal retrieval path
- no dedicated chunking policy model in ontology-service
- no end-to-end permission-aware search contract shared across ontology and AI surfaces

Also worth noting: `services/ontology-service/src/domain/search/semantic.rs` still keeps the deterministic hash embedder as a fallback path. The important change is that it is no longer the only semantic signal available to ontology search.

## Related pages

- [Object sets and search](/ontology-building/object-sets-and-search)
- [Functions](/ontology-building/functions)
- [Ontology-aware applications](/ontology-building/ontology-aware-applications)
- [Ontology architecture](/ontology-building/ontology-architecture/)
