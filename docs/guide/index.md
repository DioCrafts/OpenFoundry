# Guide

This section is the onboarding path for contributors who need to understand how OpenFoundry is assembled and how to work in the repo without breaking adjacent systems.

## At A Glance

- `apps/web` is the SvelteKit control-plane frontend.
- `services/*` contains the Rust microservices that implement bounded domains.
- `libs/*` contains shared Rust crates reused by multiple services.
- `proto/*` defines cross-service contracts and feeds generated artifacts.
- `tools/of-cli` is the operational CLI used for docs, smoke tests, benchmarks, and schema generation.
- `infra/*` contains local Compose files, Helm charts, Terraform assets, scripts, and runbooks.
- `sdks/*` contains generated SDKs for TypeScript, Python, and Java.
- `smoke/*` and `benchmarks/*` turn critical journeys into executable checks.

## Why This Layout Matters

OpenFoundry is not a single app with a thin API. It is a platform repository with several delivery surfaces:

- a browser UI
- a gateway
- multiple domain services
- generated contracts and SDKs
- infrastructure packaging
- operational automation

That means a change in one directory can affect CI, generated artifacts, deployability, or runtime compatibility in another. The rest of this guide helps you reason about those connections quickly.

## Next Steps

- Read [Repository Map](/guide/repository-map) to learn where each concern lives.
- Read [Local Development](/guide/local-development) to get a productive day-to-day workflow.
- Read [Quality Gates](/guide/quality-gates) to understand what the CI pipelines protect.
