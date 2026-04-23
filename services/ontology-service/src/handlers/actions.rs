use std::collections::{HashMap, HashSet};

use auth_middleware::layer::AuthUser;
use axum::{
	extract::{Path, Query, State},
	http::StatusCode,
	response::{IntoResponse, Response},
	Json,
};
use reqwest::{Method, Url};
use serde::Deserialize;
use serde_json::{Map, Value, json};
use uuid::Uuid;

use crate::{
	domain::type_system::{validate_property_type, validate_property_value},
	models::{
		action_type::{
			ActionInputField, ActionOperationKind, ActionType, ActionTypeRow, CreateActionTypeRequest,
			ExecuteActionRequest, ExecuteActionResponse, ListActionTypesQuery,
			ListActionTypesResponse, UpdateActionTypeRequest, ValidateActionRequest,
			ValidateActionResponse,
		},
		link_type::LinkType,
		property::Property,
	},
	AppState,
};

use super::{links::LinkInstance, objects::ObjectInstance};

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
		invocation: HttpInvocationConfig,
		payload: Value,
	},
	InvokeWebhook {
		target: Option<ObjectInstance>,
		invocation: HttpInvocationConfig,
		payload: Value,
	},
}

fn default_source_role() -> String {
	"source".to_string()
}

fn default_http_method() -> String {
	"POST".to_string()
}

fn invalid_action(message: impl Into<String>) -> Response {
	(StatusCode::BAD_REQUEST, Json(json!({ "error": message.into() }))).into_response()
}

