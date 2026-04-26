# Contracts And SDKs

OpenFoundry treats contracts as first-class repository artifacts.

## Source Of Truth

The contract source starts in `proto/`, where domains are split into focused folders such as:

- `auth`
- `dataset`
- `pipeline`
- `query`
- `ontology`
- `workflow`
- `report`
- `ai`
- `ml`
- `geospatial`

Buf configuration lives alongside them in:

- `proto/buf.yaml`
- `proto/buf.lock`
- `proto/buf.gen.yaml`

## Generation Flow

```text
proto/*.proto
   |
   v
of-cli docs generate-openapi / validate-openapi
   |
   v
apps/web/static/generated/openapi/openfoundry.json
   |
   +--> TypeScript SDK
   +--> Python SDK
   +--> Java SDK
   +--> frontend consumers
```

## Artifact Destinations

| Artifact | Path |
| --- | --- |
| OpenAPI contract | `apps/web/static/generated/openapi/openfoundry.json` |
| TypeScript SDK | `sdks/typescript/openfoundry-sdk` |
| Python SDK | `sdks/python/openfoundry-sdk` |
| Java SDK | `sdks/java/openfoundry-sdk` |
| Terraform provider schema | `infra/terraform/providers/openfoundry/provider.schema.json` |
| Web-consumable Terraform schema | `apps/web/static/generated/terraform/openfoundry-provider.json` |

## Operational Tooling

`tools/of-cli` is the bridge between the Rust workspace and generated documentation assets. The `justfile` exposes the common flows:

- `just openapi-gen`
- `just openapi-check`
- `just sdk-typescript-gen`
- `just sdk-typescript-check`
- `just sdk-python-gen`
- `just sdk-python-check`
- `just sdk-java-gen`
- `just sdk-java-check`
- `just terraform-schema`

## Why This Matters

Generated artifacts are not secondary documentation. In OpenFoundry they are part of the platform contract:

- frontend code depends on the generated API shape
- SDKs depend on the checked-in OpenAPI output
- Terraform documentation depends on the provider schema JSON
- CI treats drift as a failure, not as optional cleanup

That makes `proto/` and the generation pipeline a critical review area for any public API change.
