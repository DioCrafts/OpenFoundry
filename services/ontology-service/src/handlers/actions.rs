use std::collections::{HashMap, HashSet};

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
use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
    AppState,
    domain::{
        access::ensure_object_access,
        function_runtime::{
            ResolvedInlineFunction, execute_inline_function, resolve_inline_function_config,
        },
        schema::{load_effective_properties, validate_object_properties},
        type_system::{validate_property_type, validate_property_value},
    },
    models::{
        action_type::{
            ActionInputField, ActionOperationKind, ActionType, ActionTypeRow,
            CreateActionTypeRequest, ExecuteActionRequest, ExecuteActionResponse,
            ExecuteBatchActionRequest, ExecuteBatchActionResponse, ListActionTypesQuery,
            ListActionTypesResponse, UpdateActionTypeRequest, ValidateActionRequest,
            ValidateActionResponse,
        },
        link_type::LinkType,
    },
};

use super::{
    links::LinkInstance,
    objects::{ObjectInstance, load_object_instance},
};

#[derive(Debug, Deserialize)]
struct UpdateObjectActionConfig {
    property_mappings: Vec<ActionPropertyMapping>,
    #[serde(default)]
    static_patch: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct ActionPropertyMapping {
    property_name: String,
    input_name: Option<String>,
    value: Option<Value>,
}

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

fn ensure_action_permission(claims: &Claims, action: &ActionType) -> Result<(), String> {
    if let Some(permission_key) = action.permission_key.as_deref() {
        if !claims.has_permission_key(permission_key) {
            return Err(format!(
                "forbidden: missing permission '{}'",
                permission_key
            ));
        }
    }

    Ok(())
}

fn ensure_confirmation_justification(
    action: &ActionType,
    justification: Option<&str>,
) -> Result<(), String> {
    if action.confirmation_required
        && justification.map(str::trim).filter(|value| !value.is_empty()).is_none()
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
		          config, confirmation_required, permission_key, owner_id, created_at, updated_at
		   FROM action_types WHERE id = $1"#,
    )
    .bind(action_id)
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
) -> Result<HashMap<String, Value>, Vec<String>> {
    let provided = parameters.as_object().cloned().unwrap_or_default();

    let mut values = HashMap::new();
    let mut errors = Vec::new();

    for field in input_schema {
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

    let output = object.get("output").filter(|value| !value.is_null()).cloned();
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
    config: &Value,
) -> Result<ActionOperationKind, String> {
    if !ensure_object_type_exists(state, object_type_id)
        .await
        .map_err(|e| format!("failed to validate object type: {e}"))?
    {
        return Err("referenced object type does not exist".to_string());
    }

    ensure_input_schema(input_schema)?;
    let operation_kind = parse_operation_kind(operation_kind_raw)?;
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

    match operation_kind {
        ActionOperationKind::UpdateObject => {
            let cfg: UpdateObjectActionConfig = serde_json::from_value(config.clone())
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
                        validate_property_value(property_type, value).map_err(|error| {
                            format!("{}: {}", mapping.property_name, error)
                        })?;
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
                    let property_type = property_types
                        .get(property_name.as_str())
                        .ok_or_else(|| format!("unknown property '{property_name}' in static_patch"))?;
                    validate_property_value(property_type, value)
                        .map_err(|error| format!("{}: {}", property_name, error))?;
                }
            }
        }
        ActionOperationKind::CreateLink => {
            let cfg: CreateLinkActionConfig = serde_json::from_value(config.clone())
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
            if !config.is_null()
                && !config
                    .as_object()
                    .map(|value| value.is_empty())
                    .unwrap_or(false)
            {
                return Err("delete_object actions do not accept config".to_string());
            }
        }
        ActionOperationKind::InvokeFunction => {
            if resolve_inline_function_config(state, config).await?.is_none() {
                validate_http_invocation_config(config)?;
            }
        }
        ActionOperationKind::InvokeWebhook => {
            validate_http_invocation_config(config)?;
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
    let parameters = materialize_parameters(&action.input_schema, &request.parameters)?;
    let operation_kind = match parse_operation_kind(&action.operation_kind) {
        Ok(kind) => kind,
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

            let cfg: UpdateObjectActionConfig = serde_json::from_value(action.config.clone())
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
                        let property_type = property_types.get(property_name.as_str()).ok_or_else(|| {
                            vec![format!("unknown property '{property_name}' in static_patch")]
                        })?;
                        validate_property_value(property_type, value)
                            .map_err(|e| vec![format!("{}: {}", property_name, e)])?;
                        patch.insert(property_name.to_string(), value.clone());
                    }
                }
            }

            Ok(ActionPlan::UpdateObject { target, patch })
        }
        ActionOperationKind::CreateLink => {
            let target_object_id = request.target_object_id.ok_or_else(|| {
                vec!["target_object_id is required for create_link actions".to_string()]
            })?;
            let target =
                load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                    .await?;

            let cfg: CreateLinkActionConfig = serde_json::from_value(action.config.clone())
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

            Ok(ActionPlan::DeleteObject { target })
        }
        ActionOperationKind::InvokeFunction => {
            let target = match request.target_object_id {
                Some(target_object_id) => Some(
                    load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                        .await?,
                ),
                None => None,
            };
            let payload = build_http_payload(action, target.as_ref(), &parameters);
            let invocation = match resolve_inline_function_config(state, &action.config).await {
                Ok(Some(config)) => FunctionInvocation::Inline(config),
                Ok(None) => FunctionInvocation::Http(
                    validate_http_invocation_config(&action.config).map_err(|e| vec![e])?,
                ),
                Err(error) => return Err(vec![error]),
            };

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
                    load_and_authorize_target(state, claims, target_object_id, action.object_type_id)
                        .await?,
                ),
                None => None,
            };
            let payload = build_http_payload(action, target.as_ref(), &parameters);
            let invocation =
                validate_http_invocation_config(&action.config).map_err(|e| vec![e])?;

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
        } => {
            let response = match &invocation {
                FunctionInvocation::Http(invocation) => {
                    invoke_http_action(state, invocation, &payload).await?
                }
                FunctionInvocation::Inline(config) => {
                    execute_inline_function(
                        state,
                        claims,
                        action,
                        target.as_ref(),
                        &parameters,
                        config,
                        justification,
                    )
                    .await?
                }
            };

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
                Some(patch) => Some(json!(apply_object_patch(state, target_object, &patch).await?)),
                None => None,
            };

            let link = match link_instruction {
                Some(instruction) => Some(json!(
                    create_link_from_instruction(state, claims, claims.sub, target_object, &instruction)
                        .await?
                )),
                None => None,
            };

            let deleted = if delete_object {
                let result = sqlx::query("DELETE FROM object_instances WHERE id = $1")
                    .bind(target_object.id)
                    .execute(&state.db)
                    .await
                    .map_err(|e| format!("failed to delete object from function response: {e}"))?;
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
    }
}

