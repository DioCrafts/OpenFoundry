use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chrono::Utc;
use serde_json::{Value, json};
use std::time::Instant;
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        access::ensure_object_access,
        function_metrics::{FunctionPackageRunContext, record_function_package_run},
        function_runtime::{
            ResolvedInlineFunction, execute_inline_function, parse_inline_function_config,
            validate_function_capabilities,
        },
    },
    handlers::objects::load_object_instance,
    models::{
        action_type::ActionType,
        function_authoring::{
            FunctionAuthoringSurfaceResponse, FunctionAuthoringTemplate,
            FunctionSdkPackageReference,
        },
        function_metrics::{
            FunctionPackageMetricsResponse, FunctionPackageMetricsRow, FunctionPackageRun,
            ListFunctionPackageRunsQuery, ListFunctionPackageRunsResponse,
        },
        function_package::{
            CreateFunctionPackageRequest, FunctionCapabilities, FunctionPackage,
            FunctionPackageRow, FunctionPackageSummary, ListFunctionPackagesQuery,
            ListFunctionPackagesResponse, SimulateFunctionPackageRequest,
            SimulateFunctionPackageResponse, UpdateFunctionPackageRequest,
            ValidateFunctionPackageRequest, ValidateFunctionPackageResponse,
            default_function_package_version, parse_function_package_version,
        },
    },
};

