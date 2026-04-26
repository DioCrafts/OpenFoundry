# Functions

Functions are the programmable extension surface of the ontology.

They are what you use when object types, properties, links, and declarative action mappings are not enough by themselves.

## What functions are for

A useful ontology function usually falls into one of four roles:

- read and enrich operational object data
- compute derived decisions or recommendations
- perform governed write logic behind an action
- integrate ontology state with AI, search, or external systems

In other words, functions are not only helpers. They are the place where semantic structure meets executable behavior.

## OpenFoundry's current function model

OpenFoundry already exposes a concrete package model through `services/ontology-service/src/models/function_package.rs`.

Each function package currently has:

- `name`
- `version`
- `display_name`
- `description`
- `runtime`
- `source`
- `entrypoint`
- `capabilities`

The package lifecycle is implemented in `services/ontology-service/src/handlers/functions.rs`, with endpoints for:

- create
- list
- fetch
- update
- delete
- validate
- simulate

That is important because it means functions are already treated as managed platform resources, not just arbitrary snippets in a repository.

OpenFoundry now also has the beginning of a real release model instead of treating every package as an unversioned mutable blob.

Function packages carry a semver `version`, and actions can now reference a function package in two ways:

- pinned by `function_package_id`
- by `function_package_name` + `function_package_version`

When a builder also sets `function_package_auto_upgrade: true`, the action resolves to the latest compatible stable version in the same major line.

For example, an action configured against `1.2.0` with auto-upgrade enabled may resolve to `1.5.3`, but not to `2.0.0`.

To keep this behavior predictable, auto-upgrade is intentionally blocked for unstable `0.y.z` versions.

## Capability policy

One of the strongest parts of the current design is the capability envelope.

`FunctionCapabilities` already lets the platform describe whether a function may:

- read ontology data
- write ontology data
- call AI services
- call networks

It also controls:

- timeout budget
- maximum source size

This is a strong design choice because functions become safer when they are declared as capability-scoped workloads instead of unrestricted code execution.

## Validation and simulation

Before a function becomes operational, builders usually need two kinds of confidence:

1. the package is structurally valid
2. the package behaves correctly against realistic object context

OpenFoundry already provides both concepts:

- validation endpoints to check the package contract
- simulation endpoints to execute the package with object context and parameters

This is visible in:

- `POST /api/v1/ontology/functions/{id}/validate`
- `POST /api/v1/ontology/functions/{id}/simulate`

That simulation flow is particularly valuable because it keeps function authoring close to ontology behavior, not just close to code compilation.

## Run history and metrics

OpenFoundry now also has a native monitoring surface for reusable function packages.

Each package can expose:

- recent execution history
- success and failure counts
- simulation versus action-backed invocation counts
- average and P95 duration
- last successful and last failed run timestamps

The current implementation records runs from two places:

- direct package simulation
- actions that resolve to a reusable function package

That matters because the monitoring view is not just a generic platform log. It is tied to the ontology function resource itself and helps builders answer operational questions such as:

- is this package healthy?
- are failures happening only in simulations or in real actions?
- did latency regress after a new release?
- which action is driving most executions?

## Authoring kits and scaffolds

OpenFoundry now exposes a first-class authoring surface for ontology functions instead of leaving templates buried in local frontend code.

The ontology service publishes a native authoring catalog with:

- reusable TypeScript and Python starter templates
- suggested capability envelopes per template
- SDK package references
- CLI scaffold commands for new function package workspaces

This makes function authoring more productized in two ways:

1. builders can start from backend-defined templates inside the Functions Platform UI
2. pro-code teams can scaffold matching local workspaces with `of-cli`

The current CLI scaffolds include:

- `cargo run -p of-cli -- project init customer-triage --template function-typescript --output packages`
- `cargo run -p of-cli -- project init anomaly-diagnostics --template function-python --output packages`

That is still lighter than a full remote repository product, but it is no longer accurate to say that OpenFoundry only has ad hoc snippets plus SDK generation.

## Relationship with actions

Functions become much more useful when they can participate in governed ontology mutations.

OpenFoundry already supports this pattern in two complementary ways:

- action types may use `invoke_function`
- standalone function packages can be simulated as if they were synthetic actions

The integration path lives across:

- `services/ontology-service/src/handlers/actions.rs`
- `services/ontology-service/src/handlers/functions.rs`
- `services/ontology-service/src/domain/function_runtime.rs`

This is the right direction because it allows the platform to keep one consistent policy surface for:

- who can call the logic
- which object is in scope
- whether the function may write
- whether AI or network egress is allowed

It also means action definitions no longer need to choose between fully inline code and a permanently pinned package ID.

They can now express three distinct release strategies:

- inline code embedded directly in the action
- a pinned package release by ID
- a named package release with optional compatible auto-upgrade

## Relationship with search, AI, and SDKs

Functions do not live in isolation.

The repo suggests a broader programmable platform around them:

- `services/ontology-service/src/domain/search/*`
- `services/ai-service`
- `tools/of-cli`
- `sdks/`

That means functions can reasonably evolve into the place where builders combine:

- ontology queries
- semantic retrieval
- agent or LLM calls
- external service orchestration
- generated SDK contracts

## Recommended authoring flow

For OpenFoundry, a good function workflow would be:

1. Model the object and property contract first.
2. Keep the first function read-only if possible.
3. Declare the smallest capability set that works.
4. Validate the package contract.
5. Simulate it against one or more target objects.
6. Attach it to an action only after the behavior is understandable.
7. Add audit and monitoring hooks before broad rollout.

## OpenFoundry mapping

The core implementation signals today are:

- `services/ontology-service/src/models/function_package.rs`
- `services/ontology-service/src/handlers/functions.rs`
- `services/ontology-service/src/domain/function_runtime.rs`
- `services/ai-service`
- `tools/of-cli/src/main.rs`

The CLI is also relevant because it already contains contract-generation commands for:

- OpenAPI generation and validation
- TypeScript SDK generation and validation
- Python SDK generation and validation
- Java SDK generation and validation

It now also contains first-class scaffolds for local function package workspaces in both TypeScript and Python.

That is not the same thing as a full hosted Git repository product, but it is now a concrete function authoring surface instead of only a side-effect of the broader developer toolchain.

## What is still missing

The repository still appears to be early or partial in a few function areas:

- no immutable publish/promote workflow for function releases; package versions exist, but the broader release-management UX is still lightweight
- no explicit package dependency graph or marketplace-grade promotion flow in the ontology surface
- no deep hosted repository workflow yet for branches, review, and release promotion tied specifically to ontology functions
- function monitoring currently centers on reusable function packages and their action-backed invocations, not on every anonymous inline function snippet
- no protobuf contract yet for ontology functions comparable to the REST handlers

## Related pages

- [Action types](/ontology-building/action-types)
- [Functions by runtime](/ontology-building/functions-runtime/)
- [Semantic search](/ontology-building/semantic-search)
- [Ontology architecture](/ontology-building/ontology-architecture/)
