use std::collections::{HashMap, HashSet};
use std::time::Instant;

use auth_middleware::{
    claims::Claims,
    jwt::{build_access_claims, encode_token},
    layer::AuthUser,
};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use reqwest::{Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        access::{clearance_rank, ensure_object_access, marking_rank, validate_marking},
        function_metrics::{FunctionPackageRunContext, record_function_package_run},
        function_runtime::{
            ResolvedInlineFunction, execute_inline_function, resolve_inline_function_config,
        },
        schema::{load_effective_properties, validate_object_properties},
        type_system::{validate_property_type, validate_property_value},
    },
    models::{
        action_type::{
            ActionAuthorizationPolicy, ActionFormCondition, ActionFormSchema, ActionInputField,
            ActionOperationKind, ActionType, ActionTypeRow, ActionWhatIfBranch,
            CreateActionTypeRequest, CreateActionWhatIfBranchRequest, ExecuteActionRequest,
            ExecuteActionResponse, ExecuteBatchActionRequest, ExecuteBatchActionResponse,
            ListActionTypesQuery, ListActionTypesResponse, ListActionWhatIfBranchesQuery,
            ListActionWhatIfBranchesResponse, UpdateActionTypeRequest, UpdateObjectActionConfig,
            ValidateActionRequest, ValidateActionResponse,
        },
        link_type::LinkType,
        property::{ExecuteInlineEditRequest, Property, PropertyInlineEditConfig},
    },
};

use super::{
    links::LinkInstance,
    objects::{ObjectInstance, load_object_instance},
};

#[derive(Debug, Deserialize)]
struct CreateLinkActionConfig {
    link_type_id: Uuid,
    target_input_name: String,
    #[serde(default = "default_source_role")]
    source_role: String,
    properties_input_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct HttpInvocationConfig {
    url: String,
    #[serde(default = "default_http_method")]
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct FunctionLinkInstruction {
    link_type_id: Uuid,
    target_object_id: Uuid,
    #[serde(default = "default_source_role")]
    source_role: String,
    properties: Option<Value>,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ActionConfigEnvelope {
    #[serde(default)]
    operation: Option<Value>,
    #[serde(default)]
    notification_side_effects: Vec<ActionNotificationSideEffectConfig>,
}

#[derive(Debug, Clone, Deserialize)]
struct ActionNotificationSideEffectConfig {
    title: String,
    body: String,
    #[serde(default)]
    severity: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    channels: Option<Vec<String>>,
    #[serde(default)]
    user_ids: Vec<Uuid>,
    #[serde(default)]
    user_id_input_name: Option<String>,
    #[serde(default)]
    target_user_property_name: Option<String>,
    #[serde(default)]
    send_to_actor: bool,
    #[serde(default)]
    send_to_target_creator: bool,
    #[serde(default)]
    broadcast: bool,
    #[serde(default)]
    metadata: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
struct InternalSendNotificationRequest {
    user_id: Option<Uuid>,
    title: String,
    body: String,
    severity: Option<String>,
    category: Option<String>,
    channels: Option<Vec<String>>,
    metadata: Option<Value>,
}

#[derive(Debug, Clone)]
struct EffectiveActionInputField {
    definition: ActionInputField,
    hidden: bool,
}

enum ActionPlan {
    UpdateObject {
        target: ObjectInstance,
        patch: Map<String, Value>,
    },
    CreateLink {
        target: ObjectInstance,
        counterpart: ObjectInstance,
        link_type: LinkType,
        properties: Option<Value>,
        source_object_id: Uuid,
        target_object_id: Uuid,
    },
    DeleteObject {
        target: ObjectInstance,
    },
    InvokeFunction {
        target: Option<ObjectInstance>,
        invocation: FunctionInvocation,
        payload: Value,
        parameters: HashMap<String, Value>,
    },
    InvokeWebhook {
        target: Option<ObjectInstance>,
        invocation: HttpInvocationConfig,
        payload: Value,
    },
}

enum FunctionInvocation {
    Http(HttpInvocationConfig),
    Inline(ResolvedInlineFunction),
}

struct ExecutedAction {
    target_object_id: Option<Uuid>,
    deleted: bool,
    preview: Value,
    object: Option<Value>,
    link: Option<Value>,
    result: Option<Value>,
}

fn default_source_role() -> String {
    "source".to_string()
}

fn default_http_method() -> String {
    "POST".to_string()
}

fn split_action_config(
    config: &Value,
) -> Result<(Value, Vec<ActionNotificationSideEffectConfig>), String> {
    let Some(config_object) = config.as_object() else {
        return Ok((config.clone(), Vec::new()));
    };

    if !config_object.contains_key("operation")
        && !config_object.contains_key("notification_side_effects")
    {
        return Ok((config.clone(), Vec::new()));
    }

    let envelope: ActionConfigEnvelope = serde_json::from_value(config.clone())
        .map_err(|error| format!("invalid action config envelope: {error}"))?;

    Ok((
        envelope.operation.unwrap_or(Value::Null),
        envelope.notification_side_effects,
    ))
}

fn validate_notification_side_effect_configs(
    configs: &[ActionNotificationSideEffectConfig],
    input_names: &HashSet<&str>,
    property_types: &HashMap<&str, &str>,
) -> Result<(), String> {
    for (index, config) in configs.iter().enumerate() {
        if config.title.trim().is_empty() {
            return Err(format!(
                "notification_side_effects[{index}] title is required"
            ));
        }
        if config.body.trim().is_empty() {
            return Err(format!(
                "notification_side_effects[{index}] body is required"
            ));
        }
        if let Some(channels) = config.channels.as_ref() {
            if channels.is_empty() {
                return Err(format!(
                    "notification_side_effects[{index}] channels must not be empty"
                ));
            }
            if channels.iter().any(|channel| channel.trim().is_empty()) {
                return Err(format!(
                    "notification_side_effects[{index}] channels cannot contain empty values"
                ));
            }
        }
        if let Some(input_name) = config.user_id_input_name.as_deref() {
            if !input_names.contains(input_name) {
                return Err(format!(
                    "notification_side_effects[{index}] references unknown input field '{input_name}'"
                ));
            }
        }
        if let Some(property_name) = config.target_user_property_name.as_deref() {
            if !property_types.contains_key(property_name) {
                return Err(format!(
                    "notification_side_effects[{index}] references unknown target property '{property_name}'"
                ));
            }
        }
        if config.user_ids.is_empty()
            && config.user_id_input_name.is_none()
            && config.target_user_property_name.is_none()
            && !config.send_to_actor
            && !config.send_to_target_creator
            && !config.broadcast
        {
            return Err(format!(
                "notification_side_effects[{index}] must define at least one recipient source"
            ));
        }
    }

    Ok(())
}

fn value_to_template_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        _ => value.to_string(),
    }
}

fn lookup_template_value(context: &Value, path: &str) -> Option<String> {
    let mut current = context;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(value_to_template_string(current))
}

fn render_template(template: &str, context: &Value) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{{") {
        rendered.push_str(&remaining[..start]);
        let after_start = &remaining[start + 2..];
        if let Some(end) = after_start.find("}}") {
            let token = after_start[..end].trim();
            if let Some(value) = lookup_template_value(context, token) {
                rendered.push_str(&value);
            }
            remaining = &after_start[end + 2..];
        } else {
            rendered.push_str(&remaining[start..]);
            return rendered;
        }
    }

    rendered.push_str(remaining);
    rendered
}

fn render_value_templates(value: &Value, context: &Value) -> Value {
    match value {
        Value::String(template) => Value::String(render_template(template, context)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| render_value_templates(item, context))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), render_value_templates(value, context)))
                .collect(),
        ),
        _ => value.clone(),
    }
}

fn lookup_path_value<'a>(context: &'a Value, path: &str) -> Option<&'a Value> {
    let mut current = context;
    for segment in path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn validate_action_form_condition(
    condition: &ActionFormCondition,
    input_names: &HashSet<&str>,
    property_types: &HashMap<&str, &str>,
    scope: &str,
) -> Result<(), String> {
    if condition.left.trim().is_empty() {
        return Err(format!("{scope} condition left path is required"));
    }

    if let Some(parameter_name) = condition.left.strip_prefix("parameters.") {
        if parameter_name.trim().is_empty() || !input_names.contains(parameter_name) {
            return Err(format!(
                "{scope} references unknown parameter path '{}'",
                condition.left
            ));
        }
    } else if let Some(property_name) = condition.left.strip_prefix("target.properties.") {
        if property_name.trim().is_empty() || !property_types.contains_key(property_name) {
            return Err(format!(
                "{scope} references unknown target property path '{}'",
                condition.left
            ));
        }
    } else if !matches!(
        condition.left.as_str(),
        "target.id" | "target.created_by" | "target.marking"
    ) {
        return Err(format!(
            "{scope} uses unsupported condition path '{}'",
            condition.left
        ));
    }

    match condition.operator.as_str() {
        "exists" | "not_exists" => Ok(()),
        "is"
        | "is_not"
        | "includes"
        | "greater_than"
        | "greater_than_or_equals"
        | "less_than"
        | "less_than_or_equals" => {
            if condition.right.is_none() {
                Err(format!(
                    "{scope} operator '{}' requires a right-hand value",
                    condition.operator
                ))
            } else {
                Ok(())
            }
        }
        other => Err(format!("{scope} uses unsupported operator '{}'", other)),
    }
}

fn validate_action_form_schema(
    form_schema: &ActionFormSchema,
    input_schema: &[ActionInputField],
    property_types: &HashMap<&str, &str>,
) -> Result<(), String> {
    let input_names = input_schema
        .iter()
        .map(|field| field.name.as_str())
        .collect::<HashSet<_>>();
    let input_types = input_schema
        .iter()
        .map(|field| (field.name.as_str(), field.property_type.as_str()))
        .collect::<HashMap<_, _>>();
    let mut seen_sections = HashSet::new();
    let mut assigned_parameters = HashSet::new();

    for (index, section) in form_schema.sections.iter().enumerate() {
        if section.id.trim().is_empty() {
            return Err(format!("form_schema.sections[{index}] id is required"));
        }
        if !seen_sections.insert(section.id.as_str()) {
            return Err(format!(
                "form_schema.sections[{index}] duplicates section id '{}'",
                section.id
            ));
        }
        if !(1..=2).contains(&section.columns) {
            return Err(format!(
                "form_schema.sections[{index}] columns must be 1 or 2"
            ));
        }
        for parameter_name in &section.parameter_names {
            if !input_names.contains(parameter_name.as_str()) {
                return Err(format!(
                    "form_schema.sections[{index}] references unknown parameter '{}'",
                    parameter_name
                ));
            }
            if !assigned_parameters.insert(parameter_name.as_str()) {
                return Err(format!(
                    "parameter '{}' is assigned to more than one form section",
                    parameter_name
                ));
            }
        }
        for (override_index, section_override) in section.overrides.iter().enumerate() {
            if let Some(columns) = section_override.columns {
                if !(1..=2).contains(&columns) {
                    return Err(format!(
                        "form_schema.sections[{index}].overrides[{override_index}] columns must be 1 or 2"
                    ));
                }
            }
            for (condition_index, condition) in section_override.conditions.iter().enumerate() {
                validate_action_form_condition(
                    condition,
                    &input_names,
                    property_types,
                    &format!(
                        "form_schema.sections[{index}].overrides[{override_index}].conditions[{condition_index}]"
                    ),
                )?;
            }
        }
    }

    for (index, parameter_override) in form_schema.parameter_overrides.iter().enumerate() {
        if parameter_override.parameter_name.trim().is_empty() {
            return Err(format!(
                "form_schema.parameter_overrides[{index}] parameter_name is required"
            ));
        }
        let property_type = input_types
            .get(parameter_override.parameter_name.as_str())
            .ok_or_else(|| {
                format!(
                    "form_schema.parameter_overrides[{index}] references unknown parameter '{}'",
                    parameter_override.parameter_name
                )
            })?;
        if let Some(default_value) = parameter_override.default_value.as_ref() {
            validate_property_value(property_type, default_value).map_err(|error| {
                format!("form_schema.parameter_overrides[{index}].default_value: {error}")
            })?;
        }
        for (condition_index, condition) in parameter_override.conditions.iter().enumerate() {
            validate_action_form_condition(
                condition,
                &input_names,
                property_types,
                &format!("form_schema.parameter_overrides[{index}].conditions[{condition_index}]"),
            )?;
        }
    }

    Ok(())
}

