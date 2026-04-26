# Architecture Overview

OpenFoundry is structured as a platform monorepo with a browser UI, a gateway, many domain services, shared contracts, and multiple generated outputs.

## Layered Model

| Layer | Primary Paths | Notes |
| --- | --- | --- |
| Experience | `apps/web`, `services/gateway` | User-facing UI plus the HTTP entrypoint for service orchestration. |
| Domain services | `services/*` | Rust microservices grouped by platform capability. |
| Shared foundations | `libs/*`, `proto/*`, `tools/of-cli` | Reusable code, contracts, and operational tooling. |
| Delivery and operations | `infra/*`, `.github/workflows/*`, `docs/`, `sdks/*` | Packaging, deployability, release automation, and docs publishing. |

## What The Repository Optimizes For

- bounded contexts rather than a single backend binary
- generated contracts before hand-maintained client drift
- explicit service packaging through per-service Dockerfiles
- local reproducibility through `just`, smoke suites, and Compose
- operational portability through Helm, Terraform, and generated schemas

## Key Architectural Signals In The Repo

- `Cargo.toml` defines a large Rust workspace with shared libraries and independently deployable services.
- `apps/web/src/lib/api/*` mirrors the platform surface exposed through the gateway and downstream services.
- `proto/*` groups contracts by domain such as `dataset`, `pipeline`, `ontology`, `ai`, and `workflow`.
- `smoke/scenarios/*` encode the critical capability chains the platform promises to support.

## Read Next

- [Monorepo Structure](/architecture/monorepo) for workspace-level organization.
- [Runtime Topology](/architecture/runtime-topology) for request flow and service grouping.
- [Services and Ports](/architecture/services-and-ports) for local runtime defaults.
- [Contracts and SDKs](/architecture/contracts-and-sdks) for generation pipelines.
- [Capability Map](/architecture/capability-map) for the domain areas validated by the smoke suites.
