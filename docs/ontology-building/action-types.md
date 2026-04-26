# Action types

Action types are the operational write interface of the ontology.

An object type tells the platform what a thing is. An action type tells the platform how that thing is allowed to change.

## Mental model

A good action type is not just an API mutation wrapped in a form. It is a governed transaction with five concerns:

1. a target semantic entity
2. a set of typed inputs
3. a constrained operation
4. authorization and submission rules
5. an observable execution result

That is why action types usually become the main bridge between object views, workflows, approvals, automations, and human decision-making.

## Core building blocks

### 1. Target and operation kind

Every action should start from a clear intent:

- update an object
- create a relationship
- delete an object
- invoke code-backed logic
- call an external system

OpenFoundry already models these operation families in `services/ontology-service/src/models/action_type.rs` through:

- `update_object`
- `create_link`
- `delete_object`
- `invoke_function`
- `invoke_webhook`

That is a solid foundation because it separates semantic edits from purely technical writes.

### 2. Typed input schema

An action becomes reusable when its inputs are explicit and validated.

In OpenFoundry, each action input field can already define:

- a stable name
- an optional display name
- a description
- a property type
- whether the field is required
- an optional default value

This shape is exposed by `ActionInputField` and enforced during validation in `services/ontology-service/src/handlers/actions.rs`.

The practical value of this is simple: different applications can invoke the same action contract without inventing their own private payload conventions.

Action inputs also reuse the ontology property type system rather than defining a separate type universe just for forms.

That matters because `media_reference` is already valid as an action parameter type. In the current implementation, a media reference can be passed either as:

- a plain URI or URL string
- a JSON object containing `uri` or `url`

The structured web renderer now exposes a dedicated editor for this shape, so media-aware action parameters are no longer just a backend validation detail.

### 2.5. Structured action forms

OpenFoundry now has a first-class `form_schema` on the action definition itself.

That schema is meant to describe how an action should be presented to users, not just how it should be executed by the backend.

At the moment, the model supports:

- `sections` with title, description, column count, and collapsible behavior
- per-section conditional overrides for visibility, title, description, and column layout
- `parameter_overrides` for conditional visibility, requiredness, labels, descriptions, and default values
- condition paths over action parameters and target object data

The important part is that this is not only cosmetic metadata.

The backend in `services/ontology-service/src/handlers/actions.rs` now applies parameter overrides during parameter materialization, so conditional requiredness and conditional defaults affect validation and execution, not only rendering.

On the web side, `apps/web/src/lib/components/ontology/ActionExecutor.svelte` renders those sections and overrides as a structured form, and the ontology manager page exposes `form_schema` during action authoring with a generator for base layouts.

This already moves the platform beyond a raw JSON payload editor and into a reusable action-form contract.

### 2.6. Action-backed inline edits

OpenFoundry now also has an explicit bridge between object properties and action types for inline edits.

At the schema level, direct properties can carry an `inline_edit_config` with:

- `action_type_id`
- optional `input_name`

That configuration lives on the property definition in `services/ontology-service/src/models/property.rs`.

The current behavior is intentionally narrow and governed:

- the referenced action must belong to the same object type
- it must be an `update_object` action
- the action must map the edited property from an input field
- the input field type must match the property type

Execution then happens through a dedicated inline-edit route, but it still reuses the same action engine as normal action submissions.

In practice, this means inline edits are not direct object patches masquerading as actions. They are real action executions with the same validation, authorization, audit, and notification behavior.

There is also a useful defaulting behavior in the current implementation: when the action updates several properties, other mapped inputs are prefilled from the current target object when possible, so the user can edit one field without rebuilding the full action payload manually.

On the web side, the ontology page now exposes:

- per-property configuration for the backing `update_object` action
- an inline execution surface inside Object Lab for configured properties

This is not yet equivalent to a platform-wide inline edit experience inside every list, table, and object view, but it is now a concrete first-class capability rather than an undocumented workaround.

### 3. Operation configuration

The input schema describes what the user provides. The action configuration describes how those values are turned into ontology effects.

The current codebase already supports several useful patterns:

- property mappings for `update_object`
- static patches for fixed values
- link creation based on a configured `link_type_id`
- inline function execution
- HTTP webhook invocation with method, URL, and headers

This logic lives mostly in `services/ontology-service/src/handlers/actions.rs`.

The action config is now also able to carry native notification side effects through a backward-compatible envelope:

- `operation`: the primary operation config
- `notification_side_effects`: optional notification rules executed after a successful action

That means an action can remain structurally the same while gaining side effects such as in-app, email, Slack, or Teams notifications through `notification-service`.