fn build_action_form_context(
    parameters: &HashMap<String, Value>,
    target: Option<&ObjectInstance>,
) -> Value {
    let target_value = target.map(|object| {
        json!({
            "id": object.id,
            "created_by": object.created_by,
            "marking": object.marking,
            "properties": object.properties,
        })
    });

    json!({
        "parameters": parameters,
        "target": target_value,
    })
}

fn value_exists(value: Option<&Value>) -> bool {
    value.is_some_and(|candidate| !candidate.is_null())
}

fn compare_numeric_values(left: &Value, right: &Value) -> Option<std::cmp::Ordering> {
    let left = left.as_f64()?;
    let right = right.as_f64()?;
    left.partial_cmp(&right)
}

fn matches_action_form_condition(condition: &ActionFormCondition, context: &Value) -> bool {
    let left = lookup_path_value(context, &condition.left);
    match condition.operator.as_str() {
        "exists" => value_exists(left),
        "not_exists" => !value_exists(left),
        "is" => left == condition.right.as_ref(),
        "is_not" => left != condition.right.as_ref(),
        "includes" => match (left, condition.right.as_ref()) {
            (Some(Value::Array(items)), Some(right)) => items.iter().any(|item| item == right),
            (Some(Value::String(left)), Some(Value::String(right))) => left.contains(right),
            _ => false,
        },
        "greater_than" => matches!(
            (left, condition.right.as_ref()),
            (Some(left), Some(right))
                if compare_numeric_values(left, right) == Some(std::cmp::Ordering::Greater)
        ),
        "greater_than_or_equals" => matches!(
            (left, condition.right.as_ref()),
            (Some(left), Some(right))
                if matches!(
                    compare_numeric_values(left, right),
                    Some(std::cmp::Ordering::Greater) | Some(std::cmp::Ordering::Equal)
                )
        ),
        "less_than" => matches!(
            (left, condition.right.as_ref()),
            (Some(left), Some(right))
                if compare_numeric_values(left, right) == Some(std::cmp::Ordering::Less)
        ),
        "less_than_or_equals" => matches!(
            (left, condition.right.as_ref()),
            (Some(left), Some(right))
                if matches!(
                    compare_numeric_values(left, right),
                    Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal)
                )
        ),
        _ => false,
    }
}

fn matches_action_form_conditions(conditions: &[ActionFormCondition], context: &Value) -> bool {
    conditions
        .iter()
        .all(|condition| matches_action_form_condition(condition, context))
}

fn resolve_effective_input_schema(
    input_schema: &[ActionInputField],
    form_schema: &ActionFormSchema,
    parameters: &HashMap<String, Value>,
    target: Option<&ObjectInstance>,
) -> Vec<EffectiveActionInputField> {
    let context = build_action_form_context(parameters, target);

    input_schema
        .iter()
        .cloned()
        .map(|field| {
            let mut resolved = field;
            let mut hidden = false;

            if let Some(parameter_override) =
                form_schema
                    .parameter_overrides
                    .iter()
                    .find(|override_block| {
                        override_block.parameter_name == resolved.name
                            && matches_action_form_conditions(&override_block.conditions, &context)
                    })
            {
                hidden = parameter_override.hidden.unwrap_or(false);
                if let Some(required) = parameter_override.required {
                    resolved.required = required;
                }
                if let Some(default_value) = parameter_override.default_value.clone() {
                    resolved.default_value = Some(default_value);
                }
                if parameter_override.display_name.is_some() {
                    resolved.display_name = parameter_override.display_name.clone();
                }
                if parameter_override.description.is_some() {
                    resolved.description = parameter_override.description.clone();
                }
            }

            EffectiveActionInputField {
                definition: resolved,
                hidden,
            }
        })
        .collect()
}

fn extract_uuid_values(value: &Value, source: &str) -> Result<Vec<Uuid>, String> {
    match value {
        Value::String(raw) => Uuid::parse_str(raw)
            .map(|uuid| vec![uuid])
            .map_err(|_| format!("{source} must contain UUID strings")),
        Value::Array(items) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .ok_or_else(|| format!("{source} must contain UUID strings"))
                    .and_then(|raw| {
                        Uuid::parse_str(raw)
                            .map_err(|_| format!("{source} must contain UUID strings"))
                    })
            })
            .collect(),
        Value::Null => Ok(Vec::new()),
        _ => Err(format!(
            "{source} must be a UUID string or list of UUID strings"
        )),
    }
}

fn build_notification_metadata(
    config: &ActionNotificationSideEffectConfig,
    context: &Value,
    action: &ActionType,
    executed: &ExecutedAction,
) -> Value {
    let custom = config
        .metadata
        .as_ref()
        .map(|value| render_value_templates(value, context))
        .unwrap_or_else(|| json!({}));

    match custom {
        Value::Object(mut object) => {
            object.insert("action_id".to_string(), json!(action.id));
            object.insert("action_name".to_string(), json!(action.name));
            object.insert("operation_kind".to_string(), json!(action.operation_kind));
            object.insert(
                "target_object_id".to_string(),
                json!(executed.target_object_id),
            );
            object.insert("deleted".to_string(), json!(executed.deleted));
            Value::Object(object)
        }
        other => json!({
            "action_id": action.id,
            "action_name": action.name,
            "operation_kind": action.operation_kind,
            "target_object_id": executed.target_object_id,
            "deleted": executed.deleted,
            "custom": other,
        }),
    }
}

fn invalid_action(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn db_error(message: impl Into<String>) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn forbidden(message: impl Into<String>) -> Response {
    (
        StatusCode::FORBIDDEN,
        Json(json!({ "error": message.into() })),
    )
        .into_response()
}

fn validate_action_authorization_policy(policy: &ActionAuthorizationPolicy) -> Result<(), String> {
    for permission_key in &policy.required_permission_keys {
        if permission_key.trim().is_empty() {
            return Err(
                "authorization_policy.required_permission_keys cannot contain empty values"
                    .to_string(),
            );
        }
    }
    for role in &policy.any_role {
        if role.trim().is_empty() {
            return Err("authorization_policy.any_role cannot contain empty values".to_string());
        }
    }
    for role in &policy.all_roles {
        if role.trim().is_empty() {
            return Err("authorization_policy.all_roles cannot contain empty values".to_string());
        }
    }
    for key in policy.attribute_equals.keys() {
        if key.trim().is_empty() {
            return Err(
                "authorization_policy.attribute_equals cannot contain empty keys".to_string(),
            );
        }
    }
    for marking in &policy.allowed_markings {
        validate_marking(marking)?;
    }
    if let Some(minimum_clearance) = policy.minimum_clearance.as_deref() {
        validate_marking(minimum_clearance)?;
    }

    Ok(())
}

pub(crate) fn ensure_action_actor_permission(
    claims: &Claims,
    action: &ActionType,
) -> Result<(), String> {
    if let Some(permission_key) = action.permission_key.as_deref() {
        if !claims.has_permission_key(permission_key) {
            return Err(format!(
                "forbidden: missing permission '{}'",
                permission_key
            ));
        }
    }

    for permission_key in &action.authorization_policy.required_permission_keys {
        if !claims.has_permission_key(permission_key) {
            return Err(format!(
                "forbidden: missing permission '{}'",
                permission_key
            ));
        }
    }

    if !action.authorization_policy.any_role.is_empty()
        && !action
            .authorization_policy
            .any_role
            .iter()
            .any(|role| claims.has_role(role))
    {
        return Err(format!(
            "forbidden: requires one of roles [{}]",
            action.authorization_policy.any_role.join(", ")
        ));
    }

    for role in &action.authorization_policy.all_roles {
        if !claims.has_role(role) {
            return Err(format!("forbidden: missing role '{}'", role));
        }
    }

    for (attribute_key, expected_value) in &action.authorization_policy.attribute_equals {
        match claims.attribute(attribute_key) {
            Some(actual_value) if actual_value == expected_value => {}
            Some(_) => {
                return Err(format!(
                    "forbidden: attribute '{}' does not satisfy the action policy",
                    attribute_key
                ));
            }
            None => {
                return Err(format!(
                    "forbidden: missing required attribute '{}'",
                    attribute_key
                ));
            }
        }
    }

    if action.authorization_policy.deny_guest_sessions && claims.is_guest_session() {
        return Err("forbidden: guest sessions may not execute this action".to_string());
    }

    if let Some(minimum_clearance) = action.authorization_policy.minimum_clearance.as_deref() {
        let required_clearance = marking_rank(minimum_clearance).ok_or_else(|| {
            format!(
                "forbidden: invalid action minimum_clearance '{}'",
                minimum_clearance
            )
        })?;
        if clearance_rank(claims) < required_clearance {
            return Err(format!(
                "forbidden: action requires '{}' classification clearance",
                minimum_clearance
            ));
        }
    }

    Ok(())
}

pub(crate) fn ensure_action_target_permission(
    action: &ActionType,
    target: Option<&ObjectInstance>,
) -> Result<(), String> {
    if action.authorization_policy.allowed_markings.is_empty() {
        return Ok(());
    }

    let Some(target) = target else {
        return Err("target object is required by the action authorization policy".to_string());
    };

    if action
        .authorization_policy
        .allowed_markings
        .iter()
        .any(|marking| marking.eq_ignore_ascii_case(&target.marking))
    {
        Ok(())
    } else {
        Err(format!(
            "forbidden: target marking '{}' is not allowed for this action",
            target.marking
        ))
    }
}

fn ensure_confirmation_justification(
    action: &ActionType,
    justification: Option<&str>,
) -> Result<(), String> {
    if action.confirmation_required
        && justification
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
    {
        return Err("justification is required for confirmation_required actions".to_string());
    }

    Ok(())
}

fn all_forbidden(errors: &[String]) -> bool {
    !errors.is_empty() && errors.iter().all(|error| error.starts_with("forbidden:"))
}

async fn load_action_row(
    state: &AppState,
    action_id: Uuid,
) -> Result<Option<ActionTypeRow>, sqlx::Error> {
    sqlx::query_as::<_, ActionTypeRow>(
        r#"SELECT id, name, display_name, description, object_type_id, operation_kind, input_schema,
		          form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id,
		          created_at, updated_at
		   FROM action_types WHERE id = $1"#,
    )
    .bind(action_id)
    .fetch_optional(&state.db)
    .await
}

async fn load_property_row(
    state: &AppState,
    object_type_id: Uuid,
    property_id: Uuid,
) -> Result<Option<Property>, sqlx::Error> {
    sqlx::query_as::<_, Property>(
        r#"SELECT id, object_type_id, name, display_name, description, property_type, required,
                  unique_constraint, time_dependent, default_value, validation_rules,
                  inline_edit_config, created_at, updated_at
           FROM properties
           WHERE id = $1 AND object_type_id = $2"#,
    )
    .bind(property_id)
    .bind(object_type_id)
    .fetch_optional(&state.db)
    .await
}

async fn ensure_object_type_exists(
    state: &AppState,
    object_type_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM object_types WHERE id = $1)")
        .bind(object_type_id)
        .fetch_one(&state.db)
        .await
}

fn parse_operation_kind(raw: &str) -> Result<ActionOperationKind, String> {
    serde_json::from_value::<ActionOperationKind>(Value::String(raw.to_string()))
        .map_err(|_| format!("invalid action operation kind '{raw}'"))
}