fn invalid(message: impl Into<String>) -> Response {
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

fn default_entrypoint() -> String {
    "handler".to_string()
}

fn ensure_entrypoint(entrypoint: &str) -> Result<(), String> {
    if matches!(entrypoint, "default" | "handler") {
        Ok(())
    } else {
        Err("entrypoint must be 'default' or 'handler'".to_string())
    }
}

fn validate_package_source(
    runtime: &str,
    source: &str,
    entrypoint: &str,
    capabilities: &FunctionCapabilities,
) -> Result<(), String> {
    ensure_entrypoint(entrypoint)?;
    let config = parse_inline_function_config(&json!({
        "runtime": runtime,
        "source": source,
    }))?
    .ok_or_else(|| "runtime/source must define a supported inline function".to_string())?;
    validate_function_capabilities(&config, capabilities, None)
}

async fn load_package(state: &AppState, id: Uuid) -> Result<Option<FunctionPackage>, String> {
    sqlx::query_as::<_, FunctionPackageRow>(
        r#"SELECT id, name, version, display_name, description, runtime, source, entrypoint,
                  capabilities, owner_id, created_at, updated_at
           FROM ontology_function_packages
           WHERE id = $1"#,
    )
    .bind(id)
    .fetch_optional(&state.db)
    .await
    .map_err(|error| format!("failed to load function package: {error}"))?
    .map(FunctionPackage::try_from)
    .transpose()
    .map_err(|error| format!("failed to decode function package: {error}"))
}

fn build_preview(package: &FunctionPackage, request: &ValidateFunctionPackageRequest) -> Value {
    json!({
        "kind": "function_package",
        "package": FunctionPackageSummary::from(package),
        "object_type_id": request.object_type_id,
        "target_object_id": request.target_object_id,
        "justification": request.justification,
        "parameter_keys": request
            .parameters
            .as_object()
            .map(|parameters| parameters.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "source_length": package.source.len(),
    })
}

fn parse_parameters(
    parameters: &Value,
) -> Result<std::collections::HashMap<String, Value>, String> {
    let Some(parameters) = parameters.as_object() else {
        return Err("parameters must be a JSON object".to_string());
    };
    Ok(parameters.clone().into_iter().collect())
}

fn build_package_invocation(package: &FunctionPackage) -> Result<ResolvedInlineFunction, String> {
    let config = parse_inline_function_config(&json!({
        "runtime": package.runtime,
        "source": package.source,
    }))?
    .ok_or_else(|| "function package runtime is not supported".to_string())?;
    Ok(ResolvedInlineFunction {
        config,
        capabilities: package.capabilities.clone(),
        package: Some(FunctionPackageSummary::from(package)),
    })
}

fn validate_run_filters(status: Option<&str>, invocation_kind: Option<&str>) -> Result<(), String> {
    if let Some(status) = status {
        if !status.is_empty() && !matches!(status, "success" | "failure") {
            return Err("status filter must be 'success' or 'failure'".to_string());
        }
    }

    if let Some(invocation_kind) = invocation_kind {
        if !invocation_kind.is_empty() && !matches!(invocation_kind, "simulation" | "action") {
            return Err("invocation_kind filter must be 'simulation' or 'action'".to_string());
        }
    }

    Ok(())
}

fn synthetic_action(package: &FunctionPackage, object_type_id: Uuid) -> ActionType {
    ActionType {
        id: package.id,
        name: package.name.clone(),
        display_name: package.display_name.clone(),
        description: package.description.clone(),
        object_type_id,
        operation_kind: "invoke_function".to_string(),
        input_schema: Vec::new(),
        form_schema: Default::default(),
        config: json!({ "function_package_id": package.id }),
        confirmation_required: false,
        permission_key: None,
        authorization_policy: Default::default(),
        owner_id: package.owner_id,
        created_at: package.created_at,
        updated_at: package.updated_at,
    }
}

fn built_in_function_authoring_templates() -> Vec<FunctionAuthoringTemplate> {
    vec![
        FunctionAuthoringTemplate {
            id: "typescript-search-companion".to_string(),
            runtime: "typescript".to_string(),
            display_name: "TypeScript search companion".to_string(),
            description:
                "Read ontology context, query related objects, and optionally summarize results with the LLM."
                    .to_string(),
            entrypoint: "default".to_string(),
            starter_source: r#"export default async function handler(context) {
  const target = context.targetObject;
  const related = await context.sdk.ontology.search({
    query: target?.properties?.name ?? 'high risk case',
    kind: 'object_instance',
    limit: 5,
  });

  const summary = context.capabilities.allowAi
    ? await context.llm.complete({
        userMessage: `Summarize the current operating posture for ${target?.id ?? 'this selection'}.`,
        maxTokens: 160,
      })
    : null;

  return {
    output: {
      inspectedObjectId: target?.id ?? null,
      related,
      summary: summary?.reply ?? null,
    },
  };
}"#
                .to_string(),
            default_capabilities: FunctionCapabilities {
                allow_ontology_read: true,
                allow_ontology_write: false,
                allow_ai: true,
                allow_network: false,
                timeout_seconds: 15,
                max_source_bytes: 65_536,
            },
            recommended_use_cases: vec![
                "semantic retrieval".to_string(),
                "read-only copilots".to_string(),
                "case summarization".to_string(),
            ],
            cli_scaffold_template: Some("function-typescript".to_string()),
            sdk_packages: vec![
                "@open-foundry/sdk".to_string(),
                "@open-foundry/sdk/react".to_string(),
            ],
        },
        FunctionAuthoringTemplate {
            id: "typescript-governed-mutation".to_string(),
            runtime: "typescript".to_string(),
            display_name: "TypeScript governed mutation".to_string(),
            description:
                "Return structured ontology effects such as object patches or link instructions behind an action."
                    .to_string(),
            entrypoint: "default".to_string(),
            starter_source: r#"export default async function handler(context) {
  const target = context.targetObject;

  return {
    output: {
      targetObjectId: target?.id ?? null,
      decidedStatus: 'reviewed',
    },
    object_patch: target
      ? {
          status: 'reviewed',
          review_note: context.parameters.payload?.note ?? 'Reviewed by governed function',
        }
      : null,
  };
}"#
                .to_string(),
            default_capabilities: FunctionCapabilities {
                allow_ontology_read: true,
                allow_ontology_write: true,
                allow_ai: false,
                allow_network: false,
                timeout_seconds: 15,
                max_source_bytes: 65_536,
            },
            recommended_use_cases: vec![
                "action-backed edits".to_string(),
                "governed object patches".to_string(),
                "decision orchestration".to_string(),
            ],
            cli_scaffold_template: Some("function-typescript".to_string()),
            sdk_packages: vec![
                "@open-foundry/sdk".to_string(),
                "@open-foundry/sdk/react".to_string(),
            ],
        },
        FunctionAuthoringTemplate {
            id: "python-analysis-kit".to_string(),
            runtime: "python".to_string(),
            display_name: "Python analysis kit".to_string(),
            description:
                "Use the Python runtime for object inspection, lightweight calculations, and controlled AI-assisted analysis."
                    .to_string(),
            entrypoint: "handler".to_string(),
            starter_source: r#"def handler(context):
    target = context.get("target_object")
    related = context["sdk"].ontology.search(
        query=(target or {}).get("properties", {}).get("name", "high risk case"),
        kind="object_instance",
        limit=5,
    )

    summary = None
    if context["capabilities"].get("allow_ai"):
        summary = context["llm"].complete(
            user_message=f"Summarize object {(target or {}).get('id', 'n/a')} in one sentence.",
            max_tokens=128,
        )

    return {
        "output": {
            "inspectedObjectId": (target or {}).get("id"),
            "related": related,
            "summary": summary,
        }
    }"#
                .to_string(),
            default_capabilities: FunctionCapabilities {
                allow_ontology_read: true,
                allow_ontology_write: false,
                allow_ai: true,
                allow_network: false,
                timeout_seconds: 15,
                max_source_bytes: 65_536,
            },
            recommended_use_cases: vec![
                "python-native analysis".to_string(),
                "operational calculators".to_string(),
                "AI-assisted diagnostics".to_string(),
            ],
            cli_scaffold_template: Some("function-python".to_string()),
            sdk_packages: vec!["openfoundry-sdk".to_string()],
        },
    ]
}

