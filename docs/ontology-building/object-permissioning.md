# Object permissioning

Object permissioning is where ontology semantics and platform security become inseparable.

If object types define what the business cares about, permissioning defines who is allowed to see, query, edit, or route those objects.

## The security layers that matter

A useful ontology permission model usually has several layers working together:

1. identity and session claims
2. coarse-grained permissions
3. attribute-based rules
4. row and column shaping
5. action-time enforcement
6. object-set enforcement

OpenFoundry already contains meaningful signals in all six areas.

## Identity, roles, and permission keys

At the base layer, the repo already has JWT-backed claims and role or permission checks across services.

The strongest primitives are in:

- `libs/auth-middleware/src/claims.rs`
- `libs/auth-middleware/src/rbac.rs`
- `services/auth-service/src/domain/rbac.rs`

This is the coarse-grained layer that answers questions such as:

- may this caller list restricted views?
- may this caller manage policies?
- does this session have the required permission key for an action?

## ABAC policy evaluation

The next layer is more interesting for ontology use cases.

`services/auth-service/src/domain/abac.rs` already evaluates policies against:

- subject attributes
- resource attributes
- org boundaries
- markings and clearance
- restricted-view scope

The evaluation result can include:

- allow or deny
- matched policy IDs
- row filters
- hidden columns
- matched restricted views
- effective clearance
- consumer mode

That is already the vocabulary of a serious object-aware policy engine.

## Restricted views

Restricted views are the main shaping mechanism visible in the current repo.

`services/auth-service/src/handlers/restricted_views.rs` supports managed definitions with:

- conditions
- row filters
- hidden columns
- allowed org IDs
- allowed markings
- guest-access rules
- consumer-mode flags

This is important because many enterprise ontology scenarios do not want only allow or deny. They want controlled partial visibility.

## Action-time enforcement

Permissions are not only about reads.

OpenFoundry already applies policy at action time through:

- actor permission checks
- target-object marking checks
- role checks
- attribute checks
- guest-session restrictions
- classification clearance checks

These rules are enforced in `services/ontology-service/src/handlers/actions.rs`.

That is exactly where ontology permissioning becomes operational: not only can a user see an object, but they may or may not be allowed to change it in a particular way.

## Object-set enforcement

`services/ontology-service/src/domain/object_sets.rs` adds another useful layer.

Object sets can already carry policy requiring:

- allowed markings
- minimum clearance
- guest-session denial
- a required restricted view

This matters because object sets often become the unit of handoff between applications, workflows, and analysis surfaces.

## OpenFoundry mapping

The most relevant repository signals are:

- `services/auth-service/src/domain/rbac.rs`
- `services/auth-service/src/domain/abac.rs`
- `services/auth-service/src/handlers/restricted_views.rs`
- `services/auth-service/src/handlers/policy_mgmt.rs`
- `services/ontology-service/src/handlers/actions.rs`
- `services/ontology-service/src/domain/object_sets.rs`

Together, these files suggest that OpenFoundry already understands permissioning as a layered concern rather than as a single boolean check.

## What users should eventually experience

If the current architecture keeps maturing, users should be able to rely on:

- object visibility shaped by org and classification boundaries
- property redaction through hidden columns
- restricted views for external or consumer-style experiences
- action execution rules based on roles, permissions, and attributes
- object sets that remain safe when shared between application surfaces

## Current gaps

The repository still appears early or partial in a few important areas:

- no clearly modeled project-based ontology permission system
- no dedicated ontology metadata layer for per-property permission semantics
- no visible end-user access-check experience comparable to a complete builder console
- no obvious unified policy contract across every service that touches ontology state

So the foundations are strong, but the product surface is still less complete than the underlying security primitives.

## Related pages

- [Action types](/ontology-building/action-types)
- [Object sets and search](/ontology-building/object-sets-and-search)
- [Object edits and conflict resolution](/ontology-building/object-edits-and-conflict-resolution)
- [Ontology architecture](/ontology-building/ontology-architecture/)
