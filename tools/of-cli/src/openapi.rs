use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub paths: BTreeMap<String, BTreeMap<String, OpenApiOperation>>,
    pub components: OpenApiComponents,
}

#[derive(Debug, Serialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct OpenApiComponents {
    pub schemas: BTreeMap<String, OpenApiSchema>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenApiSchema {
    #[serde(skip_serializing_if = "Option::is_none", rename = "type")]
    pub schema_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<BTreeMap<String, OpenApiSchema>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<OpenApiSchema>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub reference: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "additionalProperties"
    )]
    pub additional_properties: Option<Box<OpenApiSchema>>,
}

#[derive(Debug, Serialize)]
pub struct OpenApiOperation {
    pub summary: String,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: BTreeMap<String, OpenApiResponse>,
}

#[derive(Debug, Serialize)]
pub struct OpenApiRequestBody {
    pub required: bool,
    pub content: BTreeMap<String, OpenApiMediaType>,
}

#[derive(Debug, Serialize)]
pub struct OpenApiResponse {
    pub description: String,
    pub content: BTreeMap<String, OpenApiMediaType>,
}

#[derive(Debug, Serialize)]
pub struct OpenApiMediaType {
    pub schema: OpenApiSchema,
}

#[derive(Debug)]
struct ProtoService {
    package: String,
    name: String,
    rpcs: Vec<ProtoRpc>,
}

#[derive(Debug)]
struct ProtoRpc {
    name: String,
    request: String,
    response: String,
}

pub fn generate_spec(proto_dir: &Path) -> Result<OpenApiSpec> {
    let mut proto_files = Vec::new();
    collect_proto_files(proto_dir, &mut proto_files)?;

    let service_regex = Regex::new(r"service\s+(?P<name>\w+)\s*\{(?P<body>[\s\S]*?)\}")?;
    let rpc_regex = Regex::new(
        r"rpc\s+(?P<name>\w+)\((?P<request>[^)]+)\)\s+returns\s+\((?P<response>[^)]+)\)",
    )?;
    let package_regex = Regex::new(r"package\s+([a-zA-Z0-9_\.]+)")?;

    let mut services = Vec::new();
    let mut schemas = BTreeMap::new();

    for file in proto_files {
        let content = fs::read_to_string(&file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let package = package_regex
            .captures(&content)
            .and_then(|captures| captures.get(1))
            .map(|value| value.as_str().to_string())
            .unwrap_or_else(|| "open_foundry.unknown".to_string());

        for service_capture in service_regex.captures_iter(&content) {
            let body = service_capture
                .name("body")
                .map(|value| value.as_str())
                .unwrap_or_default();
            let rpcs = rpc_regex
                .captures_iter(body)
                .filter_map(|rpc| {
                    Some(ProtoRpc {
                        name: rpc.name("name")?.as_str().to_string(),
                        request: sanitize_type_name(rpc.name("request")?.as_str()),
                        response: sanitize_type_name(rpc.name("response")?.as_str()),
                    })
                })
                .collect::<Vec<_>>();

            if !rpcs.is_empty() {
                services.push(ProtoService {
                    package: package.clone(),
                    name: service_capture
                        .name("name")
                        .map(|value| value.as_str())
                        .unwrap_or_default()
                        .to_string(),
                    rpcs,
                });
            }
        }

        schemas.extend(parse_message_schemas(&content)?);
    }

    let mut paths = BTreeMap::new();
    for service in &services {
        let base_path = package_to_base_path(&service.package);
        for rpc in &service.rpcs {
            let path = format!("/api/v1/{base_path}/{}", to_kebab_case(&rpc.name));
            let method = http_method_for_rpc(&rpc.name).to_string();
            let request_ref = schema_ref(&rpc.request);
            let response_ref = schema_ref(&rpc.response);
            let operation = OpenApiOperation {
                summary: format!("{} {}", service.name, rpc.name),
                operation_id: format!("{}.{}.{}", service.package, service.name, rpc.name),
                tags: vec![service.package.clone()],
                request_body: if method == "get" || method == "delete" {
                    None
                } else {
                    Some(OpenApiRequestBody {
                        required: true,
                        content: BTreeMap::from([(
                            "application/json".to_string(),
                            OpenApiMediaType {
                                schema: request_ref,
                            },
                        )]),
                    })
                },
                responses: BTreeMap::from([(
                    "200".to_string(),
                    OpenApiResponse {
                        description: "Successful response".to_string(),
                        content: BTreeMap::from([(
                            "application/json".to_string(),
                            OpenApiMediaType {
                                schema: response_ref,
                            },
                        )]),
                    },
                )]),
            };

            paths
                .entry(path)
                .or_insert_with(BTreeMap::new)
                .insert(method, operation);
        }
    }

    Ok(OpenApiSpec {
        openapi: "3.1.0".to_string(),
        info: OpenApiInfo {
            title: "OpenFoundry Proto-Derived API".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description:
                "Auto-generated JSON/HTTP contract derived from OpenFoundry proto services."
                    .to_string(),
        },
        paths,
        components: OpenApiComponents { schemas },
    })
}

pub fn validate_generated_spec(proto_dir: &Path, expected_path: &Path) -> Result<()> {
    let generated = serde_json::to_value(generate_spec(proto_dir)?)?;
    let expected_bytes = fs::read(expected_path).with_context(|| {
        format!(
            "failed to read checked-in OpenAPI spec {}",
            expected_path.display()
        )
    })?;
    let expected: Value = serde_json::from_slice(&expected_bytes).with_context(|| {
        format!(
            "failed to parse checked-in OpenAPI spec {}",
            expected_path.display()
        )
    })?;

    if generated != expected {
        bail!(
            "OpenAPI drift detected in {}. Regenerate it with `cargo run -p of-cli -- docs generate-openapi --output {}`",
            expected_path.display(),
            expected_path.display(),
        );
    }

    Ok(())
}

fn collect_proto_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_proto_files(&path, files)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("proto") {
            files.push(path);
        }
    }
    Ok(())
}