fn db_error(message: impl Into<String>) -> Response {
	(StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": message.into() }))).into_response()
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

async fn load_object_instance(
	state: &AppState,
	object_id: Uuid,
) -> Result<Option<ObjectInstance>, sqlx::Error> {
	sqlx::query_as::<_, ObjectInstance>("SELECT * FROM object_instances WHERE id = $1")
		.bind(object_id)
		.fetch_optional(&state.db)
		.await
}

async fn load_object_properties(
	state: &AppState,
	object_type_id: Uuid,
) -> Result<Vec<Property>, sqlx::Error> {
	sqlx::query_as::<_, Property>(
		r#"SELECT * FROM properties WHERE object_type_id = $1 ORDER BY created_at ASC"#,
	)
	.bind(object_type_id)
	.fetch_all(&state.db)
	.await
}

async fn ensure_object_type_exists(state: &AppState, object_type_id: Uuid) -> Result<bool, sqlx::Error> {
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

fn resolve_uuid_parameter(parameters: &HashMap<String, Value>, field_name: &str) -> Result<Uuid, String> {
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
			"name": action.name,
			"display_name": action.display_name,
			"object_type_id": action.object_type_id,
			"operation_kind": action.operation_kind,
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
		let detail = if text.trim().is_empty() { status.to_string() } else { text.clone() };
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
	let properties = load_object_properties(state, target.object_type_id)
		.await
		.map_err(|e| format!("failed to load property definitions: {e}"))?;
	let property_types = properties
		.into_iter()
		.map(|property| (property.name, property.property_type))
		.collect::<HashMap<_, _>>();

	let mut next_properties = target.properties.as_object().cloned().unwrap_or_default();
	for (property_name, value) in patch {
		if let Some(property_type) = property_types.get(property_name.as_str()) {
			validate_property_value(property_type, value)
				.map_err(|e| format!("{}: {}", property_name, e))?;
		}
		next_properties.insert(property_name.clone(), value.clone());
	}

	sqlx::query_as::<_, ObjectInstance>(
		r#"UPDATE object_instances
		   SET properties = $2::jsonb,
		       updated_at = NOW()
		   WHERE id = $1
		   RETURNING *"#,
	)
	.bind(target.id)
	.bind(Value::Object(next_properties))
	.fetch_one(&state.db)
	.await
	.map_err(|e| format!("failed to apply object patch: {e}"))
}

async fn create_link_from_instruction(
	state: &AppState,
	actor_id: Uuid,
	target: &ObjectInstance,
	instruction: &FunctionLinkInstruction,
) -> Result<LinkInstance, String> {
	let counterpart = load_object_instance(state, instruction.target_object_id)
		.await
		.map_err(|e| format!("failed to load linked object: {e}"))?
		.ok_or_else(|| "linked object was not found".to_string())?;
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

	let (source_object_id, target_object_id, expected_counterpart_type) = if instruction.source_role == "source" {
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
) -> Result<(Option<Value>, Option<Value>, Option<FunctionLinkInstruction>, bool), String> {
	let Some(object) = response.as_object() else {
		return Ok((Some(response.clone()), None, None, false));
	};

	let output = object.get("output").cloned();
	let object_patch = object.get("object_patch").cloned();
	let link = object
		.get("link")
		.cloned()
		.map(serde_json::from_value::<FunctionLinkInstruction>)
		.transpose()
		.map_err(|e| format!("invalid function link instruction: {e}"))?;
	let delete_object = object.get("delete_object").and_then(Value::as_bool).unwrap_or(false);

	if delete_object && (object_patch.is_some() || link.is_some()) {
		return Err("function response cannot request delete_object together with object_patch or link".to_string());
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
	let input_names = input_schema.iter().map(|field| field.name.as_str()).collect::<HashSet<_>>();

	match operation_kind {
		ActionOperationKind::UpdateObject => {
			let cfg: UpdateObjectActionConfig = serde_json::from_value(config.clone())
				.map_err(|e| format!("invalid update_object action config: {e}"))?;
			if cfg.property_mappings.is_empty() && cfg.static_patch.as_ref().is_none() {
				return Err("update_object action requires property_mappings or static_patch".to_string());
			}
			for mapping in cfg.property_mappings {
				if mapping.property_name.trim().is_empty() {
					return Err("property_name is required for update_object mappings".to_string());
				}
				match (&mapping.input_name, &mapping.value) {
					(Some(input_name), None) => {
						if !input_names.contains(input_name.as_str()) {
							return Err(format!("unknown input field '{input_name}' in action config"));
						}
					}
					(None, Some(_)) => {}
					_ => {
						return Err(
							"each update_object mapping needs either input_name or value".to_string(),
						)
					}
				}
			}
			if let Some(static_patch) = config.get("static_patch") {
				if !static_patch.is_object() {
					return Err("static_patch must be a JSON object".to_string());
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
				return Err("action object_type_id does not match configured link endpoint".to_string());
			}
		}
		ActionOperationKind::DeleteObject => {
			if !config.is_null() && !config.as_object().map(|value| value.is_empty()).unwrap_or(false) {
				return Err("delete_object actions do not accept config".to_string());
			}
		}
		ActionOperationKind::InvokeFunction | ActionOperationKind::InvokeWebhook => {
			validate_http_invocation_config(config)?;
		}
	}

	Ok(operation_kind)
}

async fn plan_action(
	state: &AppState,
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
			let target_object_id = request
				.target_object_id
				.ok_or_else(|| vec!["target_object_id is required for update_object actions".to_string()])?;
			let target = load_object_instance(state, target_object_id)
				.await
				.map_err(|e| vec![format!("failed to load target object: {e}")])?
				.ok_or_else(|| vec!["target object was not found".to_string()])?;
			ensure_object_type_match(&target, action.object_type_id).map_err(|e| vec![e])?;

			let cfg: UpdateObjectActionConfig = serde_json::from_value(action.config.clone())
				.map_err(|e| vec![format!("invalid action config: {e}")])?;
			let properties = load_object_properties(state, action.object_type_id)
				.await
				.map_err(|e| vec![format!("failed to load property definitions: {e}")])?;
			let property_types = properties
				.into_iter()
				.map(|property| (property.name, property.property_type))
				.collect::<HashMap<_, _>>();

			let mut patch = Map::new();
			for mapping in cfg.property_mappings {
				let value = if let Some(input_name) = mapping.input_name {
					parameters
						.get(&input_name)
						.cloned()
						.ok_or_else(|| vec![format!("missing input '{input_name}' for property mapping")])?
				} else {
					mapping.value.unwrap_or(Value::Null)
				};

				if let Some(property_type) = property_types.get(mapping.property_name.as_str()) {
					validate_property_value(property_type, &value)
						.map_err(|e| vec![format!("{}: {}", mapping.property_name, e)])?;
				}

				patch.insert(mapping.property_name, value);
			}

			if let Some(static_patch) = cfg.static_patch {
				if let Some(values) = static_patch.as_object() {
					for (property_name, value) in values {
						if let Some(property_type) = property_types.get(property_name.as_str()) {
							validate_property_value(property_type, value)
								.map_err(|e| vec![format!("{}: {}", property_name, e)])?;
						}
						patch.insert(property_name.to_string(), value.clone());
					}
				}
			}

			Ok(ActionPlan::UpdateObject { target, patch })
		}
		ActionOperationKind::CreateLink => {
			let target_object_id = request
				.target_object_id
				.ok_or_else(|| vec!["target_object_id is required for create_link actions".to_string()])?;
			let target = load_object_instance(state, target_object_id)
				.await
				.map_err(|e| vec![format!("failed to load target object: {e}")])?
				.ok_or_else(|| vec!["target object was not found".to_string()])?;
			ensure_object_type_match(&target, action.object_type_id).map_err(|e| vec![e])?;

			let cfg: CreateLinkActionConfig = serde_json::from_value(action.config.clone())
				.map_err(|e| vec![format!("invalid action config: {e}")])?;
			let counterpart_id = resolve_uuid_parameter(&parameters, &cfg.target_input_name)
				.map_err(|e| vec![e])?;
			let counterpart = load_object_instance(state, counterpart_id)
				.await
				.map_err(|e| vec![format!("failed to load linked object: {e}")])?
				.ok_or_else(|| vec!["linked object was not found".to_string()])?;
			let link_type = sqlx::query_as::<_, LinkType>("SELECT * FROM link_types WHERE id = $1")
				.bind(cfg.link_type_id)
				.fetch_optional(&state.db)
				.await
				.map_err(|e| vec![format!("failed to load link type: {e}")])?
				.ok_or_else(|| vec!["configured link type was not found".to_string()])?;

			let (source_object_id, target_link_object_id, expected_counterpart_type) = if cfg.source_role == "source" {
				(target.id, counterpart.id, link_type.target_type_id)
			} else {
				(counterpart.id, target.id, link_type.source_type_id)
			};

			if counterpart.object_type_id != expected_counterpart_type {
				return Err(vec!["linked object does not match configured link type".to_string()]);
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
			let target_object_id = request
				.target_object_id
				.ok_or_else(|| vec!["target_object_id is required for delete_object actions".to_string()])?;
			let target = load_object_instance(state, target_object_id)
				.await
				.map_err(|e| vec![format!("failed to load target object: {e}")])?
				.ok_or_else(|| vec!["target object was not found".to_string()])?;
			ensure_object_type_match(&target, action.object_type_id).map_err(|e| vec![e])?;

			Ok(ActionPlan::DeleteObject { target })
		}
		ActionOperationKind::InvokeFunction | ActionOperationKind::InvokeWebhook => {
			let invocation = validate_http_invocation_config(&action.config).map_err(|e| vec![e])?;
			let target = if let Some(target_object_id) = request.target_object_id {
				let target = load_object_instance(state, target_object_id)
					.await
					.map_err(|e| vec![format!("failed to load target object: {e}")])?
					.ok_or_else(|| vec!["target object was not found".to_string()])?;
				ensure_object_type_match(&target, action.object_type_id).map_err(|e| vec![e])?;
				Some(target)
			} else {
				None
			};
			let payload = build_http_payload(action, target.as_ref(), &parameters);

			if operation_kind == ActionOperationKind::InvokeFunction {
				Ok(ActionPlan::InvokeFunction {
					target,
					invocation,
					payload,
				})
			} else {
				Ok(ActionPlan::InvokeWebhook {
					target,
					invocation,
					payload,
				})
			}
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
		} => json!({
			"kind": "invoke_function",
			"target_object_id": target.as_ref().map(|object| object.id),
			"request": {
				"url": invocation.url,
				"method": invocation.method,
				"headers": invocation.headers,
				"payload": payload,
			},
		}),
		ActionPlan::InvokeWebhook {
			target,
			invocation,
			payload,
		} => json!({
			"kind": "invoke_webhook",
			"target_object_id": target.as_ref().map(|object| object.id),
			"request": {
				"url": invocation.url,
				"method": invocation.method,
				"headers": invocation.headers,
				"payload": payload,
			},
		}),
	}
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

	Json(ListActionTypesResponse { data, total, page, per_page }).into_response()
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

	let operation_kind = body.operation_kind.unwrap_or(existing.operation_kind.clone());
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
	_user: AuthUser,
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

	match plan_action(&state, &action, &body).await {
		Ok(plan) => Json(ValidateActionResponse {
			valid: true,
			errors: vec![],
			preview: plan_preview(&plan),
		})
		.into_response(),
		Err(errors) => Json(ValidateActionResponse {
			valid: false,
			errors,
			preview: Value::Null,
		})
		.into_response(),
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
	let validation_request = ValidateActionRequest {
		target_object_id: body.target_object_id,
		parameters: body.parameters,
	};
	let plan = match plan_action(&state, &action, &validation_request).await {
		Ok(plan) => plan,
		Err(errors) => {
			return (
				StatusCode::BAD_REQUEST,
				Json(json!({ "error": "action validation failed", "details": errors })),
			)
			.into_response()
		}
	};
	let preview = plan_preview(&plan);

	match plan {
		ActionPlan::UpdateObject { target, patch } => {
			let mut next_properties = target.properties.as_object().cloned().unwrap_or_default();
			for (key, value) in &patch {
				next_properties.insert(key.clone(), value.clone());
			}

			let updated = sqlx::query_as::<_, ObjectInstance>(
				r#"UPDATE object_instances
				   SET properties = $2::jsonb,
				       updated_at = NOW()
				   WHERE id = $1
				   RETURNING *"#,
			)
			.bind(target.id)
			.bind(Value::Object(next_properties))
			.fetch_one(&state.db)
			.await;

			match updated {
				Ok(object) => Json(ExecuteActionResponse {
					action,
					target_object_id: Some(target.id),
					deleted: false,
					preview: preview.clone(),
					object: Some(json!(object)),
					link: None,
					result: None,
				})
				.into_response(),
				Err(e) => db_error(format!("failed to execute update_object action: {e}")),
			}
		}
		ActionPlan::CreateLink {
			target,
			link_type,
			properties,
			source_object_id,
			target_object_id,
			..
		} => {
			let created = sqlx::query_as::<_, LinkInstance>(
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
			.await;

			match created {
				Ok(link) => Json(ExecuteActionResponse {
					action,
					target_object_id: Some(target.id),
					deleted: false,
					preview: preview.clone(),
					object: None,
					link: Some(json!(link)),
					result: None,
				})
				.into_response(),
				Err(e) => db_error(format!("failed to execute create_link action: {e}")),
			}
		}
		ActionPlan::DeleteObject { target } => match sqlx::query("DELETE FROM object_instances WHERE id = $1")
			.bind(target.id)
			.execute(&state.db)
			.await
		{
			Ok(result) if result.rows_affected() > 0 => Json(ExecuteActionResponse {
				action,
				target_object_id: Some(target.id),
				deleted: true,
				preview: preview.clone(),
				object: None,
				link: None,
				result: None,
			})
			.into_response(),
			Ok(_) => StatusCode::NOT_FOUND.into_response(),
			Err(e) => db_error(format!("failed to execute delete_object action: {e}")),
		},
		ActionPlan::InvokeWebhook {
			target,
			invocation,
			payload,
		} => match invoke_http_action(&state, &invocation, &payload).await {
			Ok(result) => Json(ExecuteActionResponse {
				action,
				target_object_id: target.as_ref().map(|object| object.id),
				deleted: false,
				preview: preview.clone(),
				object: None,
				link: None,
				result: Some(result),
			})
			.into_response(),
			Err(e) => db_error(format!("failed to execute webhook action: {e}")),
		},
		ActionPlan::InvokeFunction {
			target,
			invocation,
			payload,
		} => match invoke_http_action(&state, &invocation, &payload).await {
			Ok(response) => {
				let (result, object_patch, link_instruction, delete_object) = match derive_function_effects(&response) {
					Ok(effects) => effects,
					Err(e) => return db_error(format!("invalid function response: {e}")),
				};

				let Some(target_object) = target.as_ref() else {
					if object_patch.is_some() || link_instruction.is_some() || delete_object {
						return invalid_action("function response requested ontology mutations but target_object_id was not provided");
					}

					return Json(ExecuteActionResponse {
						action,
						target_object_id: None,
						deleted: false,
						preview: preview.clone(),
						object: None,
						link: None,
						result: result.or(Some(response)),
					})
					.into_response();
				};

				let object = match object_patch {
					Some(patch) => match apply_object_patch(&state, target_object, &patch).await {
						Ok(updated) => Some(json!(updated)),
						Err(e) => return db_error(e),
					},
					None => None,
				};

				let link = match link_instruction {
					Some(instruction) => match create_link_from_instruction(&state, claims.sub, target_object, &instruction).await {
						Ok(created) => Some(json!(created)),
						Err(e) => return db_error(e),
					},
					None => None,
				};

				let deleted = if delete_object {
					match sqlx::query("DELETE FROM object_instances WHERE id = $1")
						.bind(target_object.id)
						.execute(&state.db)
						.await
					{
						Ok(result) if result.rows_affected() > 0 => true,
						Ok(_) => return StatusCode::NOT_FOUND.into_response(),
						Err(e) => return db_error(format!("failed to delete object from function response: {e}")),
					}
				} else {
					false
				};

				Json(ExecuteActionResponse {
					action,
					target_object_id: Some(target_object.id),
					deleted,
					preview: preview.clone(),
					object,
					link,
					result: result.or(Some(response)),
				})
				.into_response()
			}
			Err(e) => db_error(format!("failed to execute function action: {e}")),
		},
	}
}
