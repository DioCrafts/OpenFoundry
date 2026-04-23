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

# Generate Terraform provider schema for docs and portal consumption
terraform-schema:
    cargo run -p of-cli -- terraform schema --output infra/terraform/providers/openfoundry/provider.schema.json
    cargo run -p of-cli -- terraform schema --output apps/web/static/generated/terraform/openfoundry-provider.json

# Run reproducible benchmark suite against a live stack
bench-critical-paths:
    cargo run -p of-cli -- bench run --scenario benchmarks/scenarios/critical-paths.json --output benchmarks/results/critical-paths.json

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

# Run frontend tests
fe-test:
    cd apps/web && pnpm test

# ── CI ───────────────────────────────────────────────────────

# Run full CI checks locally
ci: lint test proto-lint
    @echo "✅ All CI checks passed"

# ── Cleanup ──────────────────────────────────────────────────

# Clean build artifacts
clean:
    cargo clean
    rm -rf apps/web/node_modules apps/web/.svelte-kit apps/web/build