fn parse_message_schemas(content: &str) -> Result<BTreeMap<String, OpenApiSchema>> {
    let message_regex = Regex::new(r"message\s+(?P<name>\w+)\s*\{(?P<body>[\s\S]*?)\}")?;
    let field_regex = Regex::new(
        r"(?P<label>repeated\s+)?(?P<type>map<[^>]+>|[a-zA-Z0-9_\.]+)\s+(?P<name>\w+)\s*=\s*\d+",
    )?;
    let mut schemas = BTreeMap::new();

    for capture in message_regex.captures_iter(content) {
        let name = capture
            .name("name")
            .map(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let body = capture
            .name("body")
            .map(|value| value.as_str())
            .unwrap_or_default();
        let mut properties = BTreeMap::new();
        for field in field_regex.captures_iter(body) {
            let field_name = field
                .name("name")
                .map(|value| value.as_str())
                .unwrap_or_default()
                .to_string();
            let field_type = field
                .name("type")
                .map(|value| value.as_str())
                .unwrap_or_default();
            let is_repeated = field.name("label").is_some();
            let schema = field_schema(field_type, is_repeated);
            properties.insert(field_name, schema);
        }

        schemas.insert(
            name,
            OpenApiSchema {
                schema_type: Some("object".to_string()),
                format: None,
                properties: Some(properties),
                items: None,
                reference: None,
                additional_properties: None,
            },
        );
    }

    Ok(schemas)
}

fn field_schema(field_type: &str, is_repeated: bool) -> OpenApiSchema {
    let schema = if field_type.starts_with("map<") {
        let inner = field_type.trim_start_matches("map<").trim_end_matches('>');
        let value_type = inner.split(',').nth(1).map(str::trim).unwrap_or("string");
        OpenApiSchema {
            schema_type: Some("object".to_string()),
            format: None,
            properties: None,
            items: None,
            reference: None,
            additional_properties: Some(Box::new(field_schema(value_type, false))),
        }
    } else {
        primitive_or_ref(field_type)
    };

    if is_repeated {
        OpenApiSchema {
            schema_type: Some("array".to_string()),
            format: None,
            properties: None,
            items: Some(Box::new(schema)),
            reference: None,
            additional_properties: None,
        }
    } else {
        schema
    }
}

fn primitive_or_ref(field_type: &str) -> OpenApiSchema {
    match field_type {
        "string" | "bytes" => OpenApiSchema {
            schema_type: Some("string".to_string()),
            format: None,
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        "bool" => OpenApiSchema {
            schema_type: Some("boolean".to_string()),
            format: None,
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        "float" | "double" => OpenApiSchema {
            schema_type: Some("number".to_string()),
            format: None,
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        "int32" | "uint32" => OpenApiSchema {
            schema_type: Some("integer".to_string()),
            format: Some("int32".to_string()),
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        "int64" | "uint64" => OpenApiSchema {
            schema_type: Some("integer".to_string()),
            format: Some("int64".to_string()),
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        "google.protobuf.Timestamp" => OpenApiSchema {
            schema_type: Some("string".to_string()),
            format: Some("date-time".to_string()),
            properties: None,
            items: None,
            reference: None,
            additional_properties: None,
        },
        _ => schema_ref(field_type),
    }
}

fn schema_ref(name: &str) -> OpenApiSchema {
    OpenApiSchema {
        schema_type: None,
        format: None,
        properties: None,
        items: None,
        reference: Some(format!("#/components/schemas/{}", sanitize_type_name(name))),
        additional_properties: None,
    }
}

fn sanitize_type_name(name: &str) -> String {
    name.split('.').last().unwrap_or(name).to_string()
}

fn package_to_base_path(package: &str) -> String {
    let segment = package.split('.').last().unwrap_or(package);
    match segment {
        "query" => "queries".to_string(),
        "dataset" => "datasets".to_string(),
        "pipeline" => "pipelines".to_string(),
        "workflow" => "workflows".to_string(),
        "notification" => "notifications".to_string(),
        "app_builder" => "apps".to_string(),
        "report" => "reports".to_string(),
        "code_repo" => "code-repos".to_string(),
        other => other.replace('_', "-"),
    }
}

fn http_method_for_rpc(name: &str) -> &'static str {
    if name.starts_with("List") || name.starts_with("Get") {
        "get"
    } else if name.starts_with("Delete") {
        "delete"
    } else if name.starts_with("Update") {
        "patch"
    } else {
        "post"
    }
}

fn to_kebab_case(value: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in value.chars().enumerate() {
        if ch.is_uppercase() {
            if idx > 0 {
                out.push('-');
            }
            for lowered in ch.to_lowercase() {
                out.push(lowered);
            }
        } else {
            out.push(ch);
        }
    }
    out
}