fn ensure_input_schema(input_schema: &[ActionInputField]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for field in input_schema {
        if field.name.trim().is_empty() {
            return Err("action input field name is required".to_string());
        }
        if !seen.insert(field.name.clone()) {
            return Err(format!("duplicate action input field '{}'", field.name));
        }
        validate_property_type(&field.property_type)?;
        if let Some(default_value) = &field.default_value {
            validate_property_value(&field.property_type, default_value)?;
        }
    }
    Ok(())
}

fn ensure_object_type_match(object: &ObjectInstance, expected_type: Uuid) -> Result<(), String> {
    if object.object_type_id == expected_type {
        Ok(())
    } else {
        Err("target object does not belong to the action object type".to_string())
    }
}

fn materialize_parameters(
    input_schema: &[ActionInputField],
    parameters: &Value,
    form_schema: &ActionFormSchema,
    target: Option<&ObjectInstance>,
) -> Result<HashMap<String, Value>, Vec<String>> {
    let provided = parameters.as_object().cloned().unwrap_or_default();
    let provided_parameters = provided
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<HashMap<_, _>>();
    let effective_input_schema =
        resolve_effective_input_schema(input_schema, form_schema, &provided_parameters, target);

    let mut values = HashMap::new();
    let mut errors = Vec::new();

    for effective_field in effective_input_schema {
        let EffectiveActionInputField {
            definition: field,
            hidden: _hidden,
        } = effective_field;
        let value = provided
            .get(&field.name)
            .cloned()
            .or_else(|| field.default_value.clone());

        match value {
            Some(value) => {
                if let Err(error) = validate_property_value(&field.property_type, &value) {
                    errors.push(format!("{}: {}", field.name, error));
                } else {
                    values.insert(field.name.clone(), value);
                }
            }
            None if field.required => errors.push(format!("{} is required", field.name)),
            None => {}
        }
    }

    if errors.is_empty() {
        Ok(values)
    } else {
        Err(errors)
    }
}

fn resolve_inline_edit_input_name(
    action: &ActionType,
    property_name: &str,
    inline_edit_config: &PropertyInlineEditConfig,
) -> Result<String, String> {
    let (operation_config, _) = split_action_config(&action.config)?;
    let update_config: UpdateObjectActionConfig = serde_json::from_value(operation_config)
        .map_err(|error| format!("invalid inline edit action config: {error}"))?;

    let candidates = update_config
        .property_mappings
        .into_iter()
        .filter(|mapping| mapping.property_name == property_name)
        .filter_map(|mapping| mapping.input_name)
        .collect::<Vec<_>>();

    if let Some(input_name) = inline_edit_config.input_name.as_deref() {
        if candidates.iter().any(|candidate| candidate == input_name) {
            return Ok(input_name.to_string());
        }
        return Err(format!(
            "inline edit action does not map property '{property_name}' from input '{input_name}'"
        ));
    }

    let unique_candidates = candidates.into_iter().collect::<HashSet<_>>();
    match unique_candidates.len() {
        0 => Err(format!(
            "inline edit action must map property '{property_name}' from an input field"
        )),
        1 => Ok(unique_candidates.into_iter().next().unwrap_or_default()),
        _ => Err(format!(
            "inline edit action maps property '{property_name}' from multiple input fields; configure inline_edit_config.input_name explicitly"
        )),
    }
}

fn build_inline_edit_parameters(
    action: &ActionType,
    property: &Property,
    target: &ObjectInstance,
    inline_edit_config: &PropertyInlineEditConfig,
    value: Value,
) -> Result<Value, String> {
    let editable_input_name =
        resolve_inline_edit_input_name(action, &property.name, inline_edit_config)?;
    let (operation_config, _) = split_action_config(&action.config)?;
    let update_config: UpdateObjectActionConfig = serde_json::from_value(operation_config)
        .map_err(|error| format!("invalid inline edit action config: {error}"))?;

    let mut parameters = Map::new();
    parameters.insert(editable_input_name.clone(), value);

    for mapping in update_config.property_mappings {
        let Some(input_name) = mapping.input_name else {
            continue;
        };
        if input_name == editable_input_name || parameters.contains_key(&input_name) {
            continue;
        }
        if let Some(current_value) = target.properties.get(&mapping.property_name) {
            parameters.insert(input_name, current_value.clone());
        }
    }

    Ok(Value::Object(parameters))
}

fn resolve_uuid_parameter(
    parameters: &HashMap<String, Value>,
    field_name: &str,
) -> Result<Uuid, String> {
    let value = parameters
        .get(field_name)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field_name} must be a UUID string"))?;

    Uuid::parse_str(value).map_err(|_| format!("{field_name} must be a valid UUID"))
}

fn resolve_notification_recipients(
    config: &ActionNotificationSideEffectConfig,
    parameters: &HashMap<String, Value>,
    target: Option<&ObjectInstance>,
    actor_id: Uuid,
) -> Result<(Vec<Uuid>, bool), String> {
    let mut recipients = HashSet::new();

    for user_id in &config.user_ids {
        recipients.insert(*user_id);
    }

    if let Some(input_name) = config.user_id_input_name.as_deref() {
        let value = parameters
            .get(input_name)
            .ok_or_else(|| format!("notification recipient input '{input_name}' is missing"))?;
        for user_id in extract_uuid_values(value, input_name)? {
            recipients.insert(user_id);
        }
    }

    if let Some(property_name) = config.target_user_property_name.as_deref() {
        let Some(target) = target else {
            return Err(format!(
                "notification recipient target property '{property_name}' requires a target object"
            ));
        };
        let value = target
            .properties
            .get(property_name)
            .ok_or_else(|| format!("target property '{property_name}' is missing"))?;
        for user_id in extract_uuid_values(value, property_name)? {
            recipients.insert(user_id);
        }
    }

    if config.send_to_actor {
        recipients.insert(actor_id);
    }

    if config.send_to_target_creator {
        let Some(target) = target else {
            return Err(
                "send_to_target_creator requires a target object for notification side effect"
                    .to_string(),
            );
        };
        recipients.insert(target.created_by);
    }

    if recipients.is_empty() && !config.broadcast {
        return Err("notification side effect resolved no recipients".to_string());
    }

    let mut recipients = recipients.into_iter().collect::<Vec<_>>();
    recipients.sort_unstable();

    Ok((recipients, config.broadcast))
}

fn validate_notification_resolution(
    configs: &[ActionNotificationSideEffectConfig],
    parameters: &HashMap<String, Value>,
    target: Option<&ObjectInstance>,
    actor_id: Uuid,
) -> Result<(), String> {
    for config in configs {
        resolve_notification_recipients(config, parameters, target, actor_id)?;
    }

    Ok(())
}

fn validate_http_invocation_config(config: &Value) -> Result<HttpInvocationConfig, String> {
    let mut invocation: HttpInvocationConfig = serde_json::from_value(config.clone())
        .map_err(|e| format!("invalid HTTP action config: {e}"))?;

    if invocation.url.trim().is_empty() {
        return Err("HTTP action config requires a non-empty url".to_string());
    }

    let parsed_url = Url::parse(&invocation.url)
        .map_err(|e| format!("invalid HTTP action url '{}': {e}", invocation.url))?;
    if parsed_url.scheme() != "http" && parsed_url.scheme() != "https" {
        return Err("HTTP action url must use http or https".to_string());
    }

    invocation.method = invocation.method.trim().to_uppercase();
    let method = Method::from_bytes(invocation.method.as_bytes())
        .map_err(|_| format!("invalid HTTP action method '{}'", invocation.method))?;
    if !matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        return Err("HTTP action method must be POST, PUT, or PATCH".to_string());
    }

    Ok(invocation)
}

fn build_http_payload(
    action: &ActionType,
    target: Option<&ObjectInstance>,
    parameters: &HashMap<String, Value>,
) -> Value {
    json!({
        "action": {
            "id": action.id,
            "name": &action.name,
            "display_name": &action.display_name,
            "object_type_id": action.object_type_id,
            "operation_kind": &action.operation_kind,
        },
        "target_object": target,
        "parameters": parameters,
    })
}

async fn invoke_http_action(
    state: &AppState,
    invocation: &HttpInvocationConfig,
    payload: &Value,
) -> Result<Value, String> {
    let method = Method::from_bytes(invocation.method.as_bytes())
        .map_err(|_| format!("invalid HTTP action method '{}'", invocation.method))?;
    let url = Url::parse(&invocation.url)
        .map_err(|e| format!("invalid HTTP action url '{}': {e}", invocation.url))?;

    let mut request = state.http_client.request(method, url);
    for (header_name, header_value) in &invocation.headers {
        request = request.header(header_name, header_value);
    }

    let response = request
        .json(payload)
        .send()
        .await
        .map_err(|e| format!("HTTP action request failed: {e}"))?;
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("failed to read HTTP action response: {e}"))?;

    if !status.is_success() {
        let detail = if text.trim().is_empty() {
            status.to_string()
        } else {
            text.clone()
        };
        return Err(format!("HTTP action returned {}: {}", status, detail));
    }

    if text.trim().is_empty() {
        Ok(Value::Null)
    } else {
        Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
    }
}

async fn apply_object_patch(
    state: &AppState,
    target: &ObjectInstance,
    patch_value: &Value,
) -> Result<ObjectInstance, String> {
    let patch = patch_value
        .as_object()
        .ok_or_else(|| "object_patch must be a JSON object".to_string())?;
    let definitions = load_effective_properties(&state.db, target.object_type_id)
        .await
        .map_err(|e| format!("failed to load property definitions: {e}"))?;
    let property_types = definitions
        .iter()
        .map(|property| (property.name.as_str(), property.property_type.as_str()))
        .collect::<HashMap<_, _>>();

    let mut next_properties = target.properties.as_object().cloned().unwrap_or_default();
    for (property_name, value) in patch {
        let property_type = property_types
            .get(property_name.as_str())
            .ok_or_else(|| format!("unknown property '{property_name}' in object_patch"))?;
        validate_property_value(property_type, value)
            .map_err(|e| format!("{}: {}", property_name, e))?;
        next_properties.insert(property_name.clone(), value.clone());
    }

    let normalized = validate_object_properties(&definitions, &Value::Object(next_properties))?;

    sqlx::query_as::<_, ObjectInstance>(
        r#"UPDATE object_instances
		   SET properties = $2::jsonb,
		       updated_at = NOW()
		   WHERE id = $1
		   RETURNING id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at"#,
    )
    .bind(target.id)
    .bind(normalized)
    .fetch_one(&state.db)
    .await
    .map_err(|e| format!("failed to apply object patch: {e}"))
}

async fn create_link_from_instruction(
    state: &AppState,
    claims: &Claims,
    actor_id: Uuid,
    target: &ObjectInstance,
    instruction: &FunctionLinkInstruction,
) -> Result<LinkInstance, String> {
    let counterpart = load_object_instance(&state.db, instruction.target_object_id)
        .await
        .map_err(|e| format!("failed to load linked object: {e}"))?
        .ok_or_else(|| "linked object was not found".to_string())?;
    ensure_object_access(claims, &counterpart)?;

    let link_type = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types WHERE id = $1")
        .bind(instruction.link_type_id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| format!("failed to load link type: {e}"))?
        .ok_or_else(|| "configured link type was not found".to_string())?;

    let expected_target_type = if instruction.source_role == "source" {
        link_type.source_type_id
    } else {
        link_type.target_type_id
    };
    if target.object_type_id != expected_target_type {
        return Err("target object does not match configured link endpoint".to_string());
    }

    let (source_object_id, target_object_id, expected_counterpart_type) =
        if instruction.source_role == "source" {
            (target.id, counterpart.id, link_type.target_type_id)
        } else {
            (counterpart.id, target.id, link_type.source_type_id)
        };

    if counterpart.object_type_id != expected_counterpart_type {
        return Err("linked object does not match configured link type".to_string());
    }

    sqlx::query_as::<_, LinkInstance>(
		r#"INSERT INTO link_instances (id, link_type_id, source_object_id, target_object_id, properties, created_by)
		   VALUES ($1, $2, $3, $4, $5, $6)
		   RETURNING *"#,
	)
	.bind(Uuid::now_v7())
	.bind(link_type.id)
	.bind(source_object_id)
	.bind(target_object_id)
	.bind(instruction.properties.clone())
	.bind(actor_id)
	.fetch_one(&state.db)
	.await
	.map_err(|e| format!("failed to create link from function response: {e}"))
}

