# OpenFoundry — Task Runner
# Usage: just <recipe>

set dotenv-load := true

# ── Default ──────────────────────────────────────────────────

default:
    @just --list

# ── Build ────────────────────────────────────────────────────

# Build all Rust services and libraries
build:
    cargo build --workspace

# Build in release mode
build-release:
    cargo build --workspace --release

# Build a specific service
build-svc svc:
    cargo build -p {{svc}}

# ── Test ─────────────────────────────────────────────────────

# Run all tests
test:
    cargo test --workspace

# Run tests for a specific crate
test-svc svc:
    cargo test -p {{svc}}

# Run tests with output
test-verbose:
    cargo test --workspace -- --nocapture

# ── Lint & Format ────────────────────────────────────────────

# Run all lints
lint: fmt-check clippy

# Check formatting
fmt-check:
    cargo fmt --all -- --check

# Format all code
fmt:
    cargo fmt --all

# Run Clippy
clippy:
    cargo clippy --workspace --all-targets -- -D warnings

# Run cargo-deny (license & vulnerability audit)
deny:
    cargo deny check

# ── Run ──────────────────────────────────────────────────────

# Run a specific service
run svc:
    cargo run -p {{svc}}

# Run the gateway
run-gateway:
    cargo run -p gateway

# Run the OpenFoundry CLI
of args='':
    cargo run -p of-cli -- {{args}}

# ── Database ─────────────────────────────────────────────────

