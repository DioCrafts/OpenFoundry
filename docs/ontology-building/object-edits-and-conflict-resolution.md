# Object edits and conflict resolution

Object edits are the moment when the ontology stops being descriptive and becomes operationally authoritative.

As soon as users can change objects, the platform has to answer harder questions:

- when is an edit visible?
- where is it persisted?
- what happens when source data changes later?
- how do delete, recreate, and what-if flows behave?

## What OpenFoundry already supports

The current repository already contains a meaningful edit surface through ontology actions and object handlers.

Today, the strongest concrete signals are:

- object instance CRUD in `services/ontology-service/src/handlers/objects.rs`
- action execution in `services/ontology-service/src/handlers/actions.rs`
- batch action execution
- what-if branches for previewing alternative outcomes
- link creation as part of action execution
- function-backed mutation paths

This means OpenFoundry is already past the purely read-only ontology stage.

## The current execution shape

In practical terms, the current edit model appears to be:

1. validate target object access
2. materialize parameters
3. build an action plan
4. preview or execute the effects
5. update objects or links directly in the operational store

That is a good operational base because it keeps edits close to the semantic layer instead of scattering them across unrelated service contracts.

## Why conflict resolution matters

The hard part starts when the same semantic object can be influenced by two sources:

- upstream datasource refreshes
- human or workflow-driven edits

At that point, the platform needs an explicit conflict model.

Typical strategies are:

- source always wins
- user edits always win
- most recent timestamp wins
- property-by-property hybrid resolution

## What is not yet clearly modeled in this repo

The repository does not currently show a full datasource-versus-user-edit merge framework comparable to a dedicated ontology writeback system.

In particular, the current codebase does not visibly model:

- per-datasource conflict-resolution strategies
- timestamp-based resolution for edited properties
- durable edit instructions replayed during reindex
- delete-edit migration or wipe semantics
- explicit edit-only properties
- a builder-facing revert system for completed actions

So the platform already supports edits, but not yet the full lifecycle semantics of edits coexisting with continuously indexed source truth.

## What-if branches are already a strong signal

One of the most promising pieces already present is the what-if branch model for actions.

That matters because what-if branches are often the first step toward:

- scenario analysis
- branch-local review flows
- safer approvals
- audit-friendly experimentation

They do not replace durable conflict handling, but they do show the product is thinking beyond blind mutation endpoints.

## Recommended next milestones

If OpenFoundry wants to grow this area deliberately, the next steps should be:

1. define a durable edit instruction model
2. state how those instructions interact with reindexing
3. choose conflict strategies at object type or datasource level
4. expose edit history and revert semantics
5. connect edit outcomes to audit-service and indexing observability

## Related pages

- [Action types](/ontology-building/action-types)
- [Indexing and materialization](/ontology-building/indexing-and-materialization)
- [Object permissioning](/ontology-building/object-permissioning)
