# OpenFoundry Benchmark Suite

This directory contains reproducible benchmark scenarios for the highest-impact platform paths: distributed queries, distributed pipelines, code search, audit listings, Nexus federation, and geospatial tile delivery.

Run the suite with a live stack and seeded fixture IDs:

```bash
export OPENFOUNDRY_BASE_URL=http://localhost:8080
export OPENFOUNDRY_TOKEN=<bearer-token>
export OPENFOUNDRY_PIPELINE_ID=<pipeline-id>
export OPENFOUNDRY_REPOSITORY_ID=<repository-id>
export OPENFOUNDRY_SHARE_ID=<share-id>
export OPENFOUNDRY_TILE_LAYER_ID=<layer-id>

just bench-critical-paths
```

The suite writes machine-readable results to `benchmarks/results/critical-paths.json` so changes can be compared across environments, branches, and scaling profiles.