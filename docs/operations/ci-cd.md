# CI/CD

OpenFoundry uses several focused GitHub Actions workflows instead of one monolithic pipeline. That keeps review scopes narrow and allows domain-specific checks to run only when relevant files change.

## Workflow Breakdown

| Workflow | Trigger Focus | Main Output |
| --- | --- | --- |
| `ci.yml` | Rust crates, proto, smoke, tooling | Workspace correctness and runtime smoke confidence |
| `ci-frontend.yml` | `apps/web` and root Node config | Linted, typed, tested, buildable frontend |
| `proto-check.yml` | `proto/`, generated artifacts, SDKs | Contract drift detection |
| `helm-check.yml` | Helm chart files | Valid rendered Kubernetes manifests |
| `terraform-check.yml` | Terraform files | Formatted and validated infra assets |
| `sdk-smoke.yml` | SDK folders | Compilable generated SDKs |
| `security-audit.yml` | schedule and `Cargo.lock` | Dependency vulnerability awareness |
| `docker-publish.yml` | `main` and tags | Published container images |
| `release.yml` | `v*` tags | GitHub releases with changelog |
| `deploy-docs.yml` | `docs/**` and manual dispatch | Published VitePress site |

## Docs Website Pipeline

The documentation website follows the same isolated pattern used by the reference `OxiCloud` repository:

1. checkout the repo
2. install docs-only Node dependencies inside `docs/` with the lockfile
3. build VitePress
4. upload `docs/.vitepress/dist`
5. deploy to GitHub Pages

### Workflow File

The workflow lives at `.github/workflows/deploy-docs.yml`.

### Trigger Policy

The docs deployment runs on:

- pushes to `main` that touch `docs/**`
- manual `workflow_dispatch`

That keeps Pages publishing decoupled from the heavier application pipelines.

### Required GitHub Setup

To make the pipeline effective, the repository should have:

- GitHub Pages enabled
- source set to GitHub Actions
- default branch aligned with `main`

## Contributor Guidance

- If you change `proto/`, expect SDK and OpenAPI workflows to matter.
- If you change infra packaging, check the Helm or Terraform pipelines, not only Rust tests.
- If you change docs navigation or VitePress config, verify `deploy-docs.yml` assumptions still hold and run `just docs-build`.