fn derive_function_effects(
    response: &Value,
) -> Result<
    (
        Option<Value>,
        Option<Value>,
        Option<FunctionLinkInstruction>,
        bool,
    ),
    String,
> {
    let Some(object) = response.as_object() else {
        return Ok((Some(response.clone()), None, None, false));
    };

    let output = object
        .get("output")
        .filter(|value| !value.is_null())
        .cloned();
    let object_patch = object
        .get("object_patch")
        .filter(|value| !value.is_null())
        .cloned();
    let link = object
        .get("link")
        .filter(|value| !value.is_null())
        .cloned()
        .map(serde_json::from_value::<FunctionLinkInstruction>)
        .transpose()
        .map_err(|e| format!("invalid function link instruction: {e}"))?;
    let delete_object = object
        .get("delete_object")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if delete_object && (object_patch.is_some() || link.is_some()) {
        return Err(
            "function response cannot request delete_object together with object_patch or link"
                .to_string(),
        );
    }

    let result = if output.is_some() {
        output
    } else if object_patch.is_none() && link.is_none() && !delete_object {
        Some(response.clone())
    } else {
        None
    };

    Ok((result, object_patch, link, delete_object))
}

async fn validate_action_definition(
    state: &AppState,
    object_type_id: Uuid,
    operation_kind_raw: &str,
    input_schema: &[ActionInputField],
    form_schema: &ActionFormSchema,
    config: &Value,
    authorization_policy: &ActionAuthorizationPolicy,
) -> Result<ActionOperationKind, String> {
    if !ensure_object_type_exists(state, object_type_id)
        .await
        .map_err(|e| format!("failed to validate object type: {e}"))?
    {
        return Err("referenced object type does not exist".to_string());
    }

    ensure_input_schema(input_schema)?;
    validate_action_authorization_policy(authorization_policy)?;
    let operation_kind = parse_operation_kind(operation_kind_raw)?;
    let (operation_config, notification_side_effects) = split_action_config(config)?;
    let input_names = input_schema
        .iter()
        .map(|field| field.name.as_str())
        .collect::<HashSet<_>>();
    let effective_properties = load_effective_properties(&state.db, object_type_id)
        .await
        .map_err(|e| format!("failed to load property definitions: {e}"))?;
    let property_types = effective_properties
        .iter()
        .map(|property| (property.name.as_str(), property.property_type.as_str()))
        .collect::<HashMap<_, _>>();

    validate_action_form_schema(form_schema, input_schema, &property_types)?;
    validate_notification_side_effect_configs(
        &notification_side_effects,
        &input_names,
        &property_types,
    )?;

    match operation_kind {
        ActionOperationKind::UpdateObject => {
            let cfg: UpdateObjectActionConfig = serde_json::from_value(operation_config)
                .map_err(|e| format!("invalid update_object action config: {e}"))?;
            if cfg.property_mappings.is_empty() && cfg.static_patch.as_ref().is_none() {
                return Err(
                    "update_object action requires property_mappings or static_patch".to_string(),
                );
            }

            for mapping in cfg.property_mappings {
                if mapping.property_name.trim().is_empty() {
                    return Err("property_name is required for update_object mappings".to_string());
                }

                let property_type = property_types
                    .get(mapping.property_name.as_str())
                    .ok_or_else(|| {
                        format!(
                            "unknown property '{}' in update_object action config",
                            mapping.property_name
                        )
                    })?;

                match (&mapping.input_name, &mapping.value) {
                    (Some(input_name), None) => {
                        if !input_names.contains(input_name.as_str()) {
                            return Err(format!(
                                "unknown input field '{input_name}' in action config"
                            ));
                        }
                    }
                    (None, Some(value)) => {
                        validate_property_value(property_type, value)
                            .map_err(|error| format!("{}: {}", mapping.property_name, error))?;
                    }
                    _ => {
                        return Err(
                            "each update_object mapping needs either input_name or value"
                                .to_string(),
                        );
                    }
                }
            }

            if let Some(static_patch) = cfg.static_patch {
                let values = static_patch
                    .as_object()
                    .ok_or_else(|| "static_patch must be a JSON object".to_string())?;
                for (property_name, value) in values {
                    let property_type =
                        property_types.get(property_name.as_str()).ok_or_else(|| {
                            format!("unknown property '{property_name}' in static_patch")
                        })?;
                    validate_property_value(property_type, value)
                        .map_err(|error| format!("{}: {}", property_name, error))?;
                }
            }
        }
        ActionOperationKind::CreateLink => {
            let cfg: CreateLinkActionConfig = serde_json::from_value(operation_config)
                .map_err(|e| format!("invalid create_link action config: {e}"))?;
            if !input_names.contains(cfg.target_input_name.as_str()) {
                return Err(format!(
                    "target_input_name '{}' does not match any input schema field",
                    cfg.target_input_name
                ));
            }
            if let Some(properties_input_name) = cfg.properties_input_name.as_ref() {
                if !input_names.contains(properties_input_name.as_str()) {
                    return Err(format!(
                        "properties_input_name '{}' does not match any input schema field",
                        properties_input_name
                    ));
                }
            }
            if cfg.source_role != "source" && cfg.source_role != "target" {
                return Err("create_link source_role must be 'source' or 'target'".to_string());
            }
            let link_type = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types WHERE id = $1")
                .bind(cfg.link_type_id)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| format!("failed to validate link type: {e}"))?
                .ok_or_else(|| "referenced link type does not exist".to_string())?;
            let expected_type = if cfg.source_role == "source" {
                link_type.source_type_id
            } else {
                link_type.target_type_id
            };
            if expected_type != object_type_id {
                return Err(
                    "action object_type_id does not match configured link endpoint".to_string(),
                );
            }
        }
        ActionOperationKind::DeleteObject => {
            if !operation_config.is_null()
                && !operation_config
                    .as_object()
                    .map(|value| value.is_empty())
                    .unwrap_or(false)
            {
                return Err("delete_object actions do not accept config".to_string());
            }
        }
        ActionOperationKind::InvokeFunction => {
            if resolve_inline_function_config(state, &operation_config)
                .await?
                .is_none()
            {
                validate_http_invocation_config(&operation_config)?;
            }
        }
        ActionOperationKind::InvokeWebhook => {
            validate_http_invocation_config(&operation_config)?;
        }
    }

    Ok(operation_kind)
}

async fn load_and_authorize_target(
    state: &AppState,
    claims: &Claims,
    target_object_id: Uuid,
    object_type_id: Uuid,
) -> Result<ObjectInstance, Vec<String>> {
    let target = load_object_instance(&state.db, target_object_id)
        .await
        .map_err(|e| vec![format!("failed to load target object: {e}")])?
        .ok_or_else(|| vec!["target object was not found".to_string()])?;
    ensure_object_type_match(&target, object_type_id).map_err(|e| vec![e])?;
    ensure_object_access(claims, &target).map_err(|e| vec![e])?;
    Ok(target)
}

async fn plan_action(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    request: &ValidateActionRequest,
) -> Result<ActionPlan, Vec<String>> {
    let materialization_target = match request.target_object_id {
        Some(target_object_id) => Some(
            load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                .await?,
        ),
        None => None,
    };
    let parameters = materialize_parameters(
        &action.input_schema,
        &request.parameters,
        &action.form_schema,
        materialization_target.as_ref(),
    )?;
    let operation_kind = match parse_operation_kind(&action.operation_kind) {
        Ok(kind) => kind,
        Err(error) => return Err(vec![error]),
    };
    let (operation_config, notification_side_effects) = match split_action_config(&action.config) {
        Ok(parts) => parts,
        Err(error) => return Err(vec![error]),
    };

    match operation_kind {
        ActionOperationKind::UpdateObject => {
            let target_object_id = request.target_object_id.ok_or_else(|| {
                vec!["target_object_id is required for update_object actions".to_string()]
            })?;
            let target =
                load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                    .await?;
            ensure_action_target_permission(action, Some(&target)).map_err(|e| vec![e])?;

            let cfg: UpdateObjectActionConfig = serde_json::from_value(operation_config)
                .map_err(|e| vec![format!("invalid action config: {e}")])?;
            let property_types = load_effective_properties(&state.db, action.object_type_id)
                .await
                .map_err(|e| vec![format!("failed to load property definitions: {e}")])?
                .into_iter()
                .map(|property| (property.name, property.property_type))
                .collect::<HashMap<_, _>>();

            let mut patch = Map::new();
            for mapping in cfg.property_mappings {
                let property_type = property_types
                    .get(mapping.property_name.as_str())
                    .ok_or_else(|| {
                        vec![format!(
                            "unknown property '{}' in update_object action config",
                            mapping.property_name
                        )]
                    })?;
                let value = if let Some(input_name) = mapping.input_name {
                    parameters.get(&input_name).cloned().ok_or_else(|| {
                        vec![format!("missing input '{input_name}' for property mapping")]
                    })?
                } else {
                    mapping.value.unwrap_or(Value::Null)
                };

                validate_property_value(property_type, &value)
                    .map_err(|e| vec![format!("{}: {}", mapping.property_name, e)])?;
                patch.insert(mapping.property_name, value);
            }

            if let Some(static_patch) = cfg.static_patch {
                if let Some(values) = static_patch.as_object() {
                    for (property_name, value) in values {
                        let property_type =
                            property_types.get(property_name.as_str()).ok_or_else(|| {
                                vec![format!(
                                    "unknown property '{property_name}' in static_patch"
                                )]
                            })?;
                        validate_property_value(property_type, value)
                            .map_err(|e| vec![format!("{}: {}", property_name, e)])?;
                        patch.insert(property_name.to_string(), value.clone());
                    }
                }
            }

            validate_notification_resolution(
                &notification_side_effects,
                &parameters,
                Some(&target),
                claims.sub,
            )
            .map_err(|error| vec![error])?;

            Ok(ActionPlan::UpdateObject { target, patch })
        }
        ActionOperationKind::CreateLink => {
            let target_object_id = request.target_object_id.ok_or_else(|| {
                vec!["target_object_id is required for create_link actions".to_string()]
            })?;
            let target =
                load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                    .await?;
            ensure_action_target_permission(action, Some(&target)).map_err(|e| vec![e])?;

            let cfg: CreateLinkActionConfig = serde_json::from_value(operation_config)
                .map_err(|e| vec![format!("invalid action config: {e}")])?;
            let counterpart_id =
                resolve_uuid_parameter(&parameters, &cfg.target_input_name).map_err(|e| vec![e])?;
            let counterpart = load_object_instance(&state.db, counterpart_id)
                .await
                .map_err(|e| vec![format!("failed to load linked object: {e}")])?
                .ok_or_else(|| vec!["linked object was not found".to_string()])?;
            ensure_object_access(claims, &counterpart).map_err(|e| vec![e])?;
            let link_type = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types WHERE id = $1")
                .bind(cfg.link_type_id)
                .fetch_optional(&state.db)
                .await
                .map_err(|e| vec![format!("failed to load link type: {e}")])?
                .ok_or_else(|| vec!["configured link type was not found".to_string()])?;

            let (source_object_id, target_link_object_id, expected_counterpart_type) =
                if cfg.source_role == "source" {
                    (target.id, counterpart.id, link_type.target_type_id)
                } else {
                    (counterpart.id, target.id, link_type.source_type_id)
                };

            if counterpart.object_type_id != expected_counterpart_type {
                return Err(vec![
                    "linked object does not match configured link type".to_string(),
                ]);
            }

            let properties = cfg
                .properties_input_name
                .and_then(|field_name| parameters.get(&field_name).cloned());

            validate_notification_resolution(
                &notification_side_effects,
                &parameters,
                Some(&target),
                claims.sub,
            )
            .map_err(|error| vec![error])?;

            Ok(ActionPlan::CreateLink {
                target,
                counterpart,
                link_type,
                properties,
                source_object_id,
                target_object_id: target_link_object_id,
            })
        }
        ActionOperationKind::DeleteObject => {
            let target_object_id = request.target_object_id.ok_or_else(|| {
                vec!["target_object_id is required for delete_object actions".to_string()]
            })?;
            let target =
                load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                    .await?;
            ensure_action_target_permission(action, Some(&target)).map_err(|e| vec![e])?;

            validate_notification_resolution(
                &notification_side_effects,
                &parameters,
                Some(&target),
                claims.sub,
            )
            .map_err(|error| vec![error])?;

            Ok(ActionPlan::DeleteObject { target })
        }
        ActionOperationKind::InvokeFunction => {
            let target = match request.target_object_id {
                Some(target_object_id) => Some(
                    load_and_authorize_target(
                        state,
                        claims,
                        target_object_id,
                        action.object_type_id,
                    )
                    .await?,
                ),
                None => None,
            };
            ensure_action_target_permission(action, target.as_ref()).map_err(|e| vec![e])?;
            let payload = build_http_payload(action, target.as_ref(), &parameters);
            let invocation = match resolve_inline_function_config(state, &operation_config).await {
                Ok(Some(config)) => FunctionInvocation::Inline(config),
                Ok(None) => FunctionInvocation::Http(
                    validate_http_invocation_config(&operation_config).map_err(|e| vec![e])?,
                ),
                Err(error) => return Err(vec![error]),
            };

            validate_notification_resolution(
                &notification_side_effects,
                &parameters,
                target.as_ref(),
                claims.sub,
            )
            .map_err(|error| vec![error])?;

            Ok(ActionPlan::InvokeFunction {
                target,
                invocation,
                payload,
                parameters,
            })
        }
        ActionOperationKind::InvokeWebhook => {
            let target = match request.target_object_id {
                Some(target_object_id) => Some(
                    load_and_authorize_target(
                        state,
                        claims,
                        target_object_id,
                        action.object_type_id,
                    )
                    .await?,
                ),
                None => None,
            };
            ensure_action_target_permission(action, target.as_ref()).map_err(|e| vec![e])?;
            let payload = build_http_payload(action, target.as_ref(), &parameters);
            let invocation =
                validate_http_invocation_config(&operation_config).map_err(|e| vec![e])?;

            validate_notification_resolution(
                &notification_side_effects,
                &parameters,
                target.as_ref(),
                claims.sub,
            )
            .map_err(|error| vec![error])?;

            Ok(ActionPlan::InvokeWebhook {
                target,
                invocation,
                payload,
            })
        }
    }
}

