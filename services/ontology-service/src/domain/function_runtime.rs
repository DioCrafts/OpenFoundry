use std::{collections::HashMap, env, time::Duration};

use auth_middleware::{
    claims::Claims,
    jwt::{build_access_claims, encode_token},
};
use chrono::{DateTime, Utc};
use pyo3::{prelude::*, types::PyDict};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{fs, process::Command, time::timeout};
use uuid::Uuid;

use crate::{
    AppState,
    domain::access::ensure_object_access,
    handlers::objects::ObjectInstance,
    models::{
        action_type::ActionType,
        function_package::{
            FunctionCapabilities, FunctionPackage, FunctionPackageRow, FunctionPackageSummary,
        },
    },
};

#[derive(Debug, Clone, Deserialize)]
pub struct InlinePythonFunctionConfig {
    pub runtime: String,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InlineTypeScriptFunctionConfig {
    pub runtime: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub enum InlineFunctionConfig {
    Python(InlinePythonFunctionConfig),
    TypeScript(InlineTypeScriptFunctionConfig),
}

impl InlineFunctionConfig {
    pub fn runtime_name(&self) -> &str {
        match self {
            Self::Python(config) => config.runtime.as_str(),
            Self::TypeScript(config) => config.runtime.as_str(),
        }
    }

    pub fn source_len(&self) -> usize {
        match self {
            Self::Python(config) => config.source.len(),
            Self::TypeScript(config) => config.source.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedInlineFunction {
    pub config: InlineFunctionConfig,
    pub capabilities: FunctionCapabilities,
    pub package: Option<FunctionPackageSummary>,
}

impl ResolvedInlineFunction {
    pub fn runtime_name(&self) -> &str {
        self.config.runtime_name()
    }

    pub fn source_len(&self) -> usize {
        self.config.source_len()
    }
}

#[derive(Debug, Deserialize)]
struct FunctionPackageReferenceConfig {
    function_package_id: Uuid,
}

#[derive(Debug, sqlx::FromRow)]
struct LinkedObjectRow {
    link_id: Uuid,
    link_type_id: Uuid,
    link_name: String,
    neighbor_id: Uuid,
    neighbor_object_type_id: Uuid,
    neighbor_properties: Value,
    neighbor_created_by: Uuid,
    neighbor_created_at: DateTime<Utc>,
    neighbor_updated_at: DateTime<Utc>,
    neighbor_organization_id: Option<Uuid>,
    neighbor_marking: String,
    direction: String,
}

#[derive(Debug, Deserialize)]
struct TypeScriptRuntimeEnvelope {
    result: Option<Value>,
    #[serde(default)]
    stdout: Vec<String>,
    #[serde(default)]
    stderr: Vec<String>,
    error: Option<TypeScriptRuntimeError>,
}

#[derive(Debug, Deserialize)]
struct TypeScriptRuntimeError {
    message: String,
}

const TYPESCRIPT_RUNTIME_RUNNER: &str = r#"import fs from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

function normalizeBaseUrl(value) {
  return value.endsWith('/') ? value : `${value}/`;
}

function toSearchParams(query) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(query ?? {})) {
    if (value === undefined || value === null || value === '') continue;
    params.set(key, String(value));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
}

function parseJson(text) {
  if (!text || !text.trim()) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function errorMessage(method, path, status, payload) {
  if (payload && typeof payload === 'object' && payload.error) {
    return `${method} ${path} failed with ${status}: ${payload.error}`;
  }
  if (typeof payload === 'string' && payload.trim()) {
    return `${method} ${path} failed with ${status}: ${payload}`;
  }
  return `${method} ${path} failed with ${status}`;
}

function renderLogValue(value) {
  if (typeof value === 'string') return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

async function main() {
  const [, , userFilePath, inputFilePath] = process.argv;
  const input = JSON.parse(await fs.readFile(inputFilePath, 'utf8'));
  const stdout = [];
  const stderr = [];
  const originalFetch = globalThis.fetch.bind(globalThis);
  const blockedCapability = (name) => async () => {
    throw new Error(`${name} capability is disabled for this function package`);
  };

  console.log = (...args) => stdout.push(args.map(renderLogValue).join(' '));
  console.error = (...args) => stderr.push(args.map(renderLogValue).join(' '));

  function toUrl(resource, baseUrl) {
    if (typeof resource === 'string' || resource instanceof URL) {
      return new URL(resource, normalizeBaseUrl(baseUrl));
    }
    if (resource && typeof resource.url === 'string') {
      return new URL(resource.url, normalizeBaseUrl(baseUrl));
    }
    throw new Error('Unsupported fetch resource');
  }

  async function guardedFetch(resource, init) {
    const resolvedUrl = toUrl(resource, input.ontologyServiceUrl);
    if (!input.policy?.allowNetwork) {
      const allowedOrigins = new Set([
        new URL(input.ontologyServiceUrl).origin,
        new URL(input.aiServiceUrl).origin,
      ]);
      if (!allowedOrigins.has(resolvedUrl.origin)) {
        throw new Error(`Network access is disabled for ${resolvedUrl.origin}`);
      }
    }
    return originalFetch(resource, init);
  }

  globalThis.fetch = guardedFetch;

  async function request(baseUrl, method, path, body, query) {
    const url = new URL(path.replace(/^\//, ''), normalizeBaseUrl(baseUrl));
    const suffix = toSearchParams(query);
    if (suffix) {
      url.search = suffix.slice(1);
    }

    const headers = {
      authorization: input.serviceToken,
    };
    if (body !== undefined) {
      headers['content-type'] = 'application/json';
    }

    const response = await guardedFetch(url, {
      method,
      headers,
      body: body !== undefined ? JSON.stringify(body) : undefined,
    });
    const text = await response.text();
    const payload = parseJson(text);
    if (!response.ok) {
      throw new Error(errorMessage(method, path, response.status, payload));
    }
    return payload;
  }

  const allowOntologyRead = input.policy?.allowOntologyRead !== false;
  const allowOntologyWrite = input.policy?.allowOntologyWrite !== false;
  const allowAi = input.policy?.allowAi !== false;

  const sdk = {
    ontology: {
      getObject: allowOntologyRead
        ? ({ typeId, objectId }) =>
            request(input.ontologyServiceUrl, 'GET', `/api/v1/ontology/types/${typeId}/objects/${objectId}`)
        : blockedCapability('ontology.read'),
      updateObject: allowOntologyWrite
        ? ({ typeId, objectId, properties, replace = false, marking }) =>
            request(input.ontologyServiceUrl, 'PATCH', `/api/v1/ontology/types/${typeId}/objects/${objectId}`, {
              properties,
              replace,
              marking,
            })
        : blockedCapability('ontology.write'),
      queryObjects: allowOntologyRead
        ? ({ typeId, equals = {}, limit }) =>
            request(input.ontologyServiceUrl, 'POST', `/api/v1/ontology/types/${typeId}/objects/query`, {
              equals,
              limit,
            })
        : blockedCapability('ontology.read'),
      listNeighbors: allowOntologyRead
        ? ({ typeId, objectId }) =>
            request(input.ontologyServiceUrl, 'GET', `/api/v1/ontology/types/${typeId}/objects/${objectId}/neighbors`)
        : blockedCapability('ontology.read'),
      createLink: allowOntologyWrite
        ? ({ linkTypeId, sourceObjectId, targetObjectId, properties }) =>
            request(input.ontologyServiceUrl, 'POST', `/api/v1/ontology/links/${linkTypeId}/instances`, {
              source_object_id: sourceObjectId,
              target_object_id: targetObjectId,
              properties,
            })
        : blockedCapability('ontology.write'),
      search: allowOntologyRead
        ? ({ query, kind, objectTypeId, limit, semantic = true }) =>
            request(input.ontologyServiceUrl, 'POST', '/api/v1/ontology/search', {
              query,
              kind,
              object_type_id: objectTypeId,
              limit,
              semantic,
            })
        : blockedCapability('ontology.read'),
      graph: allowOntologyRead
        ? ({ rootObjectId, rootTypeId, depth, limit } = {}) =>
            request(input.ontologyServiceUrl, 'GET', '/api/v1/ontology/graph', undefined, {
              root_object_id: rootObjectId,
              root_type_id: rootTypeId,
              depth,
              limit,
            })
        : blockedCapability('ontology.read'),
    },
    ai: {
      complete: allowAi
        ? ({
            userMessage,
            systemPrompt,
            preferredProviderId,
            knowledgeBaseId,
            temperature = 0.2,
            maxTokens = 512,
          }) =>
            request(input.aiServiceUrl, 'POST', '/api/v1/ai/chat/completions', {
              user_message: userMessage,
              system_prompt: systemPrompt,
              preferred_provider_id: preferredProviderId,
              knowledge_base_id: knowledgeBaseId,
              fallback_enabled: true,
              temperature,
              max_tokens: maxTokens,
            })
        : blockedCapability('ai.complete'),
    },
  };

  const llm = {
    complete: sdk.ai.complete,
  };

  try {
    const userModule = await import(pathToFileURL(userFilePath).href);
    const preferredEntrypoint = input.functionPackage?.entrypoint;
    const handler =
      (preferredEntrypoint === 'default' ? userModule.default : undefined) ??
      (preferredEntrypoint && preferredEntrypoint !== 'default' ? userModule[preferredEntrypoint] : undefined) ??
      userModule.default ??
      userModule.handler;
    if (typeof handler !== 'function') {
      throw new Error(
        'TypeScript function must export a default async function or a named handler(context)',
      );
    }

    const context = {
      ...input.context,
      sdk,
      llm,
      functionPackage: input.functionPackage ?? null,
      capabilities: input.policy ?? {},
    };

    const result = await handler(context);
    process.stdout.write(JSON.stringify({ result, stdout, stderr }));
  } catch (error) {
    process.stdout.write(
      JSON.stringify({
        result: null,
        stdout,
        stderr,
        error: {
          message: error?.stack ?? String(error),
        },
      }),
    );
    process.exitCode = 1;
  }
}

await main();
"#;

pub fn parse_inline_function_config(config: &Value) -> Result<Option<InlineFunctionConfig>, String> {
    let Some(runtime) = config.get("runtime").and_then(Value::as_str) else {
        return Ok(None);
    };

    match runtime {
        "python" => {
            let parsed: InlinePythonFunctionConfig =
                serde_json::from_value(config.clone()).map_err(|error| error.to_string())?;
            if parsed.source.trim().is_empty() {
                return Err("inline python function requires a non-empty source".to_string());
            }
            Ok(Some(InlineFunctionConfig::Python(parsed)))
        }
        "typescript" | "javascript" => {
            let parsed: InlineTypeScriptFunctionConfig =
                serde_json::from_value(config.clone()).map_err(|error| error.to_string())?;
            if parsed.source.trim().is_empty() {
                return Err(format!(
                    "inline {} function requires a non-empty source",
                    parsed.runtime
                ));
            }
            Ok(Some(InlineFunctionConfig::TypeScript(parsed)))
        }
        _ => Err(format!(
            "unsupported function runtime '{runtime}', supported runtimes: 'python', 'typescript', 'javascript'"
        )),
    }
}

async fn load_function_package(
    state: &AppState,
    function_package_id: Uuid,
) -> Result<Option<FunctionPackage>, String> {
    sqlx::query_as::<_, FunctionPackageRow>(
        r#"SELECT id, name, display_name, description, runtime, source, entrypoint,
                  capabilities, owner_id, created_at, updated_at
           FROM ontology_function_packages
           WHERE id = $1"#,
    )
    .bind(function_package_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load function package: {error}"))?
    .map(FunctionPackage::try_from)
    .transpose()
    .map_err(|error| format!("failed to decode function package: {error}"))
}

pub fn validate_function_capabilities(
    config: &InlineFunctionConfig,
    capabilities: &FunctionCapabilities,
    package: Option<&FunctionPackageSummary>,
) -> Result<(), String> {
    if config.source_len() > capabilities.max_source_bytes {
        let source = package
            .map(|package| format!("function package '{}'", package.name))
            .unwrap_or_else(|| "inline function".to_string());
        return Err(format!(
            "{source} exceeds max_source_bytes ({} > {})",
            config.source_len(),
            capabilities.max_source_bytes
        ));
    }

    if capabilities.timeout_seconds == 0 || capabilities.timeout_seconds > 300 {
        return Err(
            "timeout_seconds must be between 1 and 300 for ontology function execution"
                .to_string(),
        );
    }

    if let Some(package) = package {
        if !matches!(package.entrypoint.as_str(), "default" | "handler") {
            return Err(format!(
                "unsupported function package entrypoint '{}', supported values: default, handler",
                package.entrypoint
            ));
        }
    }

    Ok(())
}

pub async fn resolve_inline_function_config(
    state: &AppState,
    config: &Value,
) -> Result<Option<ResolvedInlineFunction>, String> {
    if let Some(function_package_id) = config.get("function_package_id") {
        let reference: FunctionPackageReferenceConfig = serde_json::from_value(json!({
            "function_package_id": function_package_id,
        }))
        .map_err(|error| format!("invalid function package reference: {error}"))?;
        let package = load_function_package(state, reference.function_package_id)
            .await?
            .ok_or_else(|| "referenced function package was not found".to_string())?;
        let package_summary = FunctionPackageSummary::from(&package);
        let inline_config = parse_inline_function_config(&json!({
            "runtime": package.runtime,
            "source": package.source,
        }))?
        .ok_or_else(|| "function package does not define a supported runtime".to_string())?;
        validate_function_capabilities(
            &inline_config,
            &package.capabilities,
            Some(&package_summary),
        )?;

        return Ok(Some(ResolvedInlineFunction {
            config: inline_config,
            capabilities: package.capabilities,
            package: Some(package_summary),
        }));
    }

    let Some(config) = parse_inline_function_config(config)? else {
        return Ok(None);
    };
    let capabilities = FunctionCapabilities::default();
    validate_function_capabilities(&config, &capabilities, None)?;
    Ok(Some(ResolvedInlineFunction {
        config,
        capabilities,
        package: None,
    }))
}

pub async fn execute_inline_function(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    target: Option<&ObjectInstance>,
    parameters: &HashMap<String, Value>,
    config: &ResolvedInlineFunction,
    justification: Option<&str>,
) -> Result<Value, String> {
    match &config.config {
        InlineFunctionConfig::Python(inner) => {
            execute_inline_python_function(
                state,
                claims,
                action,
                target,
                parameters,
                inner,
                config,
                justification,
            )
            .await
        }
        InlineFunctionConfig::TypeScript(inner) => {
            execute_inline_typescript_function(
                state,
                claims,
                action,
                target,
                parameters,
                inner,
                config,
                justification,
            )
            .await
        }
    }
}

pub async fn execute_inline_python_function(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    target: Option<&ObjectInstance>,
    parameters: &HashMap<String, Value>,
    config: &InlinePythonFunctionConfig,
    resolved: &ResolvedInlineFunction,
    justification: Option<&str>,
) -> Result<Value, String> {
    let object_set = load_accessible_object_set(state, claims, action.object_type_id).await?;
    let linked_objects = match target {
        Some(target) => load_linked_objects(state, claims, target.id).await?,
        None => Vec::new(),
    };
    let target_json = serde_json::to_string(&target.cloned().map(object_to_json))
        .map_err(|error| error.to_string())?;
    let action_json = serde_json::to_string(&json!({
        "id": action.id,
        "name": &action.name,
        "display_name": &action.display_name,
        "object_type_id": action.object_type_id,
        "operation_kind": &action.operation_kind,
        "permission_key": &action.permission_key,
    }))
    .map_err(|error| error.to_string())?;
    let parameters_json = serde_json::to_string(parameters).map_err(|error| error.to_string())?;
    let object_set_json = serde_json::to_string(&object_set).map_err(|error| error.to_string())?;
    let linked_objects_json =
        serde_json::to_string(&linked_objects).map_err(|error| error.to_string())?;
    let justification_json =
        serde_json::to_string(&justification).map_err(|error| error.to_string())?;
    let function_package_json =
        serde_json::to_string(&resolved.package).map_err(|error| error.to_string())?;
    let capabilities_json =
        serde_json::to_string(&resolved.capabilities).map_err(|error| error.to_string())?;

    Python::with_gil(|py| -> Result<Value, String> {
        let locals = PyDict::new_bound(py);
        locals
            .set_item("action_json", action_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("target_object_json", target_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("parameters_json", parameters_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("object_set_json", object_set_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("linked_objects_json", linked_objects_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("justification_json", justification_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("context_now", Utc::now().to_rfc3339())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("function_package_json", function_package_json.clone())
            .map_err(|error| error.to_string())?;
        locals
            .set_item("capabilities_json", capabilities_json.clone())
            .map_err(|error| error.to_string())?;

        py.run_bound(
            "import io, json, sys\naction = json.loads(action_json)\ntarget_object = json.loads(target_object_json)\nparameters = json.loads(parameters_json)\nobject_set = json.loads(object_set_json)\nlinked_objects = json.loads(linked_objects_json)\njustification = json.loads(justification_json)\nfunction_package = json.loads(function_package_json)\ncapabilities = json.loads(capabilities_json)\ncontext = {\n    'action': action,\n    'target_object': target_object,\n    'parameters': parameters,\n    'object_set': object_set,\n    'linked_objects': linked_objects,\n    'justification': justification,\n    'context_now': context_now,\n    'function_package': function_package,\n    'capabilities': capabilities,\n}\nresult = None\nobject_patch = None\nlink = None\ndelete_object = False\n_buf = io.StringIO()\n_real_stdout = sys.stdout\nsys.stdout = _buf",
            None,
            Some(&locals),
        )
        .map_err(|error| error.to_string())?;

        let execution = py.run_bound(&config.source, None, Some(&locals));
        let stdout = py
            .eval_bound("_buf.getvalue()", None, Some(&locals))
            .ok()
            .and_then(|value| value.extract::<String>().ok())
            .unwrap_or_default();
        let response_json = py
            .eval_bound(
                r#"json.dumps({
                    'output': result,
                    'object_patch': object_patch,
                    'link': link,
                    'delete_object': bool(delete_object),
                    'stdout': _buf.getvalue(),
                })"#,
                None,
                Some(&locals),
            )
            .ok()
            .and_then(|value| value.extract::<String>().ok());
        let _ = py.run_bound("sys.stdout = _real_stdout", None, Some(&locals));

        execution.map_err(|error| error.to_string())?;

        let mut response = response_json
            .map(|raw| serde_json::from_str::<Value>(&raw).map_err(|error| error.to_string()))
            .transpose()?
            .unwrap_or_else(|| json!({}));

        if let Some(object) = response.as_object_mut() {
            let has_output = object.get("output").is_some_and(|value| !value.is_null());
            if !has_output && !stdout.trim().is_empty() {
                object.insert("output".to_string(), json!({ "stdout": stdout }));
            }
        }

        Ok(response)
    })
}

async fn execute_inline_typescript_function(
    state: &AppState,
    claims: &Claims,
    action: &ActionType,
    target: Option<&ObjectInstance>,
    parameters: &HashMap<String, Value>,
    config: &InlineTypeScriptFunctionConfig,
    resolved: &ResolvedInlineFunction,
    justification: Option<&str>,
) -> Result<Value, String> {
    let object_set = load_accessible_object_set(state, claims, action.object_type_id).await?;
    let linked_objects = match target {
        Some(target) => load_linked_objects(state, claims, target.id).await?,
        None => Vec::new(),
    };
    let service_token = issue_inline_function_token(state, claims)?;

    let input = json!({
        "context": {
            "action": {
                "id": action.id,
                "name": &action.name,
                "display_name": &action.display_name,
                "object_type_id": action.object_type_id,
                "operation_kind": &action.operation_kind,
                "permission_key": &action.permission_key,
            },
            "targetObject": target.cloned().map(object_to_json),
            "parameters": parameters,
            "objectSet": object_set,
            "linkedObjects": linked_objects,
            "justification": justification,
            "contextNow": Utc::now().to_rfc3339(),
        },
        "policy": resolved.capabilities,
        "functionPackage": resolved.package,
        "serviceToken": service_token,
        "ontologyServiceUrl": state.ontology_service_url,
        "aiServiceUrl": state.ai_service_url,
    });

    let temp_dir = env::temp_dir().join(format!("of-ontology-inline-ts-{}", Uuid::now_v7()));
    fs::create_dir_all(&temp_dir)
        .await
        .map_err(|error| format!("failed to create TypeScript function temp dir: {error}"))?;
    let user_file_path = temp_dir.join("user.ts");
    let runner_file_path = temp_dir.join("runner.mjs");
    let input_file_path = temp_dir.join("input.json");

    fs::write(&user_file_path, &config.source)
        .await
        .map_err(|error| format!("failed to write TypeScript function source: {error}"))?;
    fs::write(&runner_file_path, TYPESCRIPT_RUNTIME_RUNNER)
        .await
        .map_err(|error| format!("failed to write TypeScript runtime harness: {error}"))?;
    fs::write(&input_file_path, serde_json::to_vec(&input).map_err(|error| error.to_string())?)
        .await
        .map_err(|error| format!("failed to write TypeScript runtime input: {error}"))?;

    let output = timeout(
        Duration::from_secs(resolved.capabilities.timeout_seconds),
        Command::new(&state.node_runtime_command)
        .arg("--experimental-strip-types")
        .arg(&runner_file_path)
        .arg(&user_file_path)
        .arg(&input_file_path)
        .output(),
    )
    .await
    .map_err(|_| {
        format!(
            "TypeScript function timed out after {} seconds",
            resolved.capabilities.timeout_seconds
        )
    })?
    .map_err(|error| format!("failed to start TypeScript function runtime: {error}"))?;

    let _ = fs::remove_dir_all(&temp_dir).await;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let envelope: TypeScriptRuntimeEnvelope = serde_json::from_str(&stdout).map_err(|error| {
        format!(
            "failed to decode TypeScript function response: {error}; raw stdout: {stdout}; raw stderr: {stderr}"
        )
    })?;

    if !output.status.success() {
        let error_message = envelope
            .error
            .map(|error| error.message)
            .unwrap_or_else(|| "TypeScript function failed".to_string());
        return Err(format!("{error_message}\n{stderr}"));
    }

    if let Some(error) = envelope.error {
        return Err(error.message);
    }

    Ok(enrich_typescript_result(
        envelope.result,
        &envelope.stdout,
        &envelope.stderr,
    ))
}

fn enrich_typescript_result(result: Option<Value>, stdout: &[String], stderr: &[String]) -> Value {
    let mut value = result.unwrap_or(Value::Null);

    match &mut value {
        Value::Object(object) => {
            if !stdout.is_empty() {
                object.insert("stdout".to_string(), json!(stdout));
            }
            if !stderr.is_empty() {
                object.insert("stderr".to_string(), json!(stderr));
            }
            let has_output = object.get("output").is_some_and(|value| !value.is_null());
            if !has_output && (!stdout.is_empty() || !stderr.is_empty()) {
                object.insert(
                    "output".to_string(),
                    json!({
                        "stdout": stdout,
                        "stderr": stderr,
                    }),
                );
            }
            Value::Object(object.clone())
        }
        Value::Null => json!({
            "output": {
                "stdout": stdout,
                "stderr": stderr,
            },
            "stdout": stdout,
            "stderr": stderr,
        }),
        other => json!({
            "output": other,
            "stdout": stdout,
            "stderr": stderr,
        }),
    }
}

fn issue_inline_function_token(state: &AppState, claims: &Claims) -> Result<String, String> {
    let service_claims = build_access_claims(
        &state.jwt_config,
        claims.sub,
        &claims.email,
        &claims.name,
        claims.roles.clone(),
        claims.permissions.clone(),
        claims.org_id,
        claims.attributes.clone(),
        claims.auth_methods.clone(),
    );
    let token = encode_token(&state.jwt_config, &service_claims)
        .map_err(|error| format!("failed to issue function runtime token: {error}"))?;
    Ok(format!("Bearer {token}"))
}

pub async fn load_accessible_object_set(
    state: &AppState,
    claims: &Claims,
    object_type_id: Uuid,
) -> Result<Vec<Value>, String> {
    let objects = sqlx::query_as::<_, ObjectInstance>(
        r#"SELECT id, object_type_id, properties, created_by, organization_id, marking, created_at, updated_at
           FROM object_instances
           WHERE object_type_id = $1
           ORDER BY created_at ASC"#,
    )
    .bind(object_type_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    Ok(objects
        .into_iter()
        .filter(|object| ensure_object_access(claims, object).is_ok())
        .map(object_to_json)
        .collect())
}

pub async fn load_linked_objects(
    state: &AppState,
    claims: &Claims,
    object_id: Uuid,
) -> Result<Vec<Value>, String> {
    let rows = sqlx::query_as::<_, LinkedObjectRow>(
        r#"SELECT li.id AS link_id,
                  li.link_type_id,
                  lt.name AS link_name,
                  CASE WHEN li.source_object_id = $1 THEN target.id ELSE source.id END AS neighbor_id,
                  CASE WHEN li.source_object_id = $1 THEN target.object_type_id ELSE source.object_type_id END AS neighbor_object_type_id,
                  CASE WHEN li.source_object_id = $1 THEN target.properties ELSE source.properties END AS neighbor_properties,
                  CASE WHEN li.source_object_id = $1 THEN target.created_by ELSE source.created_by END AS neighbor_created_by,
                  CASE WHEN li.source_object_id = $1 THEN target.created_at ELSE source.created_at END AS neighbor_created_at,
                  CASE WHEN li.source_object_id = $1 THEN target.updated_at ELSE source.updated_at END AS neighbor_updated_at,
                  CASE WHEN li.source_object_id = $1 THEN target.organization_id ELSE source.organization_id END AS neighbor_organization_id,
                  CASE WHEN li.source_object_id = $1 THEN target.marking ELSE source.marking END AS neighbor_marking,
                  CASE WHEN li.source_object_id = $1 THEN 'outbound' ELSE 'inbound' END AS direction
           FROM link_instances li
           INNER JOIN link_types lt ON lt.id = li.link_type_id
           INNER JOIN object_instances source ON source.id = li.source_object_id
           INNER JOIN object_instances target ON target.id = li.target_object_id
           WHERE li.source_object_id = $1 OR li.target_object_id = $1
           ORDER BY li.created_at ASC"#,
    )
    .bind(object_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    let mut linked = Vec::new();
    for row in rows {
        let neighbor = ObjectInstance {
            id: row.neighbor_id,
            object_type_id: row.neighbor_object_type_id,
            properties: row.neighbor_properties.clone(),
            created_by: row.neighbor_created_by,
            organization_id: row.neighbor_organization_id,
            marking: row.neighbor_marking.clone(),
            created_at: row.neighbor_created_at,
            updated_at: row.neighbor_updated_at,
        };
        if ensure_object_access(claims, &neighbor).is_err() {
            continue;
        }

        linked.push(json!({
            "direction": row.direction,
            "link_id": row.link_id,
            "link_type_id": row.link_type_id,
            "link_name": row.link_name,
            "object": object_to_json(neighbor),
        }));
    }

    Ok(linked)
}

pub fn object_to_json(object: ObjectInstance) -> Value {
    json!({
        "id": object.id,
        "object_type_id": object.object_type_id,
        "organization_id": object.organization_id,
        "marking": object.marking,
        "properties": object.properties,
        "created_by": object.created_by,
        "created_at": object.created_at,
        "updated_at": object.updated_at,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{InlineFunctionConfig, enrich_typescript_result, parse_inline_function_config};

    #[test]
    fn parses_typescript_runtime_config() {
        let parsed = parse_inline_function_config(&json!({
            "runtime": "typescript",
            "source": "export default async function handler() { return { ok: true }; }",
        }))
        .expect("config should parse")
        .expect("config should be detected");

        assert!(matches!(parsed, InlineFunctionConfig::TypeScript(_)));
        assert_eq!(parsed.runtime_name(), "typescript");
    }

    #[test]
    fn enriches_typescript_result_with_logs() {
        let result = enrich_typescript_result(
            Some(json!({ "object_patch": { "status": "done" } })),
            &["hello".to_string()],
            &[],
        );
        assert_eq!(result["stdout"], json!(["hello"]));
        assert_eq!(result["output"]["stdout"], json!(["hello"]));
    }
}