fn function_sdk_packages() -> Vec<FunctionSdkPackageReference> {
    vec![
        FunctionSdkPackageReference {
            language: "typescript".to_string(),
            path: "sdks/typescript/openfoundry-sdk".to_string(),
            package_name: "@open-foundry/sdk".to_string(),
            generated_by:
                "cargo run -p of-cli -- docs generate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk"
                    .to_string(),
        },
        FunctionSdkPackageReference {
            language: "python".to_string(),
            path: "sdks/python/openfoundry-sdk".to_string(),
            package_name: "openfoundry-sdk".to_string(),
            generated_by:
                "cargo run -p of-cli -- docs generate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk"
                    .to_string(),
        },
        FunctionSdkPackageReference {
            language: "java".to_string(),
            path: "sdks/java/openfoundry-sdk".to_string(),
            package_name: "com.openfoundry.sdk".to_string(),
            generated_by:
                "cargo run -p of-cli -- docs generate-sdk-java --input apps/web/static/generated/openapi/openfoundry.json --output sdks/java/openfoundry-sdk"
                    .to_string(),
        },
    ]
}

pub async fn get_function_authoring_surface() -> impl IntoResponse {
    Json(FunctionAuthoringSurfaceResponse {
        templates: built_in_function_authoring_templates(),
        sdk_packages: function_sdk_packages(),
        cli_commands: vec![
            "cargo run -p of-cli -- project init customer-triage --template function-typescript --output packages".to_string(),
            "cargo run -p of-cli -- project init anomaly-diagnostics --template function-python --output packages".to_string(),
            "cargo run -p of-cli -- docs generate-sdk-typescript --input apps/web/static/generated/openapi/openfoundry.json --output sdks/typescript/openfoundry-sdk".to_string(),
            "cargo run -p of-cli -- docs generate-sdk-python --input apps/web/static/generated/openapi/openfoundry.json --output sdks/python/openfoundry-sdk".to_string(),
        ],
    })
}