fn plan_preview(plan: &ActionPlan) -> Value {
    match plan {
        ActionPlan::UpdateObject { target, patch } => json!({
            "kind": "update_object",
            "target_object_id": target.id,
            "patch": patch,
        }),
        ActionPlan::CreateLink {
            target,
            counterpart,
            link_type,
            properties,
            source_object_id,
            target_object_id,
        } => json!({
            "kind": "create_link",
            "target_object_id": target.id,
            "counterpart_object_id": counterpart.id,
            "link_type_id": link_type.id,
            "source_object_id": source_object_id,
            "linked_object_id": target_object_id,
            "properties": properties,
        }),
        ActionPlan::DeleteObject { target } => json!({
            "kind": "delete_object",
            "target_object_id": target.id,
        }),
        ActionPlan::InvokeFunction {
            target,
            invocation,
            payload,
            ..
        } => match invocation {
            FunctionInvocation::Http(invocation) => json!({
                "kind": "invoke_function",
                "runtime": "http",
                "target_object_id": target.as_ref().map(|object| object.id),
                "request": {
                    "url": &invocation.url,
                    "method": &invocation.method,
                    "headers": &invocation.headers,
                    "payload": payload,
                },
            }),
            FunctionInvocation::Inline(invocation) => json!({
                "kind": "invoke_function",
                "runtime": invocation.runtime_name(),
                "target_object_id": target.as_ref().map(|object| object.id),
                "request": {
                    "payload": payload,
                },
                "source_length": invocation.source_len(),
                "capabilities": invocation.capabilities,
                "function_package": invocation.package,
            }),
        },
        ActionPlan::InvokeWebhook {
            target,
            invocation,
            payload,
        } => json!({
            "kind": "invoke_webhook",
            "target_object_id": target.as_ref().map(|object| object.id),
            "request": {
                "url": &invocation.url,
                "method": &invocation.method,
                "headers": &invocation.headers,
                "payload": payload,
            },
        }),
    }
}

fn target_snapshot_from_plan(plan: &ActionPlan) -> Option<ObjectInstance> {
    match plan {
        ActionPlan::UpdateObject { target, .. }
        | ActionPlan::CreateLink { target, .. }
        | ActionPlan::DeleteObject { target }
        | ActionPlan::InvokeFunction {
            target: Some(target),
            ..
        }
        | ActionPlan::InvokeWebhook {
            target: Some(target),
            ..
        } => Some(target.clone()),
        _ => None,
    }
}

async fn simulate_target_after_preview(
    state: &AppState,
    target: &ObjectInstance,
    preview: &Value,
) -> Result<Option<Value>, String> {
    if preview
        .get("kind")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "delete_object")
    {
        return Ok(None);
    }

    let mut merged = target.properties.as_object().cloned().unwrap_or_default();
    if let Some(patch) = preview.get("patch").and_then(Value::as_object) {
        for (key, value) in patch {
            merged.insert(key.clone(), value.clone());
        }
    }

    let definitions = load_effective_properties(&state.db, target.object_type_id)
        .await
        .map_err(|error| format!("failed to load property definitions: {error}"))?;
    let normalized = validate_object_properties(&definitions, &Value::Object(merged))
        .map_err(|error| format!("invalid simulated action branch: {error}"))?;

    let mut simulated = target.clone();
    simulated.properties = normalized;
    simulated.updated_at = chrono::Utc::now();
    Ok(Some(json!(simulated)))
}

fn issue_service_token(state: &AppState, claims: &Claims) -> Result<String, String> {
    let service_claims = build_access_claims(
        &state.jwt_config,
        Uuid::now_v7(),
        "ontology-service@internal.openfoundry",
        "ontology-service",
        vec!["admin".to_string()],
        vec!["*:*".to_string()],
        claims.org_id,
        json!({
            "service": "ontology-service",
            "classification_clearance": "pii",
            "impersonated_actor_id": claims.sub,
        }),
        vec!["service".to_string()],
    );
    let token = encode_token(&state.jwt_config, &service_claims)
        .map_err(|error| format!("failed to issue service token for audit: {error}"))?;
    Ok(format!("Bearer {token}"))
}

fn classification_for_target(target: Option<&ObjectInstance>) -> &'static str {
    match target.map(|object| object.marking.as_str()) {
        Some("confidential") => "confidential",
        Some("pii") => "pii",
        _ => "public",
    }
}

async fn emit_action_audit_event(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    target: Option<&ObjectInstance>,
    target_object_id: Option<Uuid>,
    status: &str,
    severity: &str,
    message: Option<&str>,
    justification: Option<&str>,
    parameters: &Value,
    preview: Option<&Value>,
    result: Option<&Value>,
) -> Result<(), String> {
    let token = issue_service_token(state, claims)?;
    let url = format!(
        "{}/api/v1/audit/events",
        state.audit_service_url.trim_end_matches('/')
    );
    let resource_id = target_object_id.unwrap_or(action.id).to_string();
    let metadata = json!({
        "action_id": action.id,
        "action_name": &action.name,
        "operation_kind": &action.operation_kind,
        "object_type_id": action.object_type_id,
        "permission_key": &action.permission_key,
        "authorization_policy": &action.authorization_policy,
        "target_object_id": target_object_id,
        "justification": justification,
        "parameters": parameters,
        "preview": preview,
        "result": result,
        "message": message,
        "actor_id": claims.sub,
        "actor_roles": &claims.roles,
        "organization_id": claims.org_id,
    });

    let response = state
        .http_client
        .post(url)
        .header("authorization", token)
        .json(&json!({
            "source_service": "ontology-service",
            "channel": "api",
            "actor": &claims.email,
            "action": "ontology.action.execute",
            "resource_type": if target_object_id.is_some() {
                "ontology_object"
            } else {
                "ontology_action"
            },
            "resource_id": resource_id,
            "status": status,
            "severity": severity,
            "classification": classification_for_target(target),
            "subject_id": claims.sub.to_string(),
            "metadata": metadata,
            "labels": ["ontology", "action", status, action.operation_kind.as_str()],
        }))
        .send()
        .await
        .map_err(|error| format!("failed to send audit event: {error}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("audit service returned {status}: {body}"))
    }
}

async fn send_notification_request(
    state: &AppState,
    request: &InternalSendNotificationRequest,
) -> Result<(), String> {
    let url = format!(
        "{}/internal/notifications",
        state.notification_service_url.trim_end_matches('/')
    );

    let response = state
        .http_client
        .post(url)
        .json(request)
        .send()
        .await
        .map_err(|error| format!("failed to send action notification: {error}"))?;

    if response.status().is_success() {
        Ok(())
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("notification service returned {status}: {body}"))
    }
}

