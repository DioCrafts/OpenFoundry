# Repository Map

OpenFoundry is organized as a platform monorepo with clear directory-level ownership boundaries.

## Top-Level Layout

| Path | Role |
| --- | --- |
| `apps/web` | SvelteKit frontend and product UI routes. |
| `services/*` | Rust microservices, one crate per service, each with its own `Cargo.toml` and Dockerfile. |
| `libs/*` | Shared Rust crates such as auth middleware, event bus, vector store, and testing helpers. |
| `proto/*` | Protobuf contracts grouped by domain, plus Buf configuration. |
| `tools/of-cli` | CLI for smoke execution, benchmarks, OpenAPI validation, SDK generation, and Terraform schema export. |
| `infra/*` | Docker Compose, Helm, Terraform, backup scripts, and operational runbooks. |
| `sdks/*` | Generated SDKs for TypeScript, Python, and Java. |
| `smoke/*` | Critical-path end-to-end scenarios used to validate real platform flows. |
| `benchmarks/*` | Reproducible benchmark scenarios and results. |
| `images/*` | Shared repo imagery used by README and related materials. |
| `.github/workflows/*` | CI, release, packaging, security, and docs automation. |

## Workspace Control Files

| File | Purpose |
| --- | --- |
| `Cargo.toml` | Root Rust workspace definition for libs, services, and tooling crates. |
| `Cargo.lock` | Locked Rust dependency graph used by CI and release flows. |
| `package.json` | Root Node scripts that delegate to the web app. |
| `pnpm-workspace.yaml` | Current pnpm workspace definition for `apps/*`. |
| `justfile` | Contributor command surface for build, test, proto, infra, smoke, and frontend tasks. |
| `.gitignore` | Keeps generated local artifacts out of version control while preserving checked-in generated specs. |

## Delivery Surfaces

The repository produces more than one artifact:

- frontend bundles from `apps/web`
- Rust binaries from `services/*` and `tools/of-cli`
- Docker images from service-specific Dockerfiles
- generated OpenAPI, SDK, and Terraform schema artifacts
- Helm templates and Terraform modules
- GitHub Pages output from `docs/`

## Where To Look First

- If the change is product UI or navigation related, start in `apps/web/src/routes`.
- If it is API or service behavior, start in the matching folder under `services/`.
- If it affects a shared concern, inspect `libs/` before duplicating logic.
- If it changes public contract shape, inspect `proto/`, generated OpenAPI, and SDK flows together.
- If it changes deployability, inspect `infra/` and the relevant workflow under `.github/workflows/`.