pub async fn list_function_packages(
    State(state): State<AppState>,
    Query(query): Query<ListFunctionPackagesQuery>,
) -> impl IntoResponse {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let search = query.search.unwrap_or_default();
    let runtime = query.runtime.unwrap_or_default();

    let rows = match sqlx::query_as::<_, FunctionPackageRow>(
        r#"SELECT id, name, version, display_name, description, runtime, source, entrypoint,
                  capabilities, owner_id, created_at, updated_at
           FROM ontology_function_packages
           WHERE ($1 = '' OR runtime = $1)
             AND ($2 = '' OR name ILIKE '%' || $2 || '%' OR display_name ILIKE '%' || $2 || '%')
           ORDER BY name ASC, created_at DESC"#,
    )
    .bind(runtime)
    .bind(search)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return db_error(format!("failed to list function packages: {error}")),
    };

    let packages = match rows
        .into_iter()
        .map(FunctionPackage::try_from)
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(packages) => packages,
        Err(error) => return db_error(format!("failed to decode function packages: {error}")),
    };

    let mut packages = packages;
    packages.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| {
                let left_version = parse_function_package_version(&left.version).ok();
                let right_version = parse_function_package_version(&right.version).ok();
                right_version
                    .cmp(&left_version)
                    .then_with(|| right.version.cmp(&left.version))
            })
            .then_with(|| right.created_at.cmp(&left.created_at))
    });

    let total = packages.len() as i64;
    let offset = ((page - 1) * per_page) as usize;
    let data = packages
        .into_iter()
        .skip(offset)
        .take(per_page as usize)
        .collect::<Vec<_>>();

    Json(ListFunctionPackagesResponse {
        data,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn create_function_package(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreateFunctionPackageRequest>,
) -> impl IntoResponse {
    if body.name.trim().is_empty() {
        return invalid("function package name is required");
    }

    let display_name = body.display_name.unwrap_or_else(|| body.name.clone());
    let description = body.description.unwrap_or_default();
    let entrypoint = body.entrypoint.unwrap_or_else(default_entrypoint);
    let version = body
        .version
        .unwrap_or_else(default_function_package_version);
    let capabilities = body.capabilities.unwrap_or_default();

    if let Err(error) = parse_function_package_version(&version) {
        return invalid(error);
    }

    if let Err(error) =
        validate_package_source(&body.runtime, &body.source, &entrypoint, &capabilities)
    {
        return invalid(error);
    }

    let row = match sqlx::query_as::<_, FunctionPackageRow>(
        r#"INSERT INTO ontology_function_packages (
               id, name, version, display_name, description, runtime, source, entrypoint, capabilities, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10)
           RETURNING id, name, version, display_name, description, runtime, source, entrypoint,
                     capabilities, owner_id, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
    .bind(version)
    .bind(display_name)
    .bind(description)
    .bind(body.runtime)
    .bind(body.source)
    .bind(entrypoint)
    .bind(json!(capabilities))
    .bind(claims.sub)
    .fetch_one(&state.db)
    .await
    {
        Ok(row) => row,
        Err(error) => return db_error(format!("failed to create function package: {error}")),
    };

    match FunctionPackage::try_from(row) {
        Ok(package) => (StatusCode::CREATED, Json(package)).into_response(),
        Err(error) => db_error(format!("failed to decode function package: {error}")),
    }
}

pub async fn get_function_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match load_package(&state, id).await {
        Ok(Some(package)) => Json(package).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => db_error(error),
    }
}

pub async fn list_function_package_runs(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<ListFunctionPackageRunsQuery>,
) -> impl IntoResponse {
    let Some(_) = (match load_package(&state, id).await {
        Ok(package) => package,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if let Err(error) =
        validate_run_filters(query.status.as_deref(), query.invocation_kind.as_deref())
    {
        return invalid(error);
    }

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).clamp(1, 100);
    let status = query.status.unwrap_or_default();
    let invocation_kind = query.invocation_kind.unwrap_or_default();

    let total = match sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(*)
           FROM ontology_function_package_runs
           WHERE function_package_id = $1
             AND ($2 = '' OR status = $2)
             AND ($3 = '' OR invocation_kind = $3)"#,
    )
    .bind(id)
    .bind(&status)
    .bind(&invocation_kind)
    .fetch_one(&state.db)
    .await
    {
        Ok(total) => total,
        Err(error) => return db_error(format!("failed to count function package runs: {error}")),
    };

    let offset = (page - 1) * per_page;
    let data = match sqlx::query_as::<_, FunctionPackageRun>(
        r#"SELECT id, function_package_id, function_package_name, function_package_version, runtime,
                  status, invocation_kind, action_id, action_name, object_type_id,
                  target_object_id, actor_id, duration_ms, error_message, started_at, completed_at
           FROM ontology_function_package_runs
           WHERE function_package_id = $1
             AND ($2 = '' OR status = $2)
             AND ($3 = '' OR invocation_kind = $3)
           ORDER BY completed_at DESC
           OFFSET $4 LIMIT $5"#,
    )
    .bind(id)
    .bind(&status)
    .bind(&invocation_kind)
    .bind(offset)
    .bind(per_page)
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => rows,
        Err(error) => return db_error(format!("failed to load function package runs: {error}")),
    };

    Json(ListFunctionPackageRunsResponse {
        data,
        total,
        page,
        per_page,
    })
    .into_response()
}

