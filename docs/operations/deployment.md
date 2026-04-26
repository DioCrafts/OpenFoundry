# Deployment Model

OpenFoundry currently supports two complementary deployment modes in-repository:

- local developer runtime with Docker-backed infrastructure and host-run services
- Kubernetes-oriented delivery through the Helm chart under `infra/k8s/helm/open-foundry`

## Local Infrastructure

The Compose stack defines:

- PostgreSQL
- Redis
- NATS
- MinIO
- Meilisearch
- Qdrant

Development overrides live in `infra/docker-compose.dev.yml`.

## Kubernetes Packaging

The Helm chart lives in:

```text
infra/k8s/helm/open-foundry
```

Important templates include:

- `deployment.yaml`
- `service.yaml`
- `ingress.yaml`
- `networkpolicy.yaml`
- `hpa.yaml`
- `scaledobject.yaml`
- `platform-profile-configmap.yaml`
- `apollo-cronjob.yaml`
- `poddisruptionbudget.yaml`

## Environment Overlays

The chart ships with multiple value overlays:

- `values.yaml`
- `values-dev.yaml`
- `values-staging.yaml`
- `values-prod.yaml`
- `values-airgap.yaml`
- `values-apollo.yaml`
- `values-multicloud.yaml`
- `values-sovereign-eu.yaml`

This layout signals that the repository is designed to support more than one operational profile instead of a single one-size-fits-all manifest.

## Local Commands

Common local deployment and runtime entry points are exposed in `justfile`:

```bash
just infra-up
just infra-down
just infra-up-full
just dev-stack
just dev-stack-fast
just smoke
```

## Chart Validation

The repository includes a `helm-check` recipe that:

- lints the base chart
- renders the base chart
- renders staging and production overlays

That gives maintainers a quick pre-merge validation path for deployment changes.