# Run all migrations
db-migrate:
    @for dir in services/*/migrations; do \
        svc=$(basename $(dirname "$dir")); \
        echo "→ Migrating $svc..."; \
        cargo sqlx migrate run --source "$dir"; \
    done

# Create a new migration for a service
db-new-migration svc name:
    cargo sqlx migrate add -r --source "services/{{svc}}/migrations" {{name}}

# ── Protobuf ─────────────────────────────────────────────────

# Generate code from .proto files
proto-gen:
    cd proto && buf generate

# Generate OpenAPI docs from proto services
openapi-gen:
    cargo run -p of-cli -- docs generate-openapi --output apps/web/static/generated/openapi/openfoundry.json

# Validate checked-in OpenAPI docs against the current proto/tooling state
openapi-check:
    cargo run -p of-cli -- docs validate-openapi --input apps/web/static/generated/openapi/openfoundry.json

# Generate the official TypeScript SDK from the checked-in OpenAPI contract
sdk-typescript-gen:
    cargo run -p of-cli -- docs generate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk

# Validate checked-in SDK output against the current OpenAPI contract
sdk-typescript-check:
    cargo run -p of-cli -- docs validate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk

# Typecheck the generated SDK with the existing frontend TypeScript toolchain
sdk-typescript-typecheck:
    cd apps/web && pnpm exec tsc -p ../../sdks/typescript/openfoundry-sdk/tsconfig.json --noEmit

# Generate the official Python SDK from the checked-in OpenAPI contract
sdk-python-gen:
    cargo run -p of-cli -- docs generate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk

# Validate checked-in Python SDK output against the current OpenAPI contract
sdk-python-check:
    cargo run -p of-cli -- docs validate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk

# Compile the generated Python SDK to catch syntax/import issues
sdk-python-compile:
    python3 -m compileall sdks/python/openfoundry-sdk

# Generate the official Java SDK from the checked-in OpenAPI contract
sdk-java-gen:
    cargo run -p of-cli -- docs generate-sdk-java --input apps/web/static/generated/openapi/openfoundry.json --output sdks/java/openfoundry-sdk

# Validate checked-in Java SDK output against the current OpenAPI contract
sdk-java-check:
    cargo run -p of-cli -- docs validate-sdk-java --input apps/web/static/generated/openapi/openfoundry.json --output sdks/java/openfoundry-sdk

# Compile the generated Java SDK sources (requires JDK 17+)
sdk-java-compile:
    find sdks/java/openfoundry-sdk/src/main/java -name '*.java' -print0 | xargs -0 javac --release 17

# Validate the Helm chart across base/dev/staging/prod overlays
helm-check:
    helm lint infra/k8s/helm/open-foundry -f infra/k8s/helm/open-foundry/values.yaml -f infra/k8s/helm/open-foundry/values-dev.yaml
    helm template open-foundry infra/k8s/helm/open-foundry --namespace openfoundry -f infra/k8s/helm/open-foundry/values.yaml >/tmp/open-foundry-base.yaml
    helm template open-foundry infra/k8s/helm/open-foundry --namespace openfoundry -f infra/k8s/helm/open-foundry/values.yaml -f infra/k8s/helm/open-foundry/values-staging.yaml >/tmp/open-foundry-staging.yaml
    helm template open-foundry infra/k8s/helm/open-foundry --namespace openfoundry -f infra/k8s/helm/open-foundry/values.yaml -f infra/k8s/helm/open-foundry/values-prod.yaml >/tmp/open-foundry-prod.yaml

# Generate Terraform provider schema for docs and portal consumption
terraform-schema:
    cargo run -p of-cli -- terraform schema --output infra/terraform/providers/openfoundry/provider.schema.json
    cargo run -p of-cli -- terraform schema --output apps/web/static/generated/terraform/openfoundry-provider.json

# Run reproducible benchmark suite against a live stack
bench-critical-paths:
    cargo run -p of-cli -- bench run --scenario benchmarks/scenarios/critical-paths.json --output benchmarks/results/critical-paths.json

# Run the critical-path smoke suite against a live stack
smoke-critical-paths:
    cargo run -p of-cli -- smoke run --scenario smoke/scenarios/p2-runtime-critical-path.json --output smoke/results/p2-runtime-critical-path.json

# Run the semantic/governance smoke suite against a live stack
smoke-p3-semantic-governance:
    cargo run -p of-cli -- smoke run --scenario smoke/scenarios/p3-semantic-governance-critical-path.json --output smoke/results/p3-semantic-governance-critical-path.json

# Run the developer platform smoke suite against a live stack
smoke-p4-developer-platform:
    cargo run -p of-cli -- smoke run --scenario smoke/scenarios/p4-developer-platform-critical-path.json --output smoke/results/p4-developer-platform-critical-path.json

# Run the AI/ML smoke suite against a live stack
smoke-p5-ai-ml:
    cargo run -p of-cli -- smoke run --scenario smoke/scenarios/p5-ai-ml-critical-path.json --output smoke/results/p5-ai-ml-critical-path.json

# Run the analytics/control-panel/nexus/enterprise smoke suite against a live stack
smoke-p6-analytics-enterprise:
    cargo run -p of-cli -- smoke run --scenario smoke/scenarios/p6-analytics-enterprise-critical-path.json --output smoke/results/p6-analytics-enterprise-critical-path.json

# Lint proto files
proto-lint:
    cd proto && buf lint

# Check for breaking changes
proto-breaking:
    cd proto && buf breaking --against '.git#branch=main'

# ── Docker ───────────────────────────────────────────────────

# Start dev infrastructure (Postgres, Redis, NATS, MinIO, Meilisearch)
infra-up:
    docker compose -f infra/docker-compose.yml -f infra/docker-compose.dev.yml up -d

# Stop dev infrastructure
infra-down:
    docker compose -f infra/docker-compose.yml -f infra/docker-compose.dev.yml down

# Start with monitoring stack
infra-up-full:
    docker compose -f infra/docker-compose.yml -f infra/docker-compose.dev.yml -f infra/docker-compose.monitoring.yml up -d

# Build all Docker images
docker-build:
    @for dir in services/*/Dockerfile; do \
        svc=$(basename $(dirname "$dir")); \
        echo "→ Building $svc..."; \
        docker build -t "open-foundry/$svc:latest" -f "$dir" .; \
    done

# ── Frontend ─────────────────────────────────────────────────

# Install frontend dependencies
fe-install:
    cd apps/web && pnpm install

# Run frontend dev server
fe-dev:
    cd apps/web && pnpm dev

# Build frontend
fe-build:
    cd apps/web && pnpm build

# Lint frontend
fe-lint:
    cd apps/web && pnpm lint

# Typecheck frontend
fe-check:
    cd apps/web && pnpm check

# Run frontend tests
fe-test:
    cd apps/web && pnpm test

# Run frontend unit tests
fe-test-unit:
    cd apps/web && pnpm test:unit

# Run frontend E2E tests
fe-test-e2e:
    cd apps/web && pnpm test:e2e

# Run frontend CI checks locally
ci-frontend: fe-lint fe-check fe-test-unit fe-build
    @echo "✅ Frontend CI checks passed"

# ── CI ───────────────────────────────────────────────────────

# Run full CI checks locally
ci: lint test proto-lint openapi-check sdk-typescript-check sdk-typescript-typecheck sdk-python-check sdk-python-compile ci-frontend
    @echo "✅ All CI checks passed"

# ── Cleanup ──────────────────────────────────────────────────

# Clean build artifacts
clean:
    cargo clean
    rm -rf apps/web/node_modules apps/web/.svelte-kit apps/web/build