pub async fn get_function_package_metrics(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    let Some(package) = (match load_package(&state, id).await {
        Ok(package) => package,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let metrics = match sqlx::query_as::<_, FunctionPackageMetricsRow>(
        r#"SELECT
               COUNT(*)::bigint AS total_runs,
               COUNT(*) FILTER (WHERE status = 'success')::bigint AS successful_runs,
               COUNT(*) FILTER (WHERE status = 'failure')::bigint AS failed_runs,
               COUNT(*) FILTER (WHERE invocation_kind = 'simulation')::bigint AS simulation_runs,
               COUNT(*) FILTER (WHERE invocation_kind = 'action')::bigint AS action_runs,
               AVG(duration_ms)::double precision AS avg_duration_ms,
               percentile_cont(0.95) WITHIN GROUP (ORDER BY duration_ms)::double precision AS p95_duration_ms,
               MAX(duration_ms)::bigint AS max_duration_ms,
               MAX(completed_at) AS last_run_at,
               MAX(completed_at) FILTER (WHERE status = 'success') AS last_success_at,
               MAX(completed_at) FILTER (WHERE status = 'failure') AS last_failure_at
           FROM ontology_function_package_runs
           WHERE function_package_id = $1"#,
    )
    .bind(id)
    .fetch_one(&state.db)
    .await
    {
        Ok(metrics) => metrics,
        Err(error) => return db_error(format!("failed to load function package metrics: {error}")),
    };

    let success_rate = if metrics.total_runs > 0 {
        metrics.successful_runs as f64 / metrics.total_runs as f64
    } else {
        0.0
    };

    Json(FunctionPackageMetricsResponse {
        package: FunctionPackageSummary::from(&package),
        total_runs: metrics.total_runs,
        successful_runs: metrics.successful_runs,
        failed_runs: metrics.failed_runs,
        simulation_runs: metrics.simulation_runs,
        action_runs: metrics.action_runs,
        success_rate,
        avg_duration_ms: metrics.avg_duration_ms,
        p95_duration_ms: metrics.p95_duration_ms,
        max_duration_ms: metrics.max_duration_ms,
        last_run_at: metrics.last_run_at,
        last_success_at: metrics.last_success_at,
        last_failure_at: metrics.last_failure_at,
    })
    .into_response()
}

pub async fn update_function_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateFunctionPackageRequest>,
) -> impl IntoResponse {
    let Some(existing) = (match load_package(&state, id).await {
        Ok(package) => package,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let runtime = body.runtime.unwrap_or(existing.runtime.clone());
    let source = body.source.unwrap_or(existing.source.clone());
    let entrypoint = body.entrypoint.unwrap_or(existing.entrypoint.clone());
    let capabilities = body.capabilities.unwrap_or(existing.capabilities.clone());

    if let Err(error) = validate_package_source(&runtime, &source, &entrypoint, &capabilities) {
        return invalid(error);
    }

    let row = match sqlx::query_as::<_, FunctionPackageRow>(
        r#"UPDATE ontology_function_packages
           SET display_name = COALESCE($2, display_name),
               description = COALESCE($3, description),
               runtime = $4,
               source = $5,
               entrypoint = $6,
               capabilities = $7::jsonb,
               updated_at = NOW()
           WHERE id = $1
           RETURNING id, name, version, display_name, description, runtime, source, entrypoint,
                     capabilities, owner_id, created_at, updated_at"#,
    )
    .bind(id)
    .bind(body.display_name)
    .bind(body.description)
    .bind(runtime)
    .bind(source)
    .bind(entrypoint)
    .bind(json!(capabilities))
    .fetch_optional(&state.db)
    .await
    {
        Ok(Some(row)) => row,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(error) => return db_error(format!("failed to update function package: {error}")),
    };

    match FunctionPackage::try_from(row) {
        Ok(package) => Json(package).into_response(),
        Err(error) => db_error(format!("failed to decode function package: {error}")),
    }
}

pub async fn delete_function_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> impl IntoResponse {
    match sqlx::query("DELETE FROM ontology_function_packages WHERE id = $1")
        .bind(id)
        .execute(&state.db)
        .await
    {
        Ok(result) if result.rows_affected() > 0 => StatusCode::NO_CONTENT.into_response(),
        Ok(_) => StatusCode::NOT_FOUND.into_response(),
        Err(error) => db_error(format!("failed to delete function package: {error}")),
    }
}

pub async fn validate_function_package(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<ValidateFunctionPackageRequest>,
) -> impl IntoResponse {
    let Some(package) = (match load_package(&state, id).await {
        Ok(package) => package,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let preview = build_preview(&package, &body);
    Json(ValidateFunctionPackageResponse {
        valid: true,
        package: FunctionPackageSummary::from(&package),
        preview,
        errors: Vec::new(),
    })
    .into_response()
}

pub async fn simulate_function_package(
    AuthUser(claims): AuthUser,
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<SimulateFunctionPackageRequest>,
) -> impl IntoResponse {
    let Some(package) = (match load_package(&state, id).await {
        Ok(package) => package,
        Err(error) => return db_error(error),
    }) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let target = match body.target_object_id {
        Some(target_object_id) => match load_object_instance(&state.db, target_object_id).await {
            Ok(Some(object)) => {
                if let Err(error) = ensure_object_access(&claims, &object) {
                    return (StatusCode::FORBIDDEN, Json(json!({ "error": error })))
                        .into_response();
                }
                Some(object)
            }
            Ok(None) => return StatusCode::NOT_FOUND.into_response(),
            Err(error) => return db_error(format!("failed to load target object: {error}")),
        },
        None => None,
    };

    let parameters = match parse_parameters(&body.parameters) {
        Ok(parameters) => parameters,
        Err(error) => return invalid(error),
    };

    let resolved = match build_package_invocation(&package) {
        Ok(resolved) => resolved,
        Err(error) => return invalid(error),
    };

    let action = synthetic_action(&package, body.object_type_id);
    let preview = json!({
        "package": FunctionPackageSummary::from(&package),
        "target_object_id": target.as_ref().map(|object| object.id),
        "parameter_keys": parameters.keys().cloned().collect::<Vec<_>>(),
        "capabilities": resolved.capabilities,
    });

    let started_at = Utc::now();
    let timer = Instant::now();
    let outcome = execute_inline_function(
        &state,
        &claims,
        &action,
        target.as_ref(),
        &parameters,
        &resolved,
        body.justification.as_deref(),
    )
    .await;
    let completed_at = Utc::now();
    let duration_ms = timer.elapsed().as_millis() as i64;

    let run_context = FunctionPackageRunContext {
        invocation_kind: "simulation",
        action_id: None,
        action_name: None,
        object_type_id: Some(body.object_type_id),
        target_object_id: target.as_ref().map(|object| object.id),
        actor_id: claims.sub,
    };

    match outcome {
        Ok(result) => {
            if let Err(error) = record_function_package_run(
                &state,
                &FunctionPackageSummary::from(&package),
                &run_context,
                started_at,
                completed_at,
                duration_ms,
                "success",
                None,
            )
            .await
            {
                tracing::warn!(function_package_id = %package.id, %error, "failed to record function package simulation");
            }

            Json(SimulateFunctionPackageResponse {
                package: FunctionPackageSummary::from(&package),
                preview,
                result,
            })
            .into_response()
        }
        Err(error) => {
            if let Err(metrics_error) = record_function_package_run(
                &state,
                &FunctionPackageSummary::from(&package),
                &run_context,
                started_at,
                completed_at,
                duration_ms,
                "failure",
                Some(&error),
            )
            .await
            {
                tracing::warn!(function_package_id = %package.id, %metrics_error, "failed to record function package simulation");
            }

            db_error(error)
        }
    }
}
