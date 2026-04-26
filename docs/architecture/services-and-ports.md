# Services and Ports

All backend services expose a health endpoint and bind to fixed default ports in local development. The API gateway listens on `8080` and proxies public traffic to these internal services.

## Service Map

| Service | Default Port | Primary Role |
| --- | --- | --- |
| `gateway` | `8080` | Edge routing, proxying, audit middleware, CORS, request IDs, rate limiting |
| `auth-service` | `50051` | Identity, sessions, RBAC, SSO, MFA, policies, control panel |
| `data-connector` | `50052` | Connector catalog, connection testing, discovery, ingestion |
| `dataset-service` | `50053` | Datasets, versions, branches, filesystem, quality, linting |
| `streaming-service` | `50054` | Streaming pipelines and archive management |
| `query-service` | `50055` | Query execution surface |
| `pipeline-service` | `50056` | Pipeline execution, orchestration, compute runtime |
| `ontology-service` | `50057` | Object types, links, search, graph, actions, simulation |
| `fusion-service` | `50058` | Fusion and spreadsheet-oriented interactions |
| `ml-service` | `50059` | Experiments, training, registry, model lifecycle |
| `ai-service` | `50060` | AI providers, chat, tools, workflows |
| `workflow-service` | `50061` | Workflow orchestration |
| `notebook-service` | `50062` | Notebook and notepad runtimes |
| `app-builder-service` | `50063` | App composition and runtime surfaces |
| `report-service` | `50064` | Report generation and delivery |
| `code-repo-service` | `50065` | Code repository APIs |
| `marketplace-service` | `50066` | Marketplace and catalog APIs |
| `nexus-service` | `50067` | Federation, sharing, and multi-org collaboration |
| `geospatial-service` | `50068` | Geospatial and mapping APIs |
| `notification-service` | `50069` | Notification transport and inbox APIs |
| `audit-service` | `50070` | Audit collection and export |

## Gateway Route Ownership

The gateway maps URL prefixes to backend services. Important examples:

- `/api/v1/auth`, `/api/v1/users`, `/api/v1/roles`, `/api/v1/policies` -> `auth-service`
- `/api/v1/datasets`, `/api/v2/filesystem` -> `dataset-service`
- `/api/v1/pipelines`, `/api/v1/lineage` -> `pipeline-service`
- `/api/v1/ontology` -> `ontology-service`
- `/api/v1/ml` -> `ml-service`
- `/api/v1/ai` -> `ai-service`
- `/api/v1/reports` -> `report-service`
- `/api/v1/code-repos` -> `code-repo-service`
- `/api/v1/marketplace` -> `marketplace-service`
- `/api/v1/nexus` -> `nexus-service`

## Cross-Service Dependencies

Configuration files show explicit service-to-service defaults for several domains:

- `data-connector` knows about dataset, pipeline, and ontology services
- `pipeline-service` depends on dataset, workflow, and AI services
- `workflow-service` depends on notification, ontology, and pipeline services
- `ontology-service` depends on audit and AI services
- `report-service` depends on dataset and geospatial services
- `notebook-service` depends on query and AI services
- `marketplace-service` depends on app-builder

## Health Convention

Every service exposes a `/health` route. This shared convention is used by:

- local runtime scripts
- GitHub Actions smoke jobs
- Helm health probes and operational checks
