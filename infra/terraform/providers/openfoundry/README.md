# OpenFoundry Terraform Provider

This directory contains the provider schema and starter examples for managing OpenFoundry resources as infrastructure as code.

Generate the latest schema with:

```bash
just terraform-schema
```

The provider currently focuses on repository integrations, audit policies, and Nexus peers. The developer portal consumes the generated schema from `apps/web/static/generated/terraform/openfoundry-provider.json`.