# Ontology building

This section is the semantic core of OpenFoundry.

It documents the part of the platform that can turn datasets, streams, workflows, policies, and AI capabilities into a shared operational model of the business.

## OpenFoundry mapping

The closest repository components today are:

- `services/ontology-service`
- `services/auth-service`
- `services/audit-service`
- `services/query-service`
- `services/workflow-service`
- `services/app-builder-service`
- `services/geospatial-service`
- `services/ai-service`
- `proto/ontology/*`
- `proto/workflow/*`
- `proto/query/*`

## Section map

- [Why create an Ontology?](/ontology-building/why-create-an-ontology)
- [Core concepts](/ontology-building/core-concepts)
- [Ontology-aware applications](/ontology-building/ontology-aware-applications)
- [Define ontologies](/ontology-building/define-ontologies)
- [Object and link types](/ontology-building/object-and-link-types)
- [Object types](/ontology-building/object-types/)
- [Properties](/ontology-building/properties/)
- [Interfaces](/ontology-building/interfaces/)
- [Action types](/ontology-building/action-types)
- [Functions](/ontology-building/functions)
- [Functions by runtime](/ontology-building/functions-runtime/)
- [Rules and simulation](/ontology-building/rules-and-simulation)
- [Object sets and search](/ontology-building/object-sets-and-search)
- [Semantic search](/ontology-building/semantic-search)
- [Object permissioning](/ontology-building/object-permissioning)
- [Indexing and materialization](/ontology-building/indexing-and-materialization)
- [Object edits and conflict resolution](/ontology-building/object-edits-and-conflict-resolution)
- [Applications](/ontology-building/applications)
- [Applications catalog](/ontology-building/applications-catalog/)
- [Ontology architecture](/ontology-building/ontology-architecture/)
- [Ontology Manager](/ontology-building/ontology-manager)

## Current status

This is one of the most developed documentation areas in the site because it aligns unusually well with the service boundaries already visible in the monorepo.

The repository already contains real signals for:

- ontology metadata management
- object and link modeling
- action execution
- function simulation
- object-set policy and materialization
- semantic search
- ABAC and restricted views

The main gaps are less about the absence of ontology ideas and more about how much of the full lifecycle has already been consolidated into first-class platform products.
