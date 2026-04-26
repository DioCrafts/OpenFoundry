# Local Development

This page describes the fastest reliable paths for working in the OpenFoundry monorepo.

## Required Tooling

- Rust `1.85` or newer
- Node.js `20+`
- pnpm `9+`
- Docker and Docker Compose
- `just` for the common contributor command surface

Optional but useful for specialized flows:

- Buf for protobuf linting and breaking-change checks
- Helm for chart validation
- Terraform for module validation

## Common Workflows

### Full Local Stack

Use this when you need infra, backend services, and the web app together:

```bash
just dev-stack
```

There is also a faster path when dependencies are already running and binaries are already built:

```bash
just dev-stack-fast
```

### Infrastructure Only

Use this when you only need backing services such as Postgres, Redis, NATS, MinIO, or Meilisearch:

```bash
just infra-up
just infra-down
```

### Backend Iteration

Build the whole Rust workspace:

```bash
just build
```

Build or run a specific service:

```bash
just build-svc gateway
just run-gateway
just run auth-service
```

Run tests:

```bash
just test
just test-svc gateway
```

### Frontend Iteration

The root Node scripts already proxy into `apps/web`:

```bash
pnpm dev
pnpm lint
pnpm test:unit
pnpm build
```

If you prefer to work directly in the app package:

```bash
pnpm --dir apps/web dev
pnpm --dir apps/web check
```

### Docs Iteration

The docs site is intentionally isolated under `docs/`:

```bash
just docs-install
just docs-dev
```

If you prefer to work directly inside the docs package:

```bash
cd docs
npm ci
npm run docs:dev
```

## Operational Assumptions

The repo is designed around service isolation rather than a single shared database. The smoke workflow creates separate Postgres databases for multiple services such as auth, datasets, pipelines, reports, geospatial, ontology, AI, and ML. That is a good mental model for local development too.

Several services also assume supporting infrastructure:

- Redis for gateway caching and stateful coordination
- NATS for async messaging
- MinIO or another object-store-compatible backend
- Meilisearch for search-oriented capabilities

## Helpful Commands From `justfile`

| Goal | Command |
| --- | --- |
| Lint and format Rust | `just lint` |
| Validate proto contracts | `just proto-lint` |
| Validate OpenAPI drift | `just openapi-check` |
| Validate TypeScript SDK drift | `just sdk-typescript-check` |
| Export Terraform schema | `just terraform-schema` |
| Run smoke suite | `just smoke` |
| Run benchmark suite | `just bench-critical-paths` |