async fn emit_action_notifications(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    target: Option<&ObjectInstance>,
    parameters: &Value,
    justification: Option<&str>,
    executed: &ExecutedAction,
) -> Result<(), String> {
    let (_, notification_configs) = split_action_config(&action.config)?;
    if notification_configs.is_empty() {
        return Ok(());
    }

    let parameters_map = materialize_parameters(
        &action.input_schema,
        parameters,
        &action.form_schema,
        target,
    )
    .map_err(|errors| errors.join("; "))?;

    let context = json!({
        "action": {
            "id": action.id,
            "name": &action.name,
            "display_name": &action.display_name,
            "description": &action.description,
            "operation_kind": &action.operation_kind,
            "object_type_id": action.object_type_id,
        },
        "actor": {
            "id": claims.sub,
            "email": &claims.email,
            "roles": &claims.roles,
            "organization_id": claims.org_id,
        },
        "target": target,
        "parameters": &parameters_map,
        "justification": justification,
        "preview": &executed.preview,
        "execution": {
            "target_object_id": executed.target_object_id,
            "deleted": executed.deleted,
        },
        "object": &executed.object,
        "link": &executed.link,
        "result": &executed.result,
    });

    for config in notification_configs {
        let (recipients, broadcast) =
            resolve_notification_recipients(&config, &parameters_map, target, claims.sub)?;
        let title = render_template(&config.title, &context);
        let body = render_template(&config.body, &context);
        let metadata = build_notification_metadata(&config, &context, action, executed);

        let channels = config.channels.clone();
        let severity = config.severity.clone();
        let category = config.category.clone();

        if broadcast {
            send_notification_request(
                state,
                &InternalSendNotificationRequest {
                    user_id: None,
                    title: title.clone(),
                    body: body.clone(),
                    severity: severity.clone(),
                    category: category.clone(),
                    channels: channels.clone(),
                    metadata: Some(metadata.clone()),
                },
            )
            .await?;
        }

        for user_id in recipients {
            send_notification_request(
                state,
                &InternalSendNotificationRequest {
                    user_id: Some(user_id),
                    title: title.clone(),
                    body: body.clone(),
                    severity: severity.clone(),
                    category: category.clone(),
                    channels: channels.clone(),
                    metadata: Some(metadata.clone()),
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn execute_plan(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    justification: Option<&str>,
    plan: ActionPlan,
) -> Result<ExecutedAction, String> {
    let preview = plan_preview(&plan);

    match plan {
        ActionPlan::UpdateObject { target, patch } => {
            let updated = apply_object_patch(state, &target, &Value::Object(patch)).await?;
            Ok(ExecutedAction {
                target_object_id: Some(target.id),
                deleted: false,
                preview,
                object: Some(json!(updated)),
                link: None,
                result: None,
            })
        }
        ActionPlan::CreateLink {
            target,
            link_type,
            properties,
            source_object_id,
            target_object_id,
            ..
        } => {
            let link = sqlx::query_as::<_, LinkInstance>(
				r#"INSERT INTO link_instances (id, link_type_id, source_object_id, target_object_id, properties, created_by)
				   VALUES ($1, $2, $3, $4, $5, $6)
				   RETURNING *"#,
			)
			.bind(Uuid::now_v7())
			.bind(link_type.id)
			.bind(source_object_id)
			.bind(target_object_id)
			.bind(properties)
			.bind(claims.sub)
			.fetch_one(&state.db)
			.await
            .map_err(|e| format!("failed to execute create_link action: {e}"))?;

            Ok(ExecutedAction {
                target_object_id: Some(target.id),
                deleted: false,
                preview,
                object: None,
                link: Some(json!(link)),
                result: None,
            })
        }
        ActionPlan::DeleteObject { target } => {
            let result = sqlx::query("DELETE FROM object_instances WHERE id = $1")
                .bind(target.id)
                .execute(&state.db)
                .await
                .map_err(|e| format!("failed to execute delete_object action: {e}"))?;

            if result.rows_affected() == 0 {
                return Err("target object no longer exists".to_string());
            }

            Ok(ExecutedAction {
                target_object_id: Some(target.id),
                deleted: true,
                preview,
                object: None,
                link: None,
                result: None,
            })
        }
        ActionPlan::InvokeWebhook {
            target,
            invocation,
            payload,
        } => {
            let result = invoke_http_action(state, &invocation, &payload).await?;
            Ok(ExecutedAction {
                target_object_id: target.as_ref().map(|object| object.id),
                deleted: false,
                preview,
                object: None,
                link: None,
                result: Some(result),
            })
        }
        ActionPlan::InvokeFunction {
            target,
            invocation,
            payload,
            parameters,
        } => match &invocation {
            FunctionInvocation::Http(invocation) => {
                let response = invoke_http_action(state, invocation, &payload).await?;

                let (result, object_patch, link_instruction, delete_object) =
                    derive_function_effects(&response)
                        .map_err(|e| format!("invalid function response: {e}"))?;

                let Some(target_object) = target.as_ref() else {
                    if object_patch.is_some() || link_instruction.is_some() || delete_object {
                        return Err(
                                "function response requested ontology mutations but target_object_id was not provided"
                                    .to_string(),
                            );
                    }

                    return Ok(ExecutedAction {
                        target_object_id: None,
                        deleted: false,
                        preview,
                        object: None,
                        link: None,
                        result: result.or(Some(response)),
                    });
                };

                let object = match object_patch {
                    Some(patch) => Some(json!(
                        apply_object_patch(state, target_object, &patch).await?
                    )),
                    None => None,
                };

                let link = match link_instruction {
                    Some(instruction) => Some(json!(
                        create_link_from_instruction(
                            state,
                            claims,
                            claims.sub,
                            target_object,
                            &instruction
                        )
                        .await?
                    )),
                    None => None,
                };

                let deleted = if delete_object {
                    let result = sqlx::query("DELETE FROM object_instances WHERE id = $1")
                        .bind(target_object.id)
                        .execute(&state.db)
                        .await
                        .map_err(|e| {
                            format!("failed to delete object from function response: {e}")
                        })?;
                    if result.rows_affected() == 0 {
                        return Err("target object no longer exists".to_string());
                    }
                    true
                } else {
                    false
                };

                Ok(ExecutedAction {
                    target_object_id: Some(target_object.id),
                    deleted,
                    preview,
                    object,
                    link,
                    result: result.or(Some(response)),
                })
            }
            FunctionInvocation::Inline(config) => {
                let started_at = Utc::now();
                let timer = Instant::now();
                let package = config.package.clone();
                let outcome: Result<ExecutedAction, String> = async {
                        let response = execute_inline_function(
                            state,
                            claims,
                            action,
                            target.as_ref(),
                            &parameters,
                            config,
                            justification,
                        )
                        .await?;

                        let (result, object_patch, link_instruction, delete_object) =
                            derive_function_effects(&response)
                                .map_err(|e| format!("invalid function response: {e}"))?;

                        let Some(target_object) = target.as_ref() else {
                            if object_patch.is_some()
                                || link_instruction.is_some()
                                || delete_object
                            {
                                return Err(
                                    "function response requested ontology mutations but target_object_id was not provided"
                                        .to_string(),
                                );
                            }

                            return Ok(ExecutedAction {
                                target_object_id: None,
                                deleted: false,
                                preview: preview.clone(),
                                object: None,
                                link: None,
                                result: result.or(Some(response)),
                            });
                        };

                        let object = match object_patch {
                            Some(patch) => Some(json!(
                                apply_object_patch(state, target_object, &patch).await?
                            )),
                            None => None,
                        };

                        let link = match link_instruction {
                            Some(instruction) => Some(json!(
                                create_link_from_instruction(
                                    state,
                                    claims,
                                    claims.sub,
                                    target_object,
                                    &instruction
                                )
                                .await?
                            )),
                            None => None,
                        };

                        let deleted = if delete_object {
                            let result = sqlx::query("DELETE FROM object_instances WHERE id = $1")
                                .bind(target_object.id)
                                .execute(&state.db)
                                .await
                                .map_err(|e| {
                                    format!("failed to delete object from function response: {e}")
                                })?;
                            if result.rows_affected() == 0 {
                                return Err("target object no longer exists".to_string());
                            }
                            true
                        } else {
                            false
                        };

                        Ok(ExecutedAction {
                            target_object_id: Some(target_object.id),
                            deleted,
                            preview: preview.clone(),
                            object,
                            link,
                            result: result.or(Some(response)),
                        })
                    }
                    .await;

                let completed_at = Utc::now();
                let duration_ms = timer.elapsed().as_millis() as i64;

                if let Some(package) = package.as_ref() {
                    let run_context = FunctionPackageRunContext {
                        invocation_kind: "action",
                        action_id: Some(action.id),
                        action_name: Some(action.name.as_str()),
                        object_type_id: Some(action.object_type_id),
                        target_object_id: target.as_ref().map(|object| object.id),
                        actor_id: claims.sub,
                    };
                    let status = if outcome.is_ok() {
                        "success"
                    } else {
                        "failure"
                    };
                    let error_message = outcome.as_ref().err().map(String::as_str);

                    if let Err(error) = record_function_package_run(
                        state,
                        package,
                        &run_context,
                        started_at,
                        completed_at,
                        duration_ms,
                        status,
                        error_message,
                    )
                    .await
                    {
                        tracing::warn!(action_id = %action.id, function_package_id = %package.id, %error, "failed to record function-backed action run");
                    }
                }

                outcome
            }
        },
    }
}

fn log_audit_failure(action_id: Uuid, error: &str) {
    tracing::warn!(%action_id, %error, "failed to emit ontology action audit event");
}

fn log_notification_failure(action_id: Uuid, error: &str) {
    tracing::warn!(%action_id, %error, "failed to emit ontology action notification");
}

pub(crate) async fn preview_action_for_simulation(
    state: &AppState,
    claims: &Claims,
    action_id: Uuid,
    target_object_id: Option<Uuid>,
    parameters: Value,
) -> Result<Value, String> {
    let row = load_action_row(state, action_id)
        .await
        .map_err(|error| format!("failed to load action type: {error}"))?
        .ok_or_else(|| "action type was not found".to_string())?;
    let action = ActionType::try_from(row)
        .map_err(|error| format!("failed to decode action type: {error}"))?;

    ensure_action_actor_permission(claims, &action)?;
    let plan = plan_action(
        state,
        claims,
        &action,
        &ValidateActionRequest {
            target_object_id,
            parameters,
        },
    )
    .await
    .map_err(|errors| errors.join("; "))?;

    Ok(plan_preview(&plan))
}

pub async fn create_action_type(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateActionTypeRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return invalid_action("action type name is required");
    }

    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let description = body.description.unwrap_or_default();
    let input_schema = body.input_schema.unwrap_or_default();
    let form_schema = body.form_schema.unwrap_or_default();
    let config = body.config.unwrap_or(Value::Null);
    let authorization_policy = body.authorization_policy.unwrap_or_default();

    if let Err(error) = validate_action_definition(
        &state,
        body.object_type_id,
        &body.operation_kind,
        &input_schema,
        &form_schema,
        &config,
        &authorization_policy,
    )
    .await
    {
        return invalid_action(error);
    }

    let result = sqlx::query_as::<_, ActionTypeRow>(
        r#"INSERT INTO action_types (
		       id, name, display_name, description, object_type_id, operation_kind,
		       input_schema, form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id
		   )
		   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9::jsonb, $10, $11, $12::jsonb, $13)
		   RETURNING id, name, display_name, description, object_type_id, operation_kind,
		             input_schema, form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id,
		             created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(&body.name)
    .bind(display_name)
    .bind(description)
    .bind(body.object_type_id)
    .bind(&body.operation_kind)
    .bind(serde_json::to_value(&input_schema).unwrap_or_else(|_| Value::Array(vec![])))
    .bind(serde_json::to_value(&form_schema).unwrap_or_else(|_| json!({})))
    .bind(config)
    .bind(body.confirmation_required.unwrap_or(false))
    .bind(body.permission_key)
    .bind(serde_json::to_value(&authorization_policy).unwrap_or_else(|_| json!({})))
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(row) => match ActionType::try_from(row) {
            Ok(action_type) => (StatusCode::CREATED, Json(json!(action_type))).into_response(),
            Err(e) => db_error(format!("failed to serialize action type: {e}")),
        },
        Err(e) => db_error(format!("create action type failed: {e}")),
    }
}

pub async fn list_action_types(
    _user: AuthUser,
    State(state): State<AppState>,
    Query(params): Query<ListActionTypesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let search_pattern = format!("%{}%", params.search.unwrap_or_default());

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*) FROM action_types
		   WHERE ($1::uuid IS NULL OR object_type_id = $1)
		     AND (name ILIKE $2 OR display_name ILIKE $2)"#,
    )
    .bind(params.object_type_id)
    .bind(&search_pattern)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let rows = sqlx::query_as::<_, ActionTypeRow>(
        r#"SELECT id, name, display_name, description, object_type_id, operation_kind,
		          input_schema, form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id,
		          created_at, updated_at
		   FROM action_types
		   WHERE ($1::uuid IS NULL OR object_type_id = $1)
		     AND (name ILIKE $2 OR display_name ILIKE $2)
		   ORDER BY created_at DESC
		   LIMIT $3 OFFSET $4"#,
    )
    .bind(params.object_type_id)
    .bind(&search_pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut data = Vec::new();
    for row in rows {
        match ActionType::try_from(row) {
            Ok(action_type) => data.push(action_type),
            Err(e) => return db_error(format!("failed to decode action type row: {e}")),
        }
    }

    Json(ListActionTypesResponse {
        data,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn get_action_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match load_action_row(&state, id).await {
        Ok(Some(row)) => match ActionType::try_from(row) {
            Ok(action_type) => Json(json!(action_type)).into_response(),
            Err(e) => db_error(format!("failed to decode action type: {e}")),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(format!("failed to load action type: {e}")),
    }
}

pub async fn update_action_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateActionTypeRequest>,
) -> impl IntoResponse {
    let Some(existing_row) = (match load_action_row(&state, id).await {
        Ok(row) => row,
        Err(e) => return db_error(format!("failed to load action type: {e}")),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let existing = match ActionType::try_from(existing_row.clone()) {
        Ok(action_type) => action_type,
        Err(e) => return db_error(format!("failed to decode action type: {e}")),
    };

    let operation_kind = body
        .operation_kind
        .unwrap_or(existing.operation_kind.clone());
    let input_schema = body.input_schema.unwrap_or(existing.input_schema.clone());
    let form_schema = body.form_schema.unwrap_or(existing.form_schema.clone());
    let config = body.config.unwrap_or(existing.config.clone());
    let authorization_policy = body
        .authorization_policy
        .unwrap_or(existing.authorization_policy.clone());

    if let Err(error) = validate_action_definition(
        &state,
        existing.object_type_id,
        &operation_kind,
        &input_schema,
        &form_schema,
        &config,
        &authorization_policy,
    )
    .await
    {
        return invalid_action(error);
    }

    let permission_key = body.permission_key.or(existing.permission_key);
    let result = sqlx::query_as::<_, ActionTypeRow>(
        r#"UPDATE action_types SET
		       display_name = COALESCE($2, display_name),
		       description = COALESCE($3, description),
		       operation_kind = $4,
		       input_schema = $5::jsonb,
		       form_schema = $6::jsonb,
		       config = $7::jsonb,
		       confirmation_required = COALESCE($8, confirmation_required),
		       permission_key = $9,
		       authorization_policy = $10::jsonb,
		       updated_at = NOW()
		   WHERE id = $1
		   RETURNING id, name, display_name, description, object_type_id, operation_kind,
		             input_schema, form_schema, config, confirmation_required, permission_key, authorization_policy, owner_id,
		             created_at, updated_at"#,
    )
    .bind(id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(operation_kind)
    .bind(serde_json::to_value(&input_schema).unwrap_or_else(|_| Value::Array(vec![])))
    .bind(serde_json::to_value(&form_schema).unwrap_or_else(|_| json!({})))
    .bind(config)
    .bind(body.confirmation_required)
    .bind(permission_key)
    .bind(serde_json::to_value(&authorization_policy).unwrap_or_else(|_| json!({})))
    .fetch_optional(&state.db)
    .await;

    match result {
        Ok(Some(row)) => match ActionType::try_from(row) {
            Ok(action_type) => Json(json!(action_type)).into_response(),
            Err(e) => db_error(format!("failed to decode action type: {e}")),
        },
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(format!("failed to update action type: {e}")),
    }
}

pub async fn delete_action_type(
    _user: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM action_types WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => db_error(format!("failed to delete action type: {e}")),
    }
}

pub async fn validate_action(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ValidateActionRequest>,
) -> impl IntoResponse {
    let Some(row) = (match load_action_row(&state, id).await {
        Ok(row) => row,
        Err(e) => return db_error(format!("failed to load action type: {e}")),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let action = match ActionType::try_from(row) {
        Ok(action_type) => action_type,
        Err(e) => return db_error(format!("failed to decode action type: {e}")),
    };

    if let Err(error) = ensure_action_actor_permission(&claims, &action) {
        return forbidden(error);
    }

    match plan_action(&state, &claims, &action, &body).await {
        Ok(plan) => Json(ValidateActionResponse {
            valid: true,
            errors: vec![],
            preview: plan_preview(&plan),
        })
        .into_response(),
        Err(errors) => {
            if all_forbidden(&errors) {
                forbidden(errors.join("; "))
            } else {
                Json(ValidateActionResponse {
                    valid: false,
                    errors,
                    preview: Value::Null,
                })
                .into_response()
            }
        }
    }
}

async fn execute_loaded_action(
    state: &AppState,
    claims: &Claims,
    action: ActionType,
    body: ExecuteActionRequest,
) -> Response {
    if let Err(error) = ensure_action_actor_permission(claims, &action) {
        if let Err(audit_error) = emit_action_audit_event(
            state,
            claims,
            &action,
            None,
            body.target_object_id,
            "denied",
            "medium",
            Some(&error),
            body.justification.as_deref(),
            &body.parameters,
            None,
            None,
        )
        .await
        {
            log_audit_failure(action.id, &audit_error);
        }
        return forbidden(error);
    }

    if let Err(error) = ensure_confirmation_justification(&action, body.justification.as_deref()) {
        if let Err(audit_error) = emit_action_audit_event(
            state,
            claims,
            &action,
            None,
            body.target_object_id,
            "failure",
            "medium",
            Some(&error),
            body.justification.as_deref(),
            &body.parameters,
            None,
            None,
        )
        .await
        {
            log_audit_failure(action.id, &audit_error);
        }
        return invalid_action(error);
    }

    let validation_request = ValidateActionRequest {
        target_object_id: body.target_object_id,
        parameters: body.parameters.clone(),
    };
    let plan = match plan_action(state, claims, &action, &validation_request).await {
        Ok(plan) => plan,
        Err(errors) => {
            let status = if all_forbidden(&errors) {
                "denied"
            } else {
                "failure"
            };
            if let Err(audit_error) = emit_action_audit_event(
                state,
                claims,
                &action,
                None,
                body.target_object_id,
                status,
                "medium",
                Some("action validation failed"),
                body.justification.as_deref(),
                &body.parameters,
                None,
                Some(&json!({ "details": errors })),
            )
            .await
            {
                log_audit_failure(action.id, &audit_error);
            }
            let payload = Json(json!({ "error": "action validation failed", "details": errors }));
            return if status == "denied" {
                (StatusCode::FORBIDDEN, payload).into_response()
            } else {
                (StatusCode::BAD_REQUEST, payload).into_response()
            };
        }
    };

    let target_snapshot = target_snapshot_from_plan(&plan);

    match execute_plan(state, claims, &action, body.justification.as_deref(), plan).await {
        Ok(executed) => {
            let audit_result = json!({
                "deleted": executed.deleted,
                "object": executed.object,
                "link": executed.link,
                "result": executed.result,
            });
            if let Err(audit_error) = emit_action_audit_event(
                state,
                claims,
                &action,
                target_snapshot.as_ref(),
                executed.target_object_id,
                "success",
                "low",
                None,
                body.justification.as_deref(),
                &body.parameters,
                Some(&executed.preview),
                Some(&audit_result),
            )
            .await
            {
                log_audit_failure(action.id, &audit_error);
            }

            if let Err(notification_error) = emit_action_notifications(
                state,
                claims,
                &action,
                target_snapshot.as_ref(),
                &body.parameters,
                body.justification.as_deref(),
                &executed,
            )
            .await
            {
                log_notification_failure(action.id, &notification_error);
            }

            Json(ExecuteActionResponse {
                action,
                target_object_id: executed.target_object_id,
                deleted: executed.deleted,
                preview: executed.preview,
                object: executed.object,
                link: executed.link,
                result: executed.result,
            })
            .into_response()
        }
        Err(error) => {
            if let Err(audit_error) = emit_action_audit_event(
                state,
                claims,
                &action,
                target_snapshot.as_ref(),
                body.target_object_id,
                "failure",
                "high",
                Some(&error),
                body.justification.as_deref(),
                &body.parameters,
                None,
                None,
            )
            .await
            {
                log_audit_failure(action.id, &audit_error);
            }
            db_error(error)
        }
    }
}

pub async fn execute_action(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ExecuteActionRequest>,
) -> impl IntoResponse {
    let Some(row) = (match load_action_row(&state, id).await {
        Ok(row) => row,
        Err(e) => return db_error(format!("failed to load action type: {e}")),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let action = match ActionType::try_from(row) {
        Ok(action_type) => action_type,
        Err(e) => return db_error(format!("failed to decode action type: {e}")),
    };

    execute_loaded_action(&state, &claims, action, body).await
}

pub async fn execute_inline_edit(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((type_id, obj_id, property_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<ExecuteInlineEditRequest>,
) -> impl IntoResponse {
    let property = match load_property_row(&state, type_id, property_id).await {
        Ok(Some(property)) => property,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return db_error(format!("failed to load property: {error}")),
    };

    let Some(inline_edit_config) = property.inline_edit_config.clone() else {
        return invalid_action("inline edit is not configured for this property");
    };

    let target = match load_and_authorize_target(&state, &claims, obj_id, type_id).await {
        Ok(target) => target,
        Err(errors) => {
            return if all_forbidden(&errors) {
                forbidden(errors.join("; "))
            } else {
                invalid_action(errors.join("; "))
            };
        }
    };

    let Some(row) = (match load_action_row(&state, inline_edit_config.action_type_id).await {
        Ok(row) => row,
        Err(error) => return db_error(format!("failed to load inline edit action: {error}")),
    }) else {
        return invalid_action("configured inline edit action type was not found");
    };

    let action = match ActionType::try_from(row) {
        Ok(action_type) => action_type,
        Err(error) => return db_error(format!("failed to decode inline edit action: {error}")),
    };

    if action.object_type_id != type_id {
        return invalid_action(
            "configured inline edit action type no longer belongs to this object type",
        );
    }

    let parameters = match build_inline_edit_parameters(
        &action,
        &property,
        &target,
        &inline_edit_config,
        body.value,
    ) {
        Ok(parameters) => parameters,
        Err(error) => return invalid_action(error),
    };

    execute_loaded_action(
        &state,
        &claims,
        action,
        ExecuteActionRequest {
            target_object_id: Some(obj_id),
            parameters,
            justification: body.justification,
        },
    )
    .await
}

pub async fn execute_action_batch(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ExecuteBatchActionRequest>,
) -> impl IntoResponse {
    if body.target_object_ids.is_empty() {
        return invalid_action("target_object_ids must not be empty");
    }

    let Some(row) = (match load_action_row(&state, id).await {
        Ok(row) => row,
        Err(e) => return db_error(format!("failed to load action type: {e}")),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let action = match ActionType::try_from(row) {
        Ok(action_type) => action_type,
        Err(e) => return db_error(format!("failed to decode action type: {e}")),
    };

    if let Err(error) = ensure_action_actor_permission(&claims, &action) {
        if let Err(audit_error) = emit_action_audit_event(
            &state,
            &claims,
            &action,
            None,
            None,
            "denied",
            "medium",
            Some(&error),
            body.justification.as_deref(),
            &body.parameters,
            None,
            Some(&json!({ "target_count": body.target_object_ids.len() })),
        )
        .await
        {
            log_audit_failure(action.id, &audit_error);
        }
        return forbidden(error);
    }

    if let Err(error) = ensure_confirmation_justification(&action, body.justification.as_deref()) {
        if let Err(audit_error) = emit_action_audit_event(
            &state,
            &claims,
            &action,
            None,
            None,
            "failure",
            "medium",
            Some(&error),
            body.justification.as_deref(),
            &body.parameters,
            None,
            Some(&json!({ "target_count": body.target_object_ids.len() })),
        )
        .await
        {
            log_audit_failure(action.id, &audit_error);
        }
        return invalid_action(error);
    }

    let total = body.target_object_ids.len();
    let mut succeeded = 0usize;
    let mut results = Vec::with_capacity(total);

    for target_object_id in body.target_object_ids {
        let validation_request = ValidateActionRequest {
            target_object_id: Some(target_object_id),
            parameters: body.parameters.clone(),
        };

        match plan_action(&state, &claims, &action, &validation_request).await {
            Ok(plan) => {
                let target_snapshot = target_snapshot_from_plan(&plan);

                match execute_plan(
                    &state,
                    &claims,
                    &action,
                    body.justification.as_deref(),
                    plan,
                )
                .await
                {
                    Ok(executed) => {
                        succeeded += 1;
                        let audit_result = json!({
                            "deleted": executed.deleted,
                            "object": executed.object,
                            "link": executed.link,
                            "result": executed.result,
                            "batch": true,
                        });
                        if let Err(audit_error) = emit_action_audit_event(
                            &state,
                            &claims,
                            &action,
                            target_snapshot.as_ref(),
                            executed.target_object_id,
                            "success",
                            "low",
                            None,
                            body.justification.as_deref(),
                            &body.parameters,
                            Some(&executed.preview),
                            Some(&audit_result),
                        )
                        .await
                        {
                            log_audit_failure(action.id, &audit_error);
                        }

                        if let Err(notification_error) = emit_action_notifications(
                            &state,
                            &claims,
                            &action,
                            target_snapshot.as_ref(),
                            &body.parameters,
                            body.justification.as_deref(),
                            &executed,
                        )
                        .await
                        {
                            log_notification_failure(action.id, &notification_error);
                        }

                        results.push(json!({
                            "target_object_id": target_object_id,
                            "status": "succeeded",
                            "deleted": executed.deleted,
                            "preview": executed.preview,
                            "object": executed.object,
                            "link": executed.link,
                            "result": executed.result,
                        }));
                    }
                    Err(error) => {
                        if let Err(audit_error) = emit_action_audit_event(
                            &state,
                            &claims,
                            &action,
                            target_snapshot.as_ref(),
                            Some(target_object_id),
                            "failure",
                            "high",
                            Some(&error),
                            body.justification.as_deref(),
                            &body.parameters,
                            None,
                            Some(&json!({ "batch": true })),
                        )
                        .await
                        {
                            log_audit_failure(action.id, &audit_error);
                        }

                        results.push(json!({
                            "target_object_id": target_object_id,
                            "status": "failed",
                            "error": error,
                        }));
                    }
                }
            }
            Err(errors) => {
                let denied = all_forbidden(&errors);
                if let Err(audit_error) = emit_action_audit_event(
                    &state,
                    &claims,
                    &action,
                    None,
                    Some(target_object_id),
                    if denied { "denied" } else { "failure" },
                    "medium",
                    Some("action validation failed"),
                    body.justification.as_deref(),
                    &body.parameters,
                    None,
                    Some(&json!({ "details": errors, "batch": true })),
                )
                .await
                {
                    log_audit_failure(action.id, &audit_error);
                }

                results.push(json!({
                    "target_object_id": target_object_id,
                    "status": if denied { "denied" } else { "failed" },
                    "errors": errors,
                }));
            }
        }
    }

    Json(ExecuteBatchActionResponse {
        action,
        total,
        succeeded,
        failed: total.saturating_sub(succeeded),
        results,
    })
    .into_response()
}

pub async fn create_action_what_if_branch(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<CreateActionWhatIfBranchRequest>,
) -> impl IntoResponse {
    let Some(row) = (match load_action_row(&state, id).await {
        Ok(row) => row,
        Err(error) => return db_error(format!("failed to load action type: {error}")),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let action = match ActionType::try_from(row) {
        Ok(action_type) => action_type,
        Err(error) => return db_error(format!("failed to decode action type: {error}")),
    };

    if let Err(error) = ensure_action_actor_permission(&claims, &action) {
        return forbidden(error);
    }

    let validation_request = ValidateActionRequest {
        target_object_id: body.target_object_id,
        parameters: body.parameters.clone(),
    };
    let plan = match plan_action(&state, &claims, &action, &validation_request).await {
        Ok(plan) => plan,
        Err(errors) => {
            let payload = Json(json!({ "error": "action validation failed", "details": errors }));
            return if all_forbidden(&errors) {
                (StatusCode::FORBIDDEN, payload).into_response()
            } else {
                (StatusCode::BAD_REQUEST, payload).into_response()
            };
        }
    };

    let preview = plan_preview(&plan);
    let target_snapshot = target_snapshot_from_plan(&plan);
    let before_object = target_snapshot.clone().map(|target| json!(target));
    let after_object = match target_snapshot.as_ref() {
        Some(target) => match simulate_target_after_preview(&state, target, &preview).await {
            Ok(after) => after,
            Err(error) => return invalid_action(error),
        },
        None => None,
    };
    let deleted = after_object.is_none() && target_snapshot.is_some();
    let branch_name = body.name.unwrap_or_else(|| {
        format!(
            "{} what-if {}",
            action.display_name,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
        )
    });

    match sqlx::query_as::<_, ActionWhatIfBranch>(
        r#"INSERT INTO action_what_if_branches (
               id, action_id, target_object_id, name, description, parameters, preview,
               before_object, after_object, deleted, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6::jsonb, $7::jsonb, $8::jsonb, $9::jsonb, $10, $11)
           RETURNING id, action_id, target_object_id, name, description, parameters, preview,
                     before_object, after_object, deleted, owner_id, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(action.id)
    .bind(body.target_object_id)
    .bind(branch_name)
    .bind(body.description.unwrap_or_default())
    .bind(body.parameters)
    .bind(preview)
    .bind(before_object)
    .bind(after_object)
    .bind(deleted)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(branch) => (StatusCode::CREATED, Json(json!(branch))).into_response(),
        Err(error) => db_error(format!("failed to create action what-if branch: {error}")),
    }
}

pub async fn list_action_what_if_branches(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(params): Query<ListActionWhatIfBranchesQuery>,
) -> impl IntoResponse {
    let page = params.page.unwrap_or(1).max(1);
    let per_page = params.per_page.unwrap_or(20).clamp(1, 100);
    let offset = (page - 1) * per_page;
    let show_all = claims.has_role("admin");

    let total = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM action_what_if_branches
           WHERE action_id = $1
             AND ($2::uuid IS NULL OR target_object_id = $2)
             AND ($3 OR owner_id = $4)"#,
    )
    .bind(id)
    .bind(params.target_object_id)
    .bind(show_all)
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let data = sqlx::query_as::<_, ActionWhatIfBranch>(
        r#"SELECT id, action_id, target_object_id, name, description, parameters, preview,
                  before_object, after_object, deleted, owner_id, created_at, updated_at
           FROM action_what_if_branches
           WHERE action_id = $1
             AND ($2::uuid IS NULL OR target_object_id = $2)
             AND ($3 OR owner_id = $4)
           ORDER BY created_at DESC
           LIMIT $5 OFFSET $6"#,
    )
    .bind(id)
    .bind(params.target_object_id)
    .bind(show_all)
    .bind(claims.sub)
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Json(ListActionWhatIfBranchesResponse {
        data,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn delete_action_what_if_branch(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path((id, branch_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    let result = sqlx::query(
        r#"DELETE FROM action_what_if_branches
           WHERE id = $1
             AND action_id = $2
             AND ($3 OR owner_id = $4)"#,
    )
    .bind(branch_id)
    .bind(id)
    .bind(claims.has_role("admin"))
    .bind(claims.sub)
    .execute(&state.db)
    .await;

    match result {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => db_error(format!("failed to delete what-if branch: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn sample_input_schema() -> Vec<ActionInputField> {
        vec![
            ActionInputField {
                name: "mode".to_string(),
                display_name: Some("Mode".to_string()),
                description: None,
                property_type: "string".to_string(),
                required: true,
                default_value: None,
            },
            ActionInputField {
                name: "reason".to_string(),
                display_name: Some("Reason".to_string()),
                description: None,
                property_type: "string".to_string(),
                required: false,
                default_value: None,
            },
            ActionInputField {
                name: "owner".to_string(),
                display_name: Some("Owner".to_string()),
                description: None,
                property_type: "string".to_string(),
                required: false,
                default_value: None,
            },
        ]
    }

    fn sample_object() -> ObjectInstance {
        ObjectInstance {
            id: Uuid::now_v7(),
            object_type_id: Uuid::now_v7(),
            properties: json!({
                "assignee": Uuid::nil().to_string(),
                "watchers": [Uuid::max().to_string()],
                "title": "Example ticket",
            }),
            created_by: Uuid::from_u128(7),
            organization_id: None,
            marking: "public".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn split_action_config_supports_legacy_and_envelope() {
        let legacy = json!({ "url": "http://localhost:1234/hook" });
        let (operation, notifications) = split_action_config(&legacy).expect("legacy config");
        assert_eq!(operation, legacy);
        assert!(notifications.is_empty());

        let envelope = json!({
            "operation": { "url": "http://localhost:1234/hook" },
            "notification_side_effects": [
                {
                    "title": "Done",
                    "body": "Action completed",
                    "send_to_actor": true
                }
            ]
        });
        let (operation, notifications) = split_action_config(&envelope).expect("envelope config");
        assert_eq!(operation, json!({ "url": "http://localhost:1234/hook" }));
        assert_eq!(notifications.len(), 1);
        assert!(notifications[0].send_to_actor);
    }

    #[test]
    fn render_template_resolves_nested_context() {
        let context = json!({
            "action": { "display_name": "Escalate ticket" },
            "parameters": { "priority": "P1" },
            "target": { "properties": { "title": "Example ticket" } }
        });

        let rendered = render_template(
            "Action {{action.display_name}} set {{target.properties.title}} to {{parameters.priority}}",
            &context,
        );

        assert_eq!(rendered, "Action Escalate ticket set Example ticket to P1");
    }

    #[test]
    fn resolve_notification_recipients_combines_sources() {
        let target = sample_object();
        let config: ActionNotificationSideEffectConfig = serde_json::from_value(json!({
            "title": "Done",
            "body": "Body",
            "user_ids": [Uuid::from_u128(10).to_string()],
            "user_id_input_name": "reviewers",
            "target_user_property_name": "watchers",
            "send_to_actor": true,
            "send_to_target_creator": true
        }))
        .expect("notification config");

        let parameters = HashMap::from([(
            "reviewers".to_string(),
            json!([
                Uuid::from_u128(11).to_string(),
                Uuid::from_u128(12).to_string()
            ]),
        )]);

        let (recipients, broadcast) = resolve_notification_recipients(
            &config,
            &parameters,
            Some(&target),
            Uuid::from_u128(13),
        )
        .expect("recipients");

        assert!(!broadcast);
        assert_eq!(
            recipients,
            vec![
                Uuid::from_u128(7),
                Uuid::from_u128(10),
                Uuid::from_u128(11),
                Uuid::from_u128(12),
                Uuid::from_u128(13),
                Uuid::max(),
            ]
        );
    }

    #[test]
    fn materialize_parameters_applies_form_overrides_and_defaults() {
        let mut target = sample_object();
        target.marking = "sensitive".to_string();

        let form_schema: ActionFormSchema = serde_json::from_value(json!({
            "parameter_overrides": [
                {
                    "parameter_name": "reason",
                    "conditions": [
                        { "left": "parameters.mode", "operator": "is", "right": "manual" }
                    ],
                    "required": true
                },
                {
                    "parameter_name": "owner",
                    "conditions": [
                        { "left": "target.marking", "operator": "is", "right": "sensitive" }
                    ],
                    "default_value": "ops"
                }
            ]
        }))
        .expect("form schema");

        let missing_reason = materialize_parameters(
            &sample_input_schema(),
            &json!({ "mode": "manual" }),
            &form_schema,
            Some(&target),
        )
        .expect_err("reason should become required");
        assert_eq!(missing_reason, vec!["reason is required"]);

        let parameters = materialize_parameters(
            &sample_input_schema(),
            &json!({ "mode": "manual", "reason": "escalation" }),
            &form_schema,
            Some(&target),
        )
        .expect("parameters");

        assert_eq!(parameters.get("owner"), Some(&json!("ops")));
        assert_eq!(parameters.get("reason"), Some(&json!("escalation")));
    }

    #[test]
    fn validate_action_form_schema_rejects_unknown_parameter_paths() {
        let input_schema = sample_input_schema();
        let property_types = HashMap::from([("status", "string")]);
        let form_schema: ActionFormSchema = serde_json::from_value(json!({
            "sections": [
                {
                    "id": "main",
                    "columns": 2,
                    "parameter_names": ["mode"]
                }
            ],
            "parameter_overrides": [
                {
                    "parameter_name": "reason",
                    "conditions": [
                        { "left": "parameters.missing", "operator": "is", "right": "yes" }
                    ],
                    "required": true
                }
            ]
        }))
        .expect("form schema");

        let error = validate_action_form_schema(&form_schema, &input_schema, &property_types)
            .expect_err("invalid condition path");
        assert!(error.contains("unknown parameter path"));
    }
}