fn log_audit_failure(action_id: Uuid, error: &str) {
    tracing::warn!(%action_id, %error, "failed to emit ontology action audit event");
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
    let action =
        ActionType::try_from(row).map_err(|error| format!("failed to decode action type: {error}"))?;

    ensure_action_permission(claims, &action)?;
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
    let config = body.config.unwrap_or(Value::Null);

    if let Err(error) = validate_action_definition(
        &state,
        body.object_type_id,
        &body.operation_kind,
        &input_schema,
        &config,
    )
    .await
    {
        return invalid_action(error);
    }

    let result = sqlx::query_as::<_, ActionTypeRow>(
        r#"INSERT INTO action_types (
		       id, name, display_name, description, object_type_id, operation_kind,
		       input_schema, config, confirmation_required, permission_key, owner_id
		   )
		   VALUES ($1, $2, $3, $4, $5, $6, $7::jsonb, $8::jsonb, $9, $10, $11)
		   RETURNING id, name, display_name, description, object_type_id, operation_kind,
		             input_schema, config, confirmation_required, permission_key, owner_id,
		             created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(&body.name)
    .bind(display_name)
    .bind(description)
    .bind(body.object_type_id)
    .bind(&body.operation_kind)
    .bind(serde_json::to_value(&input_schema).unwrap_or_else(|_| Value::Array(vec![])))
    .bind(config)
    .bind(body.confirmation_required.unwrap_or(false))
    .bind(body.permission_key)
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
		          input_schema, config, confirmation_required, permission_key, owner_id,
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
    let config = body.config.unwrap_or(existing.config.clone());

    if let Err(error) = validate_action_definition(
        &state,
        existing.object_type_id,
        &operation_kind,
        &input_schema,
        &config,
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
		       config = $6::jsonb,
		       confirmation_required = COALESCE($7, confirmation_required),
		       permission_key = $8,
		       updated_at = NOW()
		   WHERE id = $1
		   RETURNING id, name, display_name, description, object_type_id, operation_kind,
		             input_schema, config, confirmation_required, permission_key, owner_id,
		             created_at, updated_at"#,
    )
    .bind(id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(operation_kind)
    .bind(serde_json::to_value(&input_schema).unwrap_or_else(|_| Value::Array(vec![])))
    .bind(config)
    .bind(body.confirmation_required)
    .bind(permission_key)
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

    if let Err(error) = ensure_action_permission(&claims, &action) {
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

    if let Err(error) = ensure_action_permission(&claims, &action) {
        if let Err(audit_error) = emit_action_audit_event(
            &state,
            &claims,
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
            &state,
            &claims,
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
    let plan = match plan_action(&state, &claims, &action, &validation_request).await {
        Ok(plan) => plan,
        Err(errors) => {
            let status = if all_forbidden(&errors) {
                "denied"
            } else {
                "failure"
            };
            let severity = if all_forbidden(&errors) {
                "medium"
            } else {
                "medium"
            };
            if let Err(audit_error) = emit_action_audit_event(
                &state,
                &claims,
                &action,
                None,
                body.target_object_id,
                status,
                severity,
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

    let target_snapshot = match &plan {
        ActionPlan::UpdateObject { target, .. }
        | ActionPlan::CreateLink { target, .. }
        | ActionPlan::DeleteObject { target }
        | ActionPlan::InvokeFunction {
            target: Some(target), ..
        }
        | ActionPlan::InvokeWebhook {
            target: Some(target), ..
        } => Some(target.clone()),
        _ => None,
    };

    match execute_plan(&state, &claims, &action, body.justification.as_deref(), plan).await {
        Ok(executed) => {
            let audit_result = json!({
                "deleted": executed.deleted,
                "object": executed.object,
                "link": executed.link,
                "result": executed.result,
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
                &state,
                &claims,
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

    if let Err(error) = ensure_action_permission(&claims, &action) {
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
                let target_snapshot = match &plan {
                    ActionPlan::UpdateObject { target, .. }
                    | ActionPlan::CreateLink { target, .. }
                    | ActionPlan::DeleteObject { target }
                    | ActionPlan::InvokeFunction {
                        target: Some(target), ..
                    }
                    | ActionPlan::InvokeWebhook {
                        target: Some(target), ..
                    } => Some(target.clone()),
                    _ => None,
                };

                match execute_plan(
                    &state,
                    &claims,
                    &action,
                    body.justification.as_deref(),
                    plan,
                )
                .await {
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
