# Quality Gates

OpenFoundry uses CI as an executable compatibility contract across Rust services, frontend code, generated artifacts, infra packaging, and SDK outputs.

## Workflow Inventory

| Workflow | Purpose | Typical Local Entry Point |
| --- | --- | --- |
| `ci.yml` | Rust check, clippy, format, test, smoke, and dependency policy. | `cargo check --workspace`, `just lint`, `just test` |
| `ci-frontend.yml` | Frontend lint, Svelte type checks, unit tests, E2E, and production build. | `just ci-frontend` |
| `proto-check.yml` | Buf lint/breaking checks plus OpenAPI and SDK drift validation. | `just proto-lint`, `just openapi-check`, `just sdk-typescript-check`, `just sdk-python-check`, `just sdk-java-check` |
| `helm-check.yml` | Helm lint and render validation across deployment overlays. | `just helm-check` |
| `terraform-check.yml` | Terraform format plus module and schema validation. | `terraform fmt -check -recursive infra/terraform` |
| `sdk-smoke.yml` | Compiles and imports generated SDKs outside the main generation workflow. | `just sdk-typescript-typecheck`, `just sdk-python-compile`, `just sdk-java-compile` |
| `security-audit.yml` | Weekly and lockfile-triggered Rust security audit. | `cargo audit` |
| `docker-publish.yml` | Builds and pushes selected service images to GHCR. | `just docker-build` |
| `release.yml` | Generates tagged GitHub releases and changelog entries. | Git tag push flow |
| `deploy-docs.yml` | Builds VitePress docs and deploys them to GitHub Pages. | `just docs-build` |

## Executable Architecture Through Smoke Tests

The smoke suites are especially important because they validate feature chains rather than isolated units:

- `p2-runtime-critical-path.json` covers connection, dataset, pipeline, query, streaming, report, and geospatial runtime flows.
- `p3-semantic-governance-critical-path.json` covers ontology and governance-oriented semantics.
- `p4-developer-platform-critical-path.json` covers code repository and platform-builder flows.
- `p5-ai-ml-critical-path.json` covers provider-backed AI and ML paths.
- `p6-analytics-enterprise-critical-path.json` covers enterprise analytics and geospatial scenarios.

When you modify cross-cutting behavior, the smoke layer is often the first place that will tell you whether the overall platform contract still holds.

## What To Watch During Review

- Contract changes can require OpenAPI, SDK, or frontend updates.
- Infra changes can break Helm, Terraform, or smoke setup even when unit tests pass.
- Service changes may need database, environment, or Compose updates outside the service folder itself.
- Docs changes should keep navigation, edit links, and Pages deployment in sync.