## Authorization and submission controls

In a mature ontology platform, the most important part of an action is often not the patch itself, but the governance around it.

OpenFoundry already contains a meaningful authorization surface for action execution:

- `permission_key`
- `required_permission_keys`
- `any_role`
- `all_roles`
- `attribute_equals`
- `allowed_markings`
- `minimum_clearance`
- `deny_guest_sessions`
- `confirmation_required`

These are modeled in `ActionAuthorizationPolicy` and checked in:

- `ensure_action_actor_permission`
- `ensure_action_target_permission`
- `ensure_confirmation_justification`

This is more than cosmetic validation. It means an action can express rules such as:

- only specific roles may run it
- only users with a given attribute may run it
- only objects with allowed markings may be targeted
- guest sessions must be blocked
- the caller must provide justification before execution

That is already close to the shape of a real operational governance layer.

## Preview, validation, and execution modes

Action systems become safer when they support a progression from intent to effect:

1. validate the inputs
2. preview the consequences
3. execute once
4. execute in batch
5. branch into what-if exploration

OpenFoundry already exposes these capabilities through routes in `services/ontology-service/src/main.rs`:

- `POST /api/v1/ontology/actions/{id}/validate`
- `POST /api/v1/ontology/actions/{id}/execute`
- `POST /api/v1/ontology/actions/{id}/execute-batch`
- `GET|POST /api/v1/ontology/actions/{id}/what-if`

The what-if branch concept is especially important. It is the point where ontology writes stop being only transactional and start becoming analytical.

## Native notification side effects

OpenFoundry now supports notification side effects as part of the action definition itself.

At the implementation level, this is handled in `services/ontology-service/src/handlers/actions.rs` by validating and dispatching `notification_side_effects` against `notification-service`.

Each notification side effect can currently define:

- `title`
- `body`
- `severity`
- `category`
- `channels`
- static `user_ids`
- `user_id_input_name`
- `target_user_property_name`
- `send_to_actor`
- `send_to_target_creator`
- `broadcast`
- optional `metadata`

The execution model is intentionally practical:

- the primary ontology mutation runs first
- notifications are emitted only after success
- delivery is best-effort, so a temporary notification outage does not roll back the action

This is already enough to cover common patterns such as:

- notify the actor that an action completed
- notify the creator or owner of the target object
- notify a user selected in the action form
- notify a user ID stored on the target object
- broadcast a system-wide operational event

Notification content also supports lightweight templating against action context, target object data, and input parameters.

## A practical design flow

When defining action types for OpenFoundry, the safest sequence is:

1. Start from one business verb, not from a generic CRUD endpoint.
2. Keep the first action narrow enough that success and failure are obvious.
3. Add typed inputs before adding UI customization.
4. Add authorization and confirmation rules before exposing the action broadly.
5. Prefer preview and what-if support when an action changes important operational state.
6. Escalate to `invoke_function` only when declarative mappings are no longer enough.
7. Use `invoke_webhook` when the action must orchestrate another system rather than only mutate ontology state.

## OpenFoundry mapping

The strongest repository signals for this area are:

- `services/ontology-service/src/models/action_type.rs`
- `services/ontology-service/src/handlers/actions.rs`
- `services/ontology-service/src/handlers/objects.rs`
- `services/ontology-service/src/domain/function_runtime.rs`
- `services/auth-service`
- `services/audit-service`

Together, these files suggest an action model that already understands:

- typed inputs
- mutation planning
- actor and target authorization
- function-backed execution
- versioned function package references with optional compatible auto-upgrade
- action-backed function package runs feeding native package monitoring
- webhook-backed execution
- batch execution
- branch-based previews

## What is still missing

Compared with a more complete ontology product, the current implementation still appears to be missing or only lightly modeled in this repository:

- a richer drag-and-drop action form designer; current authoring is still JSON-first even though `form_schema` is now first-class
- native binary upload widgets and attachment lifecycle management for action parameters; `media_reference` values are supported, but file upload itself is not yet first-class in ontology actions
- a richer action contract in `proto/ontology/action.proto`, which is currently empty
- first-class action metrics and dashboards in the ontology surface itself
- explicit conflict-resolution strategies between datasource updates and user edits
- interface-native action rules

## Related pages

- [Functions](/ontology-building/functions)
- [Object permissioning](/ontology-building/object-permissioning)
- [Indexing and materialization](/ontology-building/indexing-and-materialization)
- [Object edits and conflict resolution](/ontology-building/object-edits-and-conflict-resolution)
