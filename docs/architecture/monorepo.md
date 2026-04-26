# Monorepo Structure

The Cargo workspace is the primary organizational unit of OpenFoundry. It groups shared libraries, developer tooling, and runtime services under one repository.

## Top-Level Layout

| Path | Role |
| --- | --- |
| `apps/web` | Svelte frontend for the platform UI |
| `libs/` | Shared Rust crates used across services |
| `services/` | Runtime microservices |
| `tools/of-cli` | Internal CLI for generation, smoke, mock providers, and benchmarks |
| `proto/` | Protobuf and code-generation inputs |
| `sdks/` | Generated TypeScript, Python, and Java SDKs |
| `infra/` | Docker Compose, Helm charts, Terraform provider schema, and deployment overlays |
| `smoke/` | Scenario definitions for end-to-end smoke validation |
| `benchmarks/` | Benchmark scenarios and result outputs |
| `docs/` | VitePress technical documentation site |

## Workspace Composition

The Rust workspace currently includes:

- 10 shared libraries
- 1 primary developer tool (`of-cli`)
- 21 backend services

This structure makes it possible to share:

- authentication and claims middleware
- storage adapters
- vector and geospatial primitives
- audit and event abstractions
- testing helpers

## Frontend and Contract Placement

The frontend sits outside the Cargo workspace but inside the monorepo so that it can directly consume:

- generated OpenAPI JSON
- generated Terraform schema
- generated TypeScript SDK outputs

That keeps the UI and contract surfaces versioned alongside the backend services that produce them.

## Service Ownership Model

Each service typically contains:

- `src/main.rs`
- `src/config.rs`
- domain and handler modules under `src/`
- `migrations/` for service-owned PostgreSQL schema

The codebase follows a service-owned database model in local development and CI smoke flows, rather than forcing all services into a single shared migration stream.
