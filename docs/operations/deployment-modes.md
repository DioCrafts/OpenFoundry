# Deployment Modes

OpenFoundry supports multiple operating profiles ranging from local development to opinionated Kubernetes overlays.

## Local Compose

The local stack lives under `infra/`:

- `infra/docker-compose.yml`
- `infra/docker-compose.dev.yml`
- `infra/docker-compose.monitoring.yml`

These files provide the backing services needed for day-to-day development and smoke execution. The `just infra-up` and `just dev-stack` commands are the normal contributor entrypoints.

## Kubernetes Via Helm

The chart lives at `infra/k8s/helm/open-foundry`.

Available values files show several deployment postures:

- `values.yaml` for the shared base
- `values-dev.yaml` for development
- `values-staging.yaml` for staging
- `values-prod.yaml` for production
- `values-multicloud.yaml` for multi-cloud topology
- `values-airgap.yaml` for air-gapped environments
- `values-sovereign-eu.yaml` for EU residency constraints
- `values-apollo.yaml` for Apollo-oriented rollout automation

The Helm CI workflow lints and renders several overlays so template validity is part of normal review.

## Terraform Assets

Terraform content is split into:

- `infra/terraform/modules/cdn`
- `infra/terraform/providers/openfoundry`

The custom provider directory also contains `provider.schema.json`, which is treated as a checked-in documentation and integration artifact.

## Docker Images

Selected services are published through `docker-publish.yml` using their service-local Dockerfiles. The workflow currently focuses on a subset of core services such as `gateway`, `auth-service`, `dataset-service`, `query-service`, `pipeline-service`, and `ontology-service`.

## Release Model

- container images are pushed through GitHub Actions
- tagged releases are published through `release.yml`
- docs are published independently through GitHub Pages

This separation keeps operational documentation deployable even when application release cadence changes.
