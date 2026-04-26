# Capability Map

The fastest way to understand what OpenFoundry is trying to deliver is to read its smoke suites as an executable platform map.

## Capability Phases Encoded In Smoke

| Phase | Scenario | Main Capability Areas |
| --- | --- | --- |
| P2 | `smoke/scenarios/p2-runtime-critical-path.json` | connectors, datasets, sync, pipelines, queries, streaming, reports, geospatial |
| P3 | `smoke/scenarios/p3-semantic-governance-critical-path.json` | ontology, interfaces, properties, governance-oriented workflows |
| P4 | `smoke/scenarios/p4-developer-platform-critical-path.json` | code repositories, branching, commits, search, developer platform flows |
| P5 | `smoke/scenarios/p5-ai-ml-critical-path.json` | AI providers, knowledge bases, embeddings, training jobs, model workflows |
| P6 | `smoke/scenarios/p6-analytics-enterprise-critical-path.json` | analytics datasets, enterprise-tier behaviors, geospatial exploration |

## How The Repo Reflects Those Phases

### Runtime and data operations

The P2 flow shows the core operational backbone:

- connect to a source
- sync into datasets
- operate on the data
- expose results through pipelines, queries, streaming, reports, and maps

This is reflected in service folders such as `data-connector`, `dataset-service`, `pipeline-service`, `query-service`, `streaming-service`, `report-service`, and `geospatial-service`.

### Semantic and governance layer

The P3 flow shows that OpenFoundry is not only a data movement stack. It also models meaning, interfaces, and governed domain structures through ontology-centric APIs.

That capability is reflected in `ontology-service`, `audit-service`, `auth-service`, and related shared middleware.

### Developer platform

The P4 flow demonstrates that the platform also includes repository-like development primitives such as branches, commits, search, and review-oriented flows.

That capability maps cleanly onto `code-repo-service`, and connects naturally with `app-builder-service` and `marketplace-service`.

### AI and ML

The P5 flow shows provider-backed AI and ML capabilities as first-class parts of the platform rather than bolt-on experiments:

- provider registration
- knowledge base creation
- document ingestion
- semantic search
- model training jobs

This is represented by `ai-service`, `ml-service`, and supporting shared crates such as `vector-store`.

### Enterprise analytics

The P6 flow extends the runtime path into richer analytics and geospatial use cases, reinforcing that the platform is meant to support decision workflows, not only CRUD APIs.

## Practical Reading Tip

If you need to understand a product area quickly, start with the matching smoke scenario and then read:

1. the corresponding frontend route in `apps/web/src/routes`
2. the service crate under `services/`
3. the domain contracts under `proto/`

That path usually gives you the shortest route from user behavior to implementation.
