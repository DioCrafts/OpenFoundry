# OpenFoundry Architecture

The canonical technical documentation for this repository now lives in [`docs/`](docs/).

Recommended entry points:

- [`docs/index.md`](docs/index.md) for the technical documentation homepage
- [`docs/guide/repository-map.md`](docs/guide/repository-map.md) for the monorepo layout
- [`docs/architecture/index.md`](docs/architecture/index.md) for the system overview
- [`docs/operations/ci-cd.md`](docs/operations/ci-cd.md) for delivery and automation flows

At a high level, OpenFoundry is a platform monorepo composed of:

- a SvelteKit frontend in `apps/web`
- a Rust gateway plus multiple bounded-context services in `services/`
- shared Rust foundations in `libs/`
- protobuf contracts in `proto/`
- generated SDK and schema artifacts in `sdks/` and `apps/web/static/generated/`
- infrastructure packaging and runbooks in `infra/`
