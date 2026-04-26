# Runtime Topology

The runtime shape of OpenFoundry is easiest to understand as a layered service mesh behind a single gateway.

## High-Level Flow

```text
Browser / API Client
        |
        v
  apps/web or external client
        |
        v
   gateway (HTTP entrypoint)
        |
        +--> auth-service
        +--> data-connector
        +--> dataset-service
        +--> pipeline-service
        +--> query-service
        +--> streaming-service
        +--> ontology-service
        +--> audit-service
        +--> app-builder-service
        +--> code-repo-service
        +--> marketplace-service
        +--> ai-service
        +--> ml-service
        +--> geospatial-service
        +--> report-service
        +--> nexus-service
        +--> other bounded services
```

## Service Families

| Family | Services |
| --- | --- |
| Entry and experience | `gateway`, `app-builder-service`, `marketplace-service`, `notebook-service`, `notification-service` |
| Data plane | `data-connector`, `dataset-service`, `pipeline-service`, `query-service`, `streaming-service`, `report-service`, `geospatial-service`, `fusion-service`, `workflow-service` |
| Governance and semantics | `auth-service`, `audit-service`, `ontology-service`, `nexus-service` |
| Developer platform | `code-repo-service` |
| AI and ML | `ai-service`, `ml-service` |

## Shared Runtime Dependencies

The repo and workflows indicate a consistent set of platform dependencies:

- Postgres for service-owned relational state
- Redis for gateway-oriented caching and coordination
- NATS for async messaging
- object storage for datasets, archives, reports, and repository payloads
- Meilisearch for search-centric features

The CI smoke job creates multiple service-specific databases, which strongly suggests database-per-service isolation rather than a shared operational schema.

## Frontend Coupling

`apps/web/src/routes/*` and `apps/web/src/lib/api/*` mirror the runtime surface area of the platform. Route families such as `datasets`, `pipelines`, `ontology`, `geospatial`, `marketplace`, `code-repos`, `ai`, `ml`, and `nexus` map cleanly onto the backend service topology.

## Why The Gateway Matters

The gateway is the main control-plane entrypoint:

- it exposes health and API paths
- it centralizes cross-cutting middleware such as auth, CORS, request IDs, rate limiting, and audit hooks
- it routes downstream traffic to specialized services instead of collapsing everything into a single backend

That keeps the browser client simpler while preserving service autonomy behind the edge.
