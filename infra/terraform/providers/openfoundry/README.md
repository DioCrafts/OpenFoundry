# OpenFoundry Terraform Provider

This directory contains the provider schema and starter examples for managing OpenFoundry resources as infrastructure as code.

Generate the latest schema with:

```bash
just terraform-schema
```

The provider currently covers repository integrations, audit policies, Nexus peers, product-delivery DevOps resources such as rollout fleets and enrollment branches, and deployment-fabric primitives for:

- multi-cloud deployment cells
- geo-fence / residency policies
- air-gapped release bundles
- Apollo rollout automation

The developer portal consumes the generated schema from `apps/web/static/generated/terraform/openfoundry-provider.json`.
