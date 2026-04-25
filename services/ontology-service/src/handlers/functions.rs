use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::{Value, json};
use uuid::Uuid;

use auth_middleware::layer::AuthUser;

use crate::{
    AppState,
    domain::{
        access::ensure_object_access,
        function_runtime::{
            ResolvedInlineFunction, execute_inline_function, parse_inline_function_config,
            validate_function_capabilities,
        },
    },
    handlers::objects::load_object_instance,
    models::{
        action_type::ActionType,
        function_package::{
            CreateFunctionPackageRequest, FunctionCapabilities, FunctionPackage,
            FunctionPackageRow, FunctionPackageSummary, ListFunctionPackagesQuery,
            ListFunctionPackagesResponse, SimulateFunctionPackageRequest,
            SimulateFunctionPackageResponse, UpdateFunctionPackageRequest,
            ValidateFunctionPackageRequest, ValidateFunctionPackageResponse,
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
        r#"SELECT id, name, display_name, description, runtime, source, entrypoint,
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

fn parse_parameters(parameters: &Value) -> Result<std::collections::HashMap<String, Value>, String> {
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

fn synthetic_action(package: &FunctionPackage, object_type_id: Uuid) -> ActionType {
    ActionType {
        id: package.id,
        name: package.name.clone(),
        display_name: package.display_name.clone(),
        description: package.description.clone(),
        object_type_id,
        operation_kind: "invoke_function".to_string(),
        input_schema: Vec::new(),
        config: json!({ "function_package_id": package.id }),
        confirmation_required: false,
        permission_key: None,
        owner_id: package.owner_id,
        created_at: package.created_at,
        updated_at: package.updated_at,
    }
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
        r#"SELECT id, name, display_name, description, runtime, source, entrypoint,
                  capabilities, owner_id, created_at, updated_at
           FROM ontology_function_packages
           WHERE ($1 = '' OR runtime = $1)
             AND ($2 = '' OR name ILIKE '%' || $2 || '%' OR display_name ILIKE '%' || $2 || '%')
           ORDER BY created_at DESC"#,
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
    let capabilities = body.capabilities.unwrap_or_default();

    if let Err(error) =
        validate_package_source(&body.runtime, &body.source, &entrypoint, &capabilities)
    {
        return invalid(error);
    }

    let row = match sqlx::query_as::<_, FunctionPackageRow>(
        r#"INSERT INTO ontology_function_packages (
               id, name, display_name, description, runtime, source, entrypoint, capabilities, owner_id
           )
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8::jsonb, $9)
           RETURNING id, name, display_name, description, runtime, source, entrypoint,
                     capabilities, owner_id, created_at, updated_at"#,
    )
    .bind(Uuid::now_v7())
    .bind(body.name.trim())
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
           RETURNING id, name, display_name, description, runtime, source, entrypoint,
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
                    return (
                        StatusCode::FORBIDDEN,
                        Json(json!({ "error": error })),
                    )
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

    match execute_inline_function(
        &state,
        &claims,
        &action,
        target.as_ref(),
        &parameters,
        &resolved,
        body.justification.as_deref(),
    )
    .await
    {
        Ok(result) => Json(SimulateFunctionPackageResponse {
            package: FunctionPackageSummary::from(&package),
            preview,
            result,
        })
        .into_response(),
        Err(error) => db_error(error),
    }
}
