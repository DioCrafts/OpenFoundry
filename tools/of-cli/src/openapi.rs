use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: OpenApiInfo,
    pub paths: BTreeMap<String, BTreeMap<String, OpenApiOperation>>,
    pub components: OpenApiComponents,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiInfo {
    pub title: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiComponents {
    pub schemas: BTreeMap<String, OpenApiSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiOperation {
    pub summary: String,
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<OpenApiParameter>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<OpenApiRequestBody>,
    pub responses: BTreeMap<String, OpenApiResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiParameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    pub required: bool,
    pub schema: OpenApiSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiRequestBody {
    pub required: bool,
    pub content: BTreeMap<String, OpenApiMediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiResponse {
    pub description: String,
    pub content: BTreeMap<String, OpenApiMediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                parameters: None,
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

    let mut spec = OpenApiSpec {
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
    };
    augment_with_rest_overlays(&mut spec);
    Ok(spec)
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

pub fn load_spec(path: &Path) -> Result<OpenApiSpec> {
    let bytes = fs::read(path)
        .with_context(|| format!("failed to read OpenAPI spec {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse OpenAPI spec {}", path.display()))
}

pub fn generate_typescript_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let files = render_typescript_sdk(&spec)?;
    write_generated_files(output_dir, &files)?;
    Ok(())
}

pub fn validate_typescript_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let expected_files = render_typescript_sdk(&spec)?;
    validate_generated_files(
        &expected_files,
        output_dir,
        "TypeScript SDK",
        &format!(
            "cargo run -p of-cli -- docs generate-sdk-typescript --input {} --output {}",
            spec_path.display(),
            output_dir.display()
        ),
    )
}

pub fn generate_python_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let files = render_python_sdk(&spec)?;
    write_generated_files(output_dir, &files)?;
    Ok(())
}

pub fn validate_python_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let expected_files = render_python_sdk(&spec)?;
    validate_generated_files(
        &expected_files,
        output_dir,
        "Python SDK",
        &format!(
            "cargo run -p of-cli -- docs generate-sdk-python --input {} --output {}",
            spec_path.display(),
            output_dir.display()
        ),
    )
}

pub fn generate_java_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let files = render_java_sdk(&spec)?;
    write_generated_files(output_dir, &files)?;
    Ok(())
}

pub fn validate_java_sdk(spec_path: &Path, output_dir: &Path) -> Result<()> {
    let spec = load_spec(spec_path)?;
    let expected_files = render_java_sdk(&spec)?;
    validate_generated_files(
        &expected_files,
        output_dir,
        "Java SDK",
        &format!(
            "cargo run -p of-cli -- docs generate-sdk-java --input {} --output {}",
            spec_path.display(),
            output_dir.display()
        ),
    )
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

fn render_typescript_sdk(spec: &OpenApiSpec) -> Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("package.json"),
        render_typescript_package_json(&spec.info.version)?,
    );
    files.insert(PathBuf::from("tsconfig.json"), render_typescript_tsconfig());
    files.insert(
        PathBuf::from("README.md"),
        render_typescript_readme(&spec.info.version),
    );
    files.insert(PathBuf::from("src/index.ts"), render_typescript_index(spec));
    files.insert(
        PathBuf::from("src/react.ts"),
        render_typescript_react_helper(),
    );
    files.insert(
        PathBuf::from("src/react-shim.d.ts"),
        render_typescript_react_shim(),
    );
    Ok(files)
}

fn validate_generated_files(
    expected_files: &BTreeMap<PathBuf, String>,
    output_dir: &Path,
    label: &str,
    regeneration_command: &str,
) -> Result<()> {
    for (relative_path, expected_content) in expected_files {
        let file_path = output_dir.join(relative_path);
        let actual_content = fs::read_to_string(&file_path).with_context(|| {
            format!(
                "missing generated {label} file {}. Regenerate it with `{regeneration_command}`",
                file_path.display(),
            )
        })?;

        if normalize_line_endings(&actual_content) != normalize_line_endings(expected_content) {
            bail!(
                "{label} drift detected in {}. Regenerate it with `{regeneration_command}`",
                file_path.display(),
            );
        }
    }

    Ok(())
}

fn write_generated_files(output_dir: &Path, files: &BTreeMap<PathBuf, String>) -> Result<()> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    for (relative_path, content) in files {
        let file_path = output_dir.join(relative_path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&file_path, content)
            .with_context(|| format!("failed to write {}", file_path.display()))?;
    }

    Ok(())
}

fn render_typescript_package_json(version: &str) -> Result<String> {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": "@open-foundry/sdk",
        "version": version,
        "private": true,
        "type": "module",
        "description": "Official TypeScript SDK generated from the OpenFoundry OpenAPI contract.",
        "license": "Apache-2.0",
        "main": "./dist/index.js",
        "types": "./dist/index.d.ts",
        "exports": {
            ".": {
                "import": "./dist/index.js",
                "types": "./dist/index.d.ts"
            },
            "./react": {
                "import": "./dist/react.js",
                "types": "./dist/react.d.ts"
            }
        },
        "files": ["dist", "src", "README.md"],
        "scripts": {
            "build": "tsc -p .",
            "check": "tsc -p . --noEmit"
        }
    }))
    .context("failed to serialize TypeScript SDK package.json")
}

fn render_typescript_tsconfig() -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "compilerOptions": {
            "target": "ES2022",
            "module": "ES2022",
            "moduleResolution": "Bundler",
            "strict": true,
            "declaration": true,
            "declarationMap": false,
            "sourceMap": false,
            "outDir": "dist",
            "lib": ["ES2022", "DOM"],
            "skipLibCheck": true
        },
        "include": ["src/**/*.ts", "src/**/*.d.ts"]
    }))
    .unwrap_or_else(|_| "{}".to_string())
}

fn render_typescript_readme(version: &str) -> String {
    format!(
        "# OpenFoundry TypeScript SDK\n\nGenerated from `apps/web/static/generated/openapi/openfoundry.json`.\n\nVersion: `{version}`\n\n## Usage\n\n```ts\nimport {{ OpenFoundryClient }} from '@open-foundry/sdk';\n\nconst client = new OpenFoundryClient({{\n  baseUrl: 'https://platform.example.com',\n  headers: {{ authorization: 'Bearer <token>' }},\n}});\n\nconst me = await client.authAuthGetMe();\n```\n\n## React helpers\n\n```ts\nimport {{ OpenFoundryProvider, useOpenFoundry, useOpenFoundryQuery }} from '@open-foundry/sdk/react';\n\nfunction DatasetCount() {{\n  const client = useOpenFoundry();\n  const datasets = useOpenFoundryQuery(() => client.datasetDatasetListdatasets(), [client]);\n  return <div>{{datasets.data?.datasets?.length ?? 0}}</div>;\n}}\n\nfunction App() {{\n  return (\n    <OpenFoundryProvider options={{{{ baseUrl: 'https://platform.example.com' }}}}>\n      <DatasetCount />\n    </OpenFoundryProvider>\n  );\n}}\n```\n"
    )
}

fn render_typescript_react_helper() -> String {
    [
        "// This file is generated by `cargo run -p of-cli -- docs generate-sdk-typescript`.",
        "// Do not edit manually.",
        "",
        "import { createContext, createElement, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from 'react';",
        "import { OpenFoundryClient, type OpenFoundryClientOptions } from './index';",
        "",
        "export interface OpenFoundryProviderProps {",
        "  client?: OpenFoundryClient;",
        "  options?: OpenFoundryClientOptions;",
        "  children?: ReactNode;",
        "}",
        "",
        "export interface OpenFoundryQueryState<T> {",
        "  data: T | null;",
        "  error: Error | null;",
        "  loading: boolean;",
        "  refetch: () => Promise<T | null>;",
        "}",
        "",
        "export interface OpenFoundryQueryOptions<T> {",
        "  enabled?: boolean;",
        "  initialData?: T | null;",
        "  preservePreviousData?: boolean;",
        "}",
        "",
        "export interface OpenFoundryMutationState<TResult, TArgs extends unknown[]> {",
        "  loading: boolean;",
        "  error: Error | null;",
        "  lastResult: TResult | null;",
        "  mutate: (...args: TArgs) => Promise<TResult>;",
        "  reset: () => void;",
        "}",
        "",
        "export interface OpenFoundryMutationOptions<TResult> {",
        "  onSuccess?: (result: TResult) => void;",
        "  onError?: (error: Error) => void;",
        "}",
        "",
        "const OpenFoundryContext = createContext<OpenFoundryClient | null>(null);",
        "",
        "export function OpenFoundryProvider(props: OpenFoundryProviderProps) {",
        "  const headersKey = stableSerialize(props.options?.headers ?? {});",
        "  const client = useMemo(() => {",
        "    if (props.client) {",
        "      return props.client;",
        "    }",
        "    if (!props.options) {",
        "      throw new Error('OpenFoundryProvider requires either a client or options');",
        "    }",
        "    return new OpenFoundryClient(props.options);",
        "  }, [props.client, props.options?.baseUrl, props.options?.fetch, headersKey]);",
        "  return createElement(OpenFoundryContext.Provider, { value: client }, props.children);",
        "}",
        "",
        "export function useOpenFoundry(): OpenFoundryClient {",
        "  const client = useContext(OpenFoundryContext);",
        "  if (!client) {",
        "    throw new Error('OpenFoundryProvider is missing from the React tree');",
        "  }",
        "  return client;",
        "}",
        "",
        "export function useOpenFoundryClient(options: OpenFoundryClientOptions): OpenFoundryClient {",
        "  const headersKey = stableSerialize(options.headers ?? {});",
        "  return useMemo(",
        "    () => new OpenFoundryClient(options),",
        "    [options.baseUrl, options.fetch, headersKey],",
        "  );",
        "}",
        "",
        "export function useOpenFoundryQuery<T>(",
        "  fetcher: () => Promise<T>,",
        "  deps: readonly unknown[] = [],",
        "  options: OpenFoundryQueryOptions<T> = {},",
        "): OpenFoundryQueryState<T> {",
        "  const fetcherRef = useRef(fetcher);",
        "  fetcherRef.current = fetcher;",
        "  const enabled = options.enabled ?? true;",
        "  const [data, setData] = useState<T | null>(options.initialData ?? null);",
        "  const [error, setError] = useState<Error | null>(null);",
        "  const [loading, setLoading] = useState(enabled);",
        "",
        "  const refetch = async (): Promise<T | null> => {",
        "    if (!enabled) {",
        "      setLoading(false);",
        "      return data;",
        "    }",
        "    setLoading(true);",
        "    setError(null);",
        "    try {",
        "      const result = await fetcherRef.current();",
        "      setData(result);",
        "      return result;",
        "    } catch (cause) {",
        "      const nextError = cause instanceof Error ? cause : new Error('OpenFoundry query failed');",
        "      setError(nextError);",
        "      return null;",
        "    } finally {",
        "      setLoading(false);",
        "    }",
        "  };",
        "",
        "  useEffect(() => {",
        "    if (!enabled) {",
        "      setLoading(false);",
        "      return;",
        "    }",
        "    if (!options.preservePreviousData && options.initialData === undefined) {",
        "      setData(null);",
        "    }",
        "    void refetch();",
        "  }, [enabled, ...deps]);",
        "",
        "  return { data, error, loading, refetch };",
        "}",
        "",
        "export function useOpenFoundryMutation<TResult, TArgs extends unknown[]>(",
        "  mutation: (...args: TArgs) => Promise<TResult>,",
        "  options: OpenFoundryMutationOptions<TResult> = {},",
        "): OpenFoundryMutationState<TResult, TArgs> {",
        "  const mutationRef = useRef(mutation);",
        "  mutationRef.current = mutation;",
        "  const [loading, setLoading] = useState(false);",
        "  const [error, setError] = useState<Error | null>(null);",
        "  const [lastResult, setLastResult] = useState<TResult | null>(null);",
        "",
        "  return {",
        "    loading,",
        "    error,",
        "    lastResult,",
        "    mutate: async (...args: TArgs): Promise<TResult> => {",
        "      setLoading(true);",
        "      setError(null);",
        "      try {",
        "        const result = await mutationRef.current(...args);",
        "        setLastResult(result);",
        "        options.onSuccess?.(result);",
        "        return result;",
        "      } catch (cause) {",
        "        const nextError = cause instanceof Error ? cause : new Error('OpenFoundry mutation failed');",
        "        setError(nextError);",
        "        options.onError?.(nextError);",
        "        throw nextError;",
        "      } finally {",
        "        setLoading(false);",
        "      }",
        "    },",
        "    reset: () => {",
        "      setError(null);",
        "      setLastResult(null);",
        "    },",
        "  };",
        "}",
        "",
        "export function createOpenFoundryQueryKey(...parts: unknown[]): string {",
        "  return stableSerialize(parts);",
        "}",
        "",
        "function stableSerialize(value: unknown): string {",
        "  if (value === null || value === undefined) {",
        "    return '';",
        "  }",
        "  try {",
        "    return JSON.stringify(value, Object.keys(value as Record<string, unknown>).sort());",
        "  } catch (_error) {",
        "    return '';",
        "  }",
        "}",
        "",
    ]
    .join("\n")
}

fn render_typescript_react_shim() -> String {
    [
        "declare module 'react' {",
        "  export type ReactNode = unknown;",
        "  export type SetStateAction<S> = S | ((previousState: S) => S);",
        "  export type Dispatch<A> = (value: A) => void;",
        "  export interface Context<T> {",
        "    Provider: unknown;",
        "  }",
        "  export function createContext<T>(defaultValue: T): Context<T>;",
        "  export function createElement(type: unknown, props: unknown, ...children: unknown[]): unknown;",
        "  export function useEffect(effect: () => void | (() => void), deps?: readonly unknown[]): void;",
        "  export function useContext<T>(context: Context<T>): T;",
        "  export function useMemo<T>(factory: () => T, deps: readonly unknown[]): T;",
        "  export function useRef<T>(initialValue: T): { current: T };",
        "  export function useState<S>(initialState: S | (() => S)): [S, Dispatch<SetStateAction<S>>];",
        "}",
        "",
    ]
    .join("\n")
}

fn render_typescript_index(spec: &OpenApiSpec) -> String {
    let mut lines = vec![
        "// This file is generated by `cargo run -p of-cli -- docs generate-sdk-typescript`.".to_string(),
        "// Do not edit manually.".to_string(),
        String::new(),
        format!("export const OPENFOUNDRY_SDK_VERSION = {:?};", spec.info.version),
        String::new(),
        "export interface OpenFoundryClientOptions {".to_string(),
        "  baseUrl: string;".to_string(),
        "  fetch?: typeof fetch;".to_string(),
        "  headers?: HeadersInit;".to_string(),
        "}".to_string(),
        String::new(),
        "export type OpenFoundryRequestInit = Omit<RequestInit, 'body' | 'method'>;".to_string(),
        "export type OpenFoundryPathParams = Record<string, string | number | boolean>;".to_string(),
        "export type OpenFoundryQuery = Record<string, string | number | boolean | undefined | null>;".to_string(),
        String::new(),
    ];

    for (name, schema) in &spec.components.schemas {
        lines.extend(render_schema_declaration(name, schema));
        lines.push(String::new());
    }

    let referenced_names = collect_referenced_schema_names(spec);
    for name in referenced_names
        .into_iter()
        .filter(|name| !spec.components.schemas.contains_key(name))
    {
        lines.push(format!(
            "export type {} = {};",
            typescript_export_name(&name),
            fallback_typescript_type(&name)
        ));
        lines.push(String::new());
    }

    lines.extend([
        "export class OpenFoundryClient {".to_string(),
        "  private readonly baseUrl: string;".to_string(),
        "  private readonly fetchImpl: typeof fetch;".to_string(),
        "  private readonly defaultHeaders: HeadersInit;".to_string(),
        String::new(),
        "  constructor(options: OpenFoundryClientOptions) {".to_string(),
        "    this.baseUrl = options.baseUrl.replace(/\\/$/, '');".to_string(),
        "    this.fetchImpl = options.fetch ?? fetch;".to_string(),
        "    this.defaultHeaders = options.headers ?? {};".to_string(),
        "  }".to_string(),
        String::new(),
    ]);

    let mut used_method_names = BTreeMap::<String, usize>::new();
    for (path, methods) in &spec.paths {
        for (method, operation) in methods {
            let method_name =
                unique_method_name(method_name_for_operation(operation), &mut used_method_names);
            let request_type = request_type_for_operation(operation);
            let response_type = response_type_for_operation(operation);
            let path_parameters = operation_path_parameters(operation);
            let query_parameters = operation_query_parameters(operation);
            let request_signature = render_typescript_method_signature(
                &path_parameters,
                &query_parameters,
                request_type.as_deref(),
            );
            let path_argument = if path_parameters.is_empty() {
                "undefined".to_string()
            } else {
                render_typescript_path_argument(&path_parameters)
            };
            let query_argument = if query_parameters.is_empty() {
                "undefined".to_string()
            } else {
                "query".to_string()
            };
            let body_argument = if request_type.is_some() {
                "body"
            } else {
                "undefined"
            };

            lines.push(format!(
                "  async {method_name}({request_signature}): Promise<{response_type}> {{"
            ));
            lines.push(format!(
                "    return this.request<{response_type}>({:?}, {:?}, {path_argument}, {query_argument}, {body_argument}, init);",
                method.to_uppercase(),
                path
            ));
            lines.push("  }".to_string());
            lines.push(String::new());
        }
    }

    lines.extend([
        "  private async request<TResponse>(".to_string(),
        "    method: string,".to_string(),
        "    pathTemplate: string,".to_string(),
        "    pathParams: OpenFoundryPathParams | undefined,".to_string(),
        "    query: OpenFoundryQuery | undefined,".to_string(),
        "    body: unknown,".to_string(),
        "    init: OpenFoundryRequestInit,".to_string(),
        "  ): Promise<TResponse> {".to_string(),
        "    const path = this.interpolatePath(pathTemplate, pathParams);".to_string(),
        "    const url = new URL(`${this.baseUrl}${path}`);".to_string(),
        "    if (query) {".to_string(),
        "      for (const [key, value] of Object.entries(query)) {".to_string(),
        "        if (value !== undefined && value !== null) {".to_string(),
        "          url.searchParams.set(key, String(value));".to_string(),
        "        }".to_string(),
        "      }".to_string(),
        "    }".to_string(),
        "    const response = await this.fetchImpl(url.toString(), {".to_string(),
        "      ...init,".to_string(),
        "      method,".to_string(),
        "      headers: {".to_string(),
        "        'content-type': 'application/json',".to_string(),
        "        ...this.defaultHeaders,".to_string(),
        "        ...(init.headers ?? {}),".to_string(),
        "      },".to_string(),
        "      body: body === undefined ? undefined : JSON.stringify(body),".to_string(),
        "    });".to_string(),
        String::new(),
        "    if (!response.ok) {".to_string(),
        "      const message = await response.text();".to_string(),
        "      throw new Error(`OpenFoundry request failed: ${response.status} ${message}`.trim());".to_string(),
        "    }".to_string(),
        String::new(),
        "    if (response.status === 204) {".to_string(),
        "      return undefined as TResponse;".to_string(),
        "    }".to_string(),
        String::new(),
        "    return (await response.json()) as TResponse;".to_string(),
        "  }".to_string(),
        String::new(),
        "  private interpolatePath(pathTemplate: string, pathParams: OpenFoundryPathParams | undefined): string {".to_string(),
        "    if (!pathParams) {".to_string(),
        "      return pathTemplate;".to_string(),
        "    }".to_string(),
        "    return Object.entries(pathParams).reduce(".to_string(),
        "      (path, [key, value]) => path.replace(`{${key}}`, encodeURIComponent(String(value))),".to_string(),
        "      pathTemplate,".to_string(),
        "    );".to_string(),
        "  }".to_string(),
        "}".to_string(),
    ]);

    lines.join("\n")
}

fn render_python_sdk(spec: &OpenApiSpec) -> Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("pyproject.toml"),
        render_python_pyproject(&spec.info.version),
    );
    files.insert(
        PathBuf::from("README.md"),
        render_python_readme(&spec.info.version),
    );
    files.insert(
        PathBuf::from("openfoundry_sdk/__init__.py"),
        render_python_init(),
    );
    files.insert(
        PathBuf::from("openfoundry_sdk/models.py"),
        render_python_models(spec),
    );
    files.insert(
        PathBuf::from("openfoundry_sdk/client.py"),
        render_python_client(spec),
    );
    Ok(files)
}

fn render_python_pyproject(version: &str) -> String {
    format!(
        r#"[build-system]
requires = ["setuptools>=68"]
build-backend = "setuptools.build_meta"

[project]
name = "openfoundry-sdk"
version = "{version}"
description = "Official Python SDK generated from the OpenFoundry OpenAPI contract."
readme = "README.md"
license = {{ text = "Apache-2.0" }}
requires-python = ">=3.11"
dependencies = []

[tool.setuptools.packages.find]
include = ["openfoundry_sdk*"]
"#
    )
}

fn render_python_readme(version: &str) -> String {
    format!(
        "# OpenFoundry Python SDK\n\nGenerated from `apps/web/static/generated/openapi/openfoundry.json`.\n\nVersion: `{version}`\n\n## Usage\n\n```python\nfrom openfoundry_sdk import OpenFoundryClient\n\nclient = OpenFoundryClient(\n    base_url=\"https://platform.example.com\",\n    headers={{\"authorization\": \"Bearer <token>\"}},\n)\n```\n"
    )
}

fn render_python_init() -> String {
    [
        "# This file is generated by `cargo run -p of-cli -- docs generate-sdk-python`.",
        "# Do not edit manually.",
        "",
        "from .client import OpenFoundryClient",
        "from . import models",
        "",
        "__all__ = [\"OpenFoundryClient\", \"models\"]",
        "",
    ]
    .join("\n")
}

fn render_python_models(spec: &OpenApiSpec) -> String {
    let mut lines = vec![
        "# This file is generated by `cargo run -p of-cli -- docs generate-sdk-python`.".to_string(),
        "# Do not edit manually.".to_string(),
        "from __future__ import annotations".to_string(),
        String::new(),
        "import dataclasses".to_string(),
        "from dataclasses import dataclass".to_string(),
        "from typing import Any, get_args, get_origin".to_string(),
        String::new(),
        "JsonValue = Any".to_string(),
        String::new(),
        "def serialize_model(value: Any) -> Any:".to_string(),
        "    if value is None:".to_string(),
        "        return None".to_string(),
        "    if dataclasses.is_dataclass(value):".to_string(),
        "        return {field.name: serialize_model(getattr(value, field.name)) for field in dataclasses.fields(value) if getattr(value, field.name) is not None}".to_string(),
        "    if isinstance(value, list):".to_string(),
        "        return [serialize_model(item) for item in value]".to_string(),
        "    if isinstance(value, dict):".to_string(),
        "        return {key: serialize_model(item) for key, item in value.items() if item is not None}".to_string(),
        "    return value".to_string(),
        String::new(),
        "def deserialize_model(model_type: Any, value: Any) -> Any:".to_string(),
        "    if value is None:".to_string(),
        "        return None".to_string(),
        "    origin = get_origin(model_type)".to_string(),
        "    if origin is list:".to_string(),
        "        args = get_args(model_type)".to_string(),
        "        item_type = args[0] if args else Any".to_string(),
        "        return [deserialize_model(item_type, item) for item in value]".to_string(),
        "    if origin is dict:".to_string(),
        "        args = get_args(model_type)".to_string(),
        "        value_type = args[1] if len(args) == 2 else Any".to_string(),
        "        return {str(key): deserialize_model(value_type, item) for key, item in value.items()}".to_string(),
        "    if origin is not None and str(origin).endswith('Union') or origin is getattr(__import__('types'), 'UnionType', object()):".to_string(),
        "        args = [candidate for candidate in get_args(model_type) if candidate is not type(None)]".to_string(),
        "        if len(args) == 1:".to_string(),
        "            return deserialize_model(args[0], value)".to_string(),
        "        return value".to_string(),
        "    if model_type in (Any, str, int, float, bool):".to_string(),
        "        return value".to_string(),
        "    if isinstance(model_type, type) and dataclasses.is_dataclass(model_type) and isinstance(value, dict):".to_string(),
        "        payload = {}".to_string(),
        "        for field in dataclasses.fields(model_type):".to_string(),
        "            payload[field.name] = deserialize_model(field.type, value.get(field.name))".to_string(),
        "        return model_type(**payload)".to_string(),
        "    return value".to_string(),
        String::new(),
    ];

    for (name, schema) in &spec.components.schemas {
        lines.extend(render_python_schema_declaration(name, schema));
        lines.push(String::new());
    }

    let referenced_names = collect_referenced_schema_names(spec);
    for name in referenced_names
        .into_iter()
        .filter(|name| !spec.components.schemas.contains_key(name))
    {
        lines.push(format!(
            "{} = {}",
            python_export_name(&name),
            python_fallback_type(&name)
        ));
        lines.push(String::new());
    }

    lines.join("\n")
}

fn render_python_client(spec: &OpenApiSpec) -> String {
    let mut lines = vec![
        "# This file is generated by `cargo run -p of-cli -- docs generate-sdk-python`.".to_string(),
        "# Do not edit manually.".to_string(),
        "from __future__ import annotations".to_string(),
        String::new(),
        "import json".to_string(),
        "import urllib.error".to_string(),
        "import urllib.parse".to_string(),
        "import urllib.request".to_string(),
        "from typing import Any, Mapping".to_string(),
        String::new(),
        "from . import models".to_string(),
        String::new(),
        "class OpenFoundryClient:".to_string(),
        "    def __init__(self, base_url: str, headers: Mapping[str, str] | None = None) -> None:".to_string(),
        "        self.base_url = base_url.rstrip('/')".to_string(),
        "        self.default_headers = dict(headers or {})".to_string(),
        String::new(),
        "    def _request(".to_string(),
        "        self,".to_string(),
        "        method: str,".to_string(),
        "        path_template: str,".to_string(),
        "        path_params: Mapping[str, Any] | None = None,".to_string(),
        "        query: Mapping[str, Any] | None = None,".to_string(),
        "        body: Any = None,".to_string(),
        "        headers: Mapping[str, str] | None = None,".to_string(),
        "    ) -> Any:".to_string(),
        "        path = path_template".to_string(),
        "        for key, value in (path_params or {}).items():".to_string(),
        "            path = path.replace('{' + key + '}', urllib.parse.quote(str(value), safe=''))".to_string(),
        "        url = f\"{self.base_url}{path}\"".to_string(),
        "        if query:".to_string(),
        "            filtered = {key: value for key, value in query.items() if value is not None}".to_string(),
        "            if filtered:".to_string(),
        "                url = f\"{url}?{urllib.parse.urlencode(filtered)}\"".to_string(),
        "        payload = None".to_string(),
        "        request_headers = {'content-type': 'application/json', **self.default_headers, **dict(headers or {})}".to_string(),
        "        if body is not None:".to_string(),
        "            payload = json.dumps(models.serialize_model(body)).encode('utf-8')".to_string(),
        "        request = urllib.request.Request(url=url, data=payload, method=method, headers=request_headers)".to_string(),
        "        try:".to_string(),
        "            with urllib.request.urlopen(request) as response:".to_string(),
        "                raw = response.read()".to_string(),
        "                if response.status == 204 or not raw:".to_string(),
        "                    return None".to_string(),
        "                return json.loads(raw.decode('utf-8'))".to_string(),
        "        except urllib.error.HTTPError as error:".to_string(),
        "            message = error.read().decode('utf-8', errors='replace')".to_string(),
        "            raise RuntimeError(f'OpenFoundry request failed: {error.code} {message}'.strip()) from error".to_string(),
        String::new(),
    ];

    let mut used_method_names = BTreeMap::<String, usize>::new();
    for (path, methods) in &spec.paths {
        for (method, operation) in methods {
            let method_name = unique_method_name(
                python_method_name(&method_name_for_operation(operation)),
                &mut used_method_names,
            );
            let request_type =
                request_type_for_operation(operation).map(|value| format!("models.{value}"));
            let response_type = format!("models.{}", response_type_for_operation(operation));
            let path_parameters = operation_path_parameters(operation);
            let query_parameters = operation_query_parameters(operation);
            let signature = render_python_method_signature(
                &path_parameters,
                &query_parameters,
                request_type.as_deref(),
            );
            let path_argument = if path_parameters.is_empty() {
                "None".to_string()
            } else {
                render_python_path_argument(&path_parameters)
            };
            let query_argument = if query_parameters.is_empty() {
                "None".to_string()
            } else {
                "{".to_string()
                    + &query_parameters
                        .iter()
                        .map(|parameter| {
                            let variable_name = render_python_variable_name(&parameter.name);
                            format!("{:?}: {variable_name}", parameter.name)
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                    + "}"
            };
            let body_argument = if request_type.is_some() {
                "body"
            } else {
                "None"
            };

            lines.push(format!(
                "    def {method_name}({signature}) -> {response_type}:"
            ));
            lines.push(format!(
                "        payload = self._request({:?}, {:?}, {}, {}, {}, headers=headers)",
                method.to_uppercase(),
                path,
                path_argument,
                query_argument,
                body_argument
            ));
            lines.push(format!(
                "        return models.deserialize_model({response_type}, payload)"
            ));
            lines.push(String::new());
        }
    }

    lines.join("\n")
}

fn render_java_sdk(spec: &OpenApiSpec) -> Result<BTreeMap<PathBuf, String>> {
    let mut files = BTreeMap::new();
    files.insert(
        PathBuf::from("pom.xml"),
        render_java_pom(&spec.info.version),
    );
    files.insert(
        PathBuf::from("README.md"),
        render_java_readme(&spec.info.version),
    );
    files.insert(
        PathBuf::from("src/main/java/com/openfoundry/sdk/OpenFoundryClient.java"),
        render_java_client(spec),
    );
    Ok(files)
}

fn render_java_pom(version: &str) -> String {
    format!(
        r#"<project xmlns="http://maven.apache.org/POM/4.0.0" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://maven.apache.org/POM/4.0.0 https://maven.apache.org/xsd/maven-4.0.0.xsd">
  <modelVersion>4.0.0</modelVersion>
  <groupId>com.openfoundry</groupId>
  <artifactId>openfoundry-sdk</artifactId>
  <version>{version}</version>
  <name>OpenFoundry Java SDK</name>
  <description>Official Java SDK generated from the OpenFoundry OpenAPI contract.</description>
  <properties>
    <maven.compiler.source>17</maven.compiler.source>
    <maven.compiler.target>17</maven.compiler.target>
    <project.build.sourceEncoding>UTF-8</project.build.sourceEncoding>
  </properties>
</project>
"#
    )
}

fn render_java_readme(version: &str) -> String {
    format!(
        "# OpenFoundry Java SDK\n\nGenerated from `apps/web/static/generated/openapi/openfoundry.json`.\n\nVersion: `{version}`\n\n## Usage\n\n```java\nvar client = new OpenFoundryClient(\"https://platform.example.com\");\nvar meJson = client.restAdminV2UsersMe();\n```\n"
    )
}

fn render_java_client(spec: &OpenApiSpec) -> String {
    let mut lines = vec![
        "package com.openfoundry.sdk;".to_string(),
        String::new(),
        "// This file is generated by `cargo run -p of-cli -- docs generate-sdk-java`.".to_string(),
        "// Do not edit manually.".to_string(),
        String::new(),
        "import java.io.IOException;".to_string(),
        "import java.net.URI;".to_string(),
        "import java.net.URLEncoder;".to_string(),
        "import java.net.http.HttpClient;".to_string(),
        "import java.net.http.HttpRequest;".to_string(),
        "import java.net.http.HttpResponse;".to_string(),
        "import java.nio.charset.StandardCharsets;".to_string(),
        "import java.util.LinkedHashMap;".to_string(),
        "import java.util.Map;".to_string(),
        String::new(),
        "public final class OpenFoundryClient {".to_string(),
        "    private final String baseUrl;".to_string(),
        "    private final HttpClient httpClient;".to_string(),
        "    private final Map<String, String> defaultHeaders;".to_string(),
        String::new(),
        "    public OpenFoundryClient(String baseUrl) {".to_string(),
        "        this(baseUrl, HttpClient.newHttpClient(), Map.of());".to_string(),
        "    }".to_string(),
        String::new(),
        "    public OpenFoundryClient(String baseUrl, HttpClient httpClient, Map<String, String> headers) {".to_string(),
        "        this.baseUrl = baseUrl.endsWith(\"/\") ? baseUrl.substring(0, baseUrl.length() - 1) : baseUrl;".to_string(),
        "        this.httpClient = httpClient;".to_string(),
        "        this.defaultHeaders = new LinkedHashMap<>(headers);".to_string(),
        "    }".to_string(),
        String::new(),
    ];

    let mut used_method_names = BTreeMap::<String, usize>::new();
    for (path, methods) in &spec.paths {
        for (method, operation) in methods {
            let method_name =
                unique_method_name(method_name_for_operation(operation), &mut used_method_names);
            let path_parameters = operation_path_parameters(operation);
            let query_parameters = operation_query_parameters(operation);
            let request_type = request_type_for_operation(operation);
            let signature = render_java_method_signature(
                &path_parameters,
                &query_parameters,
                request_type.is_some(),
            );
            lines.push(format!(
                "    public String {method_name}({signature}) throws IOException, InterruptedException {{"
            ));
            if !path_parameters.is_empty() {
                lines.push(
                    "        Map<String, Object> pathParams = new LinkedHashMap<>();".to_string(),
                );
                for parameter in &path_parameters {
                    let variable_name = render_java_variable_name(&parameter.name);
                    lines.push(format!(
                        "        pathParams.put({:?}, {});",
                        parameter.name, variable_name
                    ));
                }
            } else {
                lines.push("        Map<String, Object> pathParams = Map.of();".to_string());
            }
            if !query_parameters.is_empty() {
                lines.push(
                    "        Map<String, Object> queryParams = new LinkedHashMap<>();".to_string(),
                );
                for parameter in &query_parameters {
                    let variable_name = render_java_variable_name(&parameter.name);
                    lines.push(format!(
                        "        if ({} != null) {{ queryParams.put({:?}, {}); }}",
                        variable_name, parameter.name, variable_name
                    ));
                }
            } else {
                lines.push("        Map<String, Object> queryParams = Map.of();".to_string());
            }
            let body_argument = if request_type.is_some() {
                "bodyJson"
            } else {
                "null"
            };
            lines.push(format!(
                "        return request({:?}, {:?}, pathParams, queryParams, {body_argument});",
                method.to_uppercase(),
                path,
            ));
            lines.push("    }".to_string());
            lines.push(String::new());
        }
    }

    lines.extend([
        "    private String request(String method, String pathTemplate, Map<String, Object> pathParams, Map<String, Object> queryParams, String bodyJson) throws IOException, InterruptedException {".to_string(),
        "        String path = interpolatePath(pathTemplate, pathParams);".to_string(),
        "        String url = buildUrl(path, queryParams);".to_string(),
        "        HttpRequest.BodyPublisher publisher = bodyJson == null ? HttpRequest.BodyPublishers.noBody() : HttpRequest.BodyPublishers.ofString(bodyJson);".to_string(),
        "        HttpRequest.Builder builder = HttpRequest.newBuilder(URI.create(url)).method(method, publisher).header(\"content-type\", \"application/json\");".to_string(),
        "        for (Map.Entry<String, String> entry : defaultHeaders.entrySet()) {".to_string(),
        "            builder.header(entry.getKey(), entry.getValue());".to_string(),
        "        }".to_string(),
        "        HttpResponse<String> response = httpClient.send(builder.build(), HttpResponse.BodyHandlers.ofString());".to_string(),
        "        if (response.statusCode() == 204) {".to_string(),
        "            return \"\";".to_string(),
        "        }".to_string(),
        "        if (response.statusCode() < 200 || response.statusCode() >= 300) {".to_string(),
        "            throw new IOException(\"OpenFoundry request failed: \" + response.statusCode() + \" \" + response.body());".to_string(),
        "        }".to_string(),
        "        return response.body();".to_string(),
        "    }".to_string(),
        String::new(),
        "    private String interpolatePath(String pathTemplate, Map<String, Object> pathParams) {".to_string(),
        "        String path = pathTemplate;".to_string(),
        "        for (Map.Entry<String, Object> entry : pathParams.entrySet()) {".to_string(),
        "            path = path.replace(\"{\" + entry.getKey() + \"}\", urlEncode(String.valueOf(entry.getValue())));".to_string(),
        "        }".to_string(),
        "        return path;".to_string(),
        "    }".to_string(),
        String::new(),
        "    private String buildUrl(String path, Map<String, Object> queryParams) {".to_string(),
        "        if (queryParams.isEmpty()) {".to_string(),
        "            return baseUrl + path;".to_string(),
        "        }".to_string(),
        "        StringBuilder query = new StringBuilder();".to_string(),
        "        for (Map.Entry<String, Object> entry : queryParams.entrySet()) {".to_string(),
        "            if (entry.getValue() == null) {".to_string(),
        "                continue;".to_string(),
        "            }".to_string(),
        "            if (query.length() > 0) {".to_string(),
        "                query.append('&');".to_string(),
        "            }".to_string(),
        "            query.append(urlEncode(entry.getKey())).append('=').append(urlEncode(String.valueOf(entry.getValue())));".to_string(),
        "        }".to_string(),
        "        return query.length() == 0 ? baseUrl + path : baseUrl + path + \"?\" + query;".to_string(),
        "    }".to_string(),
        String::new(),
        "    private String urlEncode(String value) {".to_string(),
        "        return URLEncoder.encode(value, StandardCharsets.UTF_8);".to_string(),
        "    }".to_string(),
        "}".to_string(),
    ]);

    lines.join("\n")
}

fn render_python_schema_declaration(name: &str, schema: &OpenApiSchema) -> Vec<String> {
    let export_name = python_export_name(name);
    if is_object_schema(schema) && schema.additional_properties.is_none() {
        let mut lines = vec![
            "@dataclass(slots=True)".to_string(),
            format!("class {export_name}:"),
        ];
        let properties = schema.properties.as_ref().cloned().unwrap_or_default();
        if properties.is_empty() {
            lines.push("    pass".to_string());
            return lines;
        }
        for (property_name, property_schema) in &properties {
            lines.push(format!(
                "    {}: {} | None = None",
                render_python_variable_name(property_name),
                python_type(property_schema)
            ));
        }
        lines.push(String::new());
        lines.push("    @classmethod".to_string());
        lines.push(format!(
            "    def from_dict(cls, data: dict[str, Any]) -> \"{export_name}\":"
        ));
        lines.push("        return deserialize_model(cls, data)".to_string());
        lines.push(String::new());
        lines.push("    def to_dict(self) -> dict[str, Any]:".to_string());
        lines.push("        return serialize_model(self)".to_string());
        return lines;
    }

    vec![format!("{} = {}", export_name, python_type(schema))]
}

fn render_python_method_signature(
    path_parameters: &[&OpenApiParameter],
    query_parameters: &[&OpenApiParameter],
    request_type: Option<&str>,
) -> String {
    let mut parts = vec!["self".to_string()];

    for parameter in path_parameters {
        parts.push(format!(
            "{}: {}",
            render_python_variable_name(&parameter.name),
            python_type(&parameter.schema)
        ));
    }

    for parameter in query_parameters {
        parts.push(format!(
            "{}: {} | None = None",
            render_python_variable_name(&parameter.name),
            python_type(&parameter.schema)
        ));
    }

    if let Some(request_type) = request_type {
        parts.push(format!("body: {request_type}"));
    }

    parts.push("headers: Mapping[str, str] | None = None".to_string());
    parts.join(", ")
}

fn render_python_path_argument(path_parameters: &[&OpenApiParameter]) -> String {
    "{".to_string()
        + &path_parameters
            .iter()
            .map(|parameter| {
                let variable_name = render_python_variable_name(&parameter.name);
                format!("{:?}: {variable_name}", parameter.name)
            })
            .collect::<Vec<_>>()
            .join(", ")
        + "}"
}

fn render_java_method_signature(
    path_parameters: &[&OpenApiParameter],
    query_parameters: &[&OpenApiParameter],
    has_body: bool,
) -> String {
    let mut parts = Vec::new();
    for parameter in path_parameters {
        parts.push(format!(
            "{} {}",
            java_type(&parameter.schema),
            render_java_variable_name(&parameter.name)
        ));
    }
    for parameter in query_parameters {
        parts.push(format!(
            "{} {}",
            boxed_java_type(&parameter.schema),
            render_java_variable_name(&parameter.name)
        ));
    }
    if has_body {
        parts.push("String bodyJson".to_string());
    }
    parts.join(", ")
}

fn render_schema_declaration(name: &str, schema: &OpenApiSchema) -> Vec<String> {
    let export_name = typescript_export_name(name);
    if is_object_schema(schema) {
        let mut lines = vec![format!("export interface {export_name} {{")];
        for (property_name, property_schema) in schema.properties.as_ref().into_iter().flatten() {
            lines.push(format!(
                "  {}?: {};",
                render_property_name(property_name),
                typescript_type(property_schema)
            ));
        }
        if let Some(additional_properties) = &schema.additional_properties {
            lines.push(format!(
                "  [key: string]: {};",
                typescript_type(additional_properties)
            ));
        }
        lines.push("}".to_string());
        return lines;
    }

    vec![format!(
        "export type {export_name} = {};",
        typescript_type(schema)
    )]
}

fn typescript_type(schema: &OpenApiSchema) -> String {
    if let Some(reference) = &schema.reference {
        return reference
            .rsplit('/')
            .next()
            .map(typescript_export_name)
            .unwrap_or_else(|| "unknown".to_string());
    }

    match schema.schema_type.as_deref() {
        Some("string") => "string".to_string(),
        Some("integer") | Some("number") => "number".to_string(),
        Some("boolean") => "boolean".to_string(),
        Some("array") => {
            let item_type = schema
                .items
                .as_deref()
                .map(typescript_type)
                .unwrap_or_else(|| "unknown".to_string());
            format!("Array<{item_type}>")
        }
        Some("object") => {
            if let Some(additional_properties) = &schema.additional_properties {
                return format!("Record<string, {}>", typescript_type(additional_properties));
            }

            if let Some(properties) = &schema.properties {
                let fields = properties
                    .iter()
                    .map(|(name, property_schema)| {
                        format!(
                            "{}?: {}",
                            render_property_name(name),
                            typescript_type(property_schema)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                return format!("{{ {fields} }}");
            }

            "Record<string, unknown>".to_string()
        }
        _ => {
            if let Some(properties) = &schema.properties {
                let fields = properties
                    .iter()
                    .map(|(name, property_schema)| {
                        format!(
                            "{}?: {}",
                            render_property_name(name),
                            typescript_type(property_schema)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                format!("{{ {fields} }}")
            } else {
                "unknown".to_string()
            }
        }
    }
}

fn collect_referenced_schema_names(spec: &OpenApiSpec) -> BTreeSet<String> {
    let mut names = BTreeSet::new();

    for schema in spec.components.schemas.values() {
        collect_references_from_schema(schema, &mut names);
    }

    for operation in spec.paths.values().flat_map(|methods| methods.values()) {
        if let Some(request_body) = &operation.request_body {
            for media_type in request_body.content.values() {
                collect_references_from_schema(&media_type.schema, &mut names);
            }
        }

        for response in operation.responses.values() {
            for media_type in response.content.values() {
                collect_references_from_schema(&media_type.schema, &mut names);
            }
        }
    }

    names
}

fn collect_references_from_schema(schema: &OpenApiSchema, names: &mut BTreeSet<String>) {
    if let Some(reference) = &schema.reference {
        if let Some(name) = reference.rsplit('/').next() {
            names.insert(name.to_string());
        }
    }

    if let Some(properties) = &schema.properties {
        for property_schema in properties.values() {
            collect_references_from_schema(property_schema, names);
        }
    }

    if let Some(items) = &schema.items {
        collect_references_from_schema(items, names);
    }

    if let Some(additional_properties) = &schema.additional_properties {
        collect_references_from_schema(additional_properties, names);
    }
}

fn request_type_for_operation(operation: &OpenApiOperation) -> Option<String> {
    operation
        .request_body
        .as_ref()
        .and_then(|request_body| request_body.content.values().next())
        .map(|media_type| typescript_type(&media_type.schema))
}

fn response_type_for_operation(operation: &OpenApiOperation) -> String {
    operation
        .responses
        .get("200")
        .and_then(|response| response.content.values().next())
        .map(|media_type| typescript_type(&media_type.schema))
        .unwrap_or_else(|| "unknown".to_string())
}

fn operation_path_parameters(operation: &OpenApiOperation) -> Vec<&OpenApiParameter> {
    operation
        .parameters
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|parameter| parameter.location == "path")
        .collect()
}

fn operation_query_parameters(operation: &OpenApiOperation) -> Vec<&OpenApiParameter> {
    operation
        .parameters
        .as_ref()
        .into_iter()
        .flatten()
        .filter(|parameter| parameter.location == "query")
        .collect()
}

fn render_typescript_method_signature(
    path_parameters: &[&OpenApiParameter],
    query_parameters: &[&OpenApiParameter],
    request_type: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    for parameter in path_parameters {
        parts.push(format!(
            "{}: {}",
            render_typescript_variable_name(&parameter.name),
            typescript_type(&parameter.schema)
        ));
    }

    if !query_parameters.is_empty() {
        let fields = query_parameters
            .iter()
            .map(|parameter| {
                format!(
                    "{}{}: {}",
                    render_property_name(&parameter.name),
                    if parameter.required { "" } else { "?" },
                    typescript_type(&parameter.schema)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        parts.push(format!("query: {{ {fields} }} = {{}}"));
    }

    if let Some(request_type) = request_type {
        parts.push(format!("body: {request_type}"));
    }

    parts.push("init: OpenFoundryRequestInit = {}".to_string());
    parts.join(", ")
}

fn render_typescript_path_argument(path_parameters: &[&OpenApiParameter]) -> String {
    let fields = path_parameters
        .iter()
        .map(|parameter| {
            let variable_name = render_typescript_variable_name(&parameter.name);
            format!("{:?}: {variable_name}", parameter.name)
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("{{ {fields} }}")
}

fn method_name_for_operation(operation: &OpenApiOperation) -> String {
    let parts = operation.operation_id.split('.').collect::<Vec<_>>();
    let package_leaf = parts
        .get(parts.len().saturating_sub(3))
        .copied()
        .unwrap_or("openfoundry");
    let service = parts
        .get(parts.len().saturating_sub(2))
        .copied()
        .unwrap_or("service")
        .trim_end_matches("Service");
    let rpc = parts.last().copied().unwrap_or("call");
    format!(
        "{}{}{}",
        to_camel_case(package_leaf),
        to_pascal_case(service),
        to_pascal_case(rpc)
    )
}

fn unique_method_name(name: String, used_method_names: &mut BTreeMap<String, usize>) -> String {
    let counter = used_method_names.entry(name.clone()).or_insert(0);
    if *counter == 0 {
        *counter += 1;
        return name;
    }

    *counter += 1;
    format!("{name}{}", counter)
}

fn render_property_name(name: &str) -> String {
    if is_typescript_identifier(name) {
        name.to_string()
    } else {
        format!("{name:?}")
    }
}

fn render_typescript_variable_name(name: &str) -> String {
    if is_typescript_identifier(name) {
        name.to_string()
    } else {
        name.chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '_' || character == '$' {
                    character
                } else {
                    '_'
                }
            })
            .collect()
    }
}

fn render_python_variable_name(name: &str) -> String {
    name.chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn render_java_variable_name(name: &str) -> String {
    let mut parts = name
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return "value".to_string();
    }
    let first = parts.remove(0).to_ascii_lowercase();
    let rest = parts
        .into_iter()
        .map(to_pascal_case)
        .collect::<Vec<_>>()
        .join("");
    format!("{first}{rest}")
}

fn typescript_export_name(name: &str) -> String {
    let mut sanitized = String::new();
    let mut capitalize_next = true;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if capitalize_next {
                sanitized.push(character.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                sanitized.push(character);
            }
        } else {
            capitalize_next = true;
        }
    }

    if sanitized.is_empty() {
        "UnknownSchema".to_string()
    } else {
        sanitized
    }
}

fn python_export_name(name: &str) -> String {
    typescript_export_name(name)
}

fn python_method_name(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_uppercase() {
            if index > 0 {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else {
            output.push(character);
        }
    }
    if output.is_empty() {
        "operation".to_string()
    } else {
        output
    }
}

fn to_camel_case(name: &str) -> String {
    let pascal = to_pascal_case(name);
    let mut characters = pascal.chars();
    match characters.next() {
        Some(first) => first.to_ascii_lowercase().to_string() + characters.as_str(),
        None => "operation".to_string(),
    }
}

fn to_pascal_case(name: &str) -> String {
    let mut output = String::new();
    let mut uppercase_next = true;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if uppercase_next {
                output.push(character.to_ascii_uppercase());
                uppercase_next = false;
            } else {
                output.push(character.to_ascii_lowercase());
            }
        } else {
            uppercase_next = true;
        }
    }

    if output.is_empty() {
        "Operation".to_string()
    } else {
        output
    }
}

fn python_type(schema: &OpenApiSchema) -> String {
    if let Some(reference) = &schema.reference {
        return reference
            .rsplit('/')
            .next()
            .map(python_export_name)
            .unwrap_or_else(|| "Any".to_string());
    }

    match schema.schema_type.as_deref() {
        Some("string") => "str".to_string(),
        Some("integer") => "int".to_string(),
        Some("number") => "float".to_string(),
        Some("boolean") => "bool".to_string(),
        Some("array") => {
            let item_type = schema
                .items
                .as_deref()
                .map(python_type)
                .unwrap_or_else(|| "Any".to_string());
            format!("list[{item_type}]")
        }
        Some("object") => {
            if let Some(additional_properties) = &schema.additional_properties {
                return format!("dict[str, {}]", python_type(additional_properties));
            }
            if schema.properties.is_some() {
                "dict[str, Any]".to_string()
            } else {
                "dict[str, Any]".to_string()
            }
        }
        _ => "Any".to_string(),
    }
}

fn java_type(schema: &OpenApiSchema) -> String {
    if schema.reference.is_some() {
        return "String".to_string();
    }
    match schema.schema_type.as_deref() {
        Some("string") => "String".to_string(),
        Some("integer") => "long".to_string(),
        Some("number") => "double".to_string(),
        Some("boolean") => "boolean".to_string(),
        _ => "String".to_string(),
    }
}

fn boxed_java_type(schema: &OpenApiSchema) -> String {
    if schema.reference.is_some() {
        return "String".to_string();
    }
    match schema.schema_type.as_deref() {
        Some("integer") => "Long".to_string(),
        Some("number") => "Double".to_string(),
        Some("boolean") => "Boolean".to_string(),
        _ => "String".to_string(),
    }
}

fn is_typescript_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' || first == '$' => {}
        _ => return false,
    }

    characters
        .all(|character| character.is_ascii_alphanumeric() || character == '_' || character == '$')
}

fn is_object_schema(schema: &OpenApiSchema) -> bool {
    schema.schema_type.as_deref() == Some("object")
        || schema.properties.is_some()
        || schema.additional_properties.is_some()
}

fn fallback_typescript_type(name: &str) -> &'static str {
    match name {
        "Uuid" => "string",
        "Value" => "unknown",
        _ if name.ends_with("Status")
            || name.ends_with("Type")
            || name.ends_with("Format")
            || name.ends_with("Mode") =>
        {
            "string"
        }
        _ => "unknown",
    }
}

fn python_fallback_type(name: &str) -> &'static str {
    match name {
        "Uuid" => "str",
        "Value" => "Any",
        _ if name.ends_with("Status")
            || name.ends_with("Type")
            || name.ends_with("Format")
            || name.ends_with("Mode") =>
        {
            "str"
        }
        _ => "Any",
    }
}

fn normalize_line_endings(value: &str) -> String {
    value.replace("\r\n", "\n")
}

fn augment_with_rest_overlays(spec: &mut OpenApiSpec) {
    let schemas = &mut spec.components.schemas;

    schemas.insert("UserResponse".to_string(), user_response_schema());
    schemas.insert(
        "UpdateUserRequest".to_string(),
        update_user_request_schema(),
    );
    schemas.insert("Permission".to_string(), permission_schema());
    schemas.insert(
        "CreatePermissionRequest".to_string(),
        create_permission_request_schema(),
    );
    schemas.insert("RoleResponse".to_string(), role_response_schema());
    schemas.insert(
        "CreateRoleRequest".to_string(),
        create_role_request_schema(),
    );
    schemas.insert(
        "UpdateRoleRequest".to_string(),
        update_role_request_schema(),
    );
    schemas.insert("GroupResponse".to_string(), group_response_schema());
    schemas.insert(
        "CreateGroupRequest".to_string(),
        create_group_request_schema(),
    );
    schemas.insert(
        "UpdateGroupRequest".to_string(),
        update_group_request_schema(),
    );
    schemas.insert("Policy".to_string(), policy_schema());
    schemas.insert(
        "UpsertPolicyRequest".to_string(),
        upsert_policy_request_schema(),
    );
    schemas.insert(
        "PolicyEvaluationResult".to_string(),
        policy_evaluation_result_schema(),
    );
    schemas.insert(
        "EvaluatePolicyRequest".to_string(),
        evaluate_policy_request_schema(),
    );
    schemas.insert(
        "AppBrandingSettings".to_string(),
        app_branding_settings_schema(),
    );
    schemas.insert(
        "ControlPanelSettings".to_string(),
        control_panel_settings_schema(),
    );
    schemas.insert(
        "UpdateControlPanelRequest".to_string(),
        update_control_panel_request_schema(),
    );
    schemas.insert(
        "AdminUsersListResponse".to_string(),
        list_response_schema("UserResponse"),
    );
    schemas.insert(
        "AdminRolesListResponse".to_string(),
        list_response_schema("RoleResponse"),
    );
    schemas.insert(
        "AdminGroupsListResponse".to_string(),
        list_response_schema("GroupResponse"),
    );
    schemas.insert(
        "AdminPermissionsListResponse".to_string(),
        list_response_schema("Permission"),
    );
    schemas.insert(
        "AdminPoliciesListResponse".to_string(),
        list_response_schema("Policy"),
    );
    schemas.insert(
        "FilesystemBreadcrumb".to_string(),
        filesystem_breadcrumb_schema(),
    );
    schemas.insert("FilesystemEntry".to_string(), filesystem_entry_schema());
    schemas.insert(
        "FilesystemSections".to_string(),
        filesystem_sections_schema(),
    );
    schemas.insert(
        "FilesystemListResponse".to_string(),
        filesystem_list_response_schema(),
    );

    insert_operation(
        spec,
        "/api/v2/admin/users",
        "get",
        manual_operation(
            "List admin users (v2)",
            "rest.admin.v2.listUsers",
            vec!["rest.admin.v2"],
            None,
            "AdminUsersListResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/users/me",
        "get",
        manual_operation(
            "Get current admin user (v2)",
            "rest.admin.v2.getCurrentUser",
            vec!["rest.admin.v2"],
            None,
            "UserResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/users/{id}",
        "patch",
        manual_operation(
            "Update admin user (v2)",
            "rest.admin.v2.updateUser",
            vec!["rest.admin.v2"],
            Some("UpdateUserRequest"),
            "UserResponse",
            vec![path_parameter("id")],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/roles",
        "get",
        manual_operation(
            "List roles (v2)",
            "rest.admin.v2.listRoles",
            vec!["rest.admin.v2"],
            None,
            "AdminRolesListResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/roles",
        "post",
        manual_operation(
            "Create role (v2)",
            "rest.admin.v2.createRole",
            vec!["rest.admin.v2"],
            Some("CreateRoleRequest"),
            "RoleResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/roles/{id}",
        "put",
        manual_operation(
            "Update role (v2)",
            "rest.admin.v2.updateRole",
            vec!["rest.admin.v2"],
            Some("UpdateRoleRequest"),
            "RoleResponse",
            vec![path_parameter("id")],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/groups",
        "get",
        manual_operation(
            "List groups (v2)",
            "rest.admin.v2.listGroups",
            vec!["rest.admin.v2"],
            None,
            "AdminGroupsListResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/groups",
        "post",
        manual_operation(
            "Create group (v2)",
            "rest.admin.v2.createGroup",
            vec!["rest.admin.v2"],
            Some("CreateGroupRequest"),
            "GroupResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/groups/{id}",
        "put",
        manual_operation(
            "Update group (v2)",
            "rest.admin.v2.updateGroup",
            vec!["rest.admin.v2"],
            Some("UpdateGroupRequest"),
            "GroupResponse",
            vec![path_parameter("id")],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/permissions",
        "get",
        manual_operation(
            "List permissions (v2)",
            "rest.admin.v2.listPermissions",
            vec!["rest.admin.v2"],
            None,
            "AdminPermissionsListResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/permissions",
        "post",
        manual_operation(
            "Create permission (v2)",
            "rest.admin.v2.createPermission",
            vec!["rest.admin.v2"],
            Some("CreatePermissionRequest"),
            "Permission",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/policies",
        "get",
        manual_operation(
            "List policies (v2)",
            "rest.admin.v2.listPolicies",
            vec!["rest.admin.v2"],
            None,
            "AdminPoliciesListResponse",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/policies",
        "post",
        manual_operation(
            "Create policy (v2)",
            "rest.admin.v2.createPolicy",
            vec!["rest.admin.v2"],
            Some("UpsertPolicyRequest"),
            "Policy",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/policies/{id}",
        "patch",
        manual_operation(
            "Update policy (v2)",
            "rest.admin.v2.updatePolicy",
            vec!["rest.admin.v2"],
            Some("UpsertPolicyRequest"),
            "Policy",
            vec![path_parameter("id")],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/policies/evaluate",
        "post",
        manual_operation(
            "Evaluate policy (v2)",
            "rest.admin.v2.evaluatePolicy",
            vec!["rest.admin.v2"],
            Some("EvaluatePolicyRequest"),
            "PolicyEvaluationResult",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/control-panel",
        "get",
        manual_operation(
            "Get control panel (v2)",
            "rest.admin.v2.getControlPanel",
            vec!["rest.admin.v2"],
            None,
            "ControlPanelSettings",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/admin/control-panel",
        "put",
        manual_operation(
            "Update control panel (v2)",
            "rest.admin.v2.updateControlPanel",
            vec!["rest.admin.v2"],
            Some("UpdateControlPanelRequest"),
            "ControlPanelSettings",
            vec![],
        ),
    );
    insert_operation(
        spec,
        "/api/v2/filesystem/datasets/{dataset_id}",
        "get",
        manual_operation(
            "List dataset filesystem (v2)",
            "rest.filesystem.v2.getDatasetFilesystem",
            vec!["rest.filesystem.v2"],
            None,
            "FilesystemListResponse",
            vec![path_parameter("dataset_id"), query_parameter("path")],
        ),
    );
}

fn insert_operation(spec: &mut OpenApiSpec, path: &str, method: &str, operation: OpenApiOperation) {
    spec.paths
        .entry(path.to_string())
        .or_default()
        .insert(method.to_string(), operation);
}

fn manual_operation(
    summary: &str,
    operation_id: &str,
    tags: Vec<&str>,
    request_schema: Option<&str>,
    response_schema: &str,
    parameters: Vec<OpenApiParameter>,
) -> OpenApiOperation {
    OpenApiOperation {
        summary: summary.to_string(),
        operation_id: operation_id.to_string(),
        tags: tags.into_iter().map(str::to_string).collect(),
        parameters: if parameters.is_empty() {
            None
        } else {
            Some(parameters)
        },
        request_body: request_schema.map(|name| OpenApiRequestBody {
            required: true,
            content: BTreeMap::from([(
                "application/json".to_string(),
                OpenApiMediaType {
                    schema: schema_ref(name),
                },
            )]),
        }),
        responses: BTreeMap::from([(
            "200".to_string(),
            OpenApiResponse {
                description: "Successful response".to_string(),
                content: BTreeMap::from([(
                    "application/json".to_string(),
                    OpenApiMediaType {
                        schema: schema_ref(response_schema),
                    },
                )]),
            },
        )]),
    }
}

fn path_parameter(name: &str) -> OpenApiParameter {
    OpenApiParameter {
        name: name.to_string(),
        location: "path".to_string(),
        required: true,
        schema: string_schema(None),
    }
}

fn query_parameter(name: &str) -> OpenApiParameter {
    OpenApiParameter {
        name: name.to_string(),
        location: "query".to_string(),
        required: false,
        schema: string_schema(None),
    }
}

fn object_schema(properties: BTreeMap<String, OpenApiSchema>) -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: Some(properties),
        items: None,
        reference: None,
        additional_properties: None,
    }
}

fn object_with_additional_properties(value_type: OpenApiSchema) -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("object".to_string()),
        format: None,
        properties: None,
        items: None,
        reference: None,
        additional_properties: Some(Box::new(value_type)),
    }
}

fn string_schema(format: Option<&str>) -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("string".to_string()),
        format: format.map(str::to_string),
        properties: None,
        items: None,
        reference: None,
        additional_properties: None,
    }
}

fn integer_schema() -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("integer".to_string()),
        format: Some("int64".to_string()),
        properties: None,
        items: None,
        reference: None,
        additional_properties: None,
    }
}

fn boolean_schema() -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("boolean".to_string()),
        format: None,
        properties: None,
        items: None,
        reference: None,
        additional_properties: None,
    }
}

fn array_schema(item_schema: OpenApiSchema) -> OpenApiSchema {
    OpenApiSchema {
        schema_type: Some("array".to_string()),
        format: None,
        properties: None,
        items: Some(Box::new(item_schema)),
        reference: None,
        additional_properties: None,
    }
}

fn any_value_schema() -> OpenApiSchema {
    object_with_additional_properties(OpenApiSchema {
        schema_type: None,
        format: None,
        properties: None,
        items: None,
        reference: None,
        additional_properties: None,
    })
}

fn list_response_schema(item_schema_name: &str) -> OpenApiSchema {
    object_schema(BTreeMap::from([
        (
            "items".to_string(),
            array_schema(schema_ref(item_schema_name)),
        ),
        ("count".to_string(), integer_schema()),
    ]))
}

fn user_response_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("id".to_string(), string_schema(Some("uuid"))),
        ("email".to_string(), string_schema(None)),
        ("name".to_string(), string_schema(None)),
        ("is_active".to_string(), boolean_schema()),
        ("roles".to_string(), array_schema(string_schema(None))),
        ("groups".to_string(), array_schema(string_schema(None))),
        ("permissions".to_string(), array_schema(string_schema(None))),
        ("organization_id".to_string(), string_schema(Some("uuid"))),
        ("attributes".to_string(), any_value_schema()),
        ("mfa_enabled".to_string(), boolean_schema()),
        ("mfa_enforced".to_string(), boolean_schema()),
        ("auth_source".to_string(), string_schema(None)),
        ("created_at".to_string(), string_schema(Some("date-time"))),
    ]))
}

fn update_user_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("name".to_string(), string_schema(None)),
        ("organization_id".to_string(), string_schema(Some("uuid"))),
        ("attributes".to_string(), any_value_schema()),
        ("mfa_enforced".to_string(), boolean_schema()),
        ("is_active".to_string(), boolean_schema()),
    ]))
}

fn permission_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("id".to_string(), string_schema(Some("uuid"))),
        ("resource".to_string(), string_schema(None)),
        ("action".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        ("created_at".to_string(), string_schema(Some("date-time"))),
    ]))
}

fn create_permission_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("resource".to_string(), string_schema(None)),
        ("action".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
    ]))
}

fn role_response_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("id".to_string(), string_schema(Some("uuid"))),
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        ("created_at".to_string(), string_schema(Some("date-time"))),
        (
            "permission_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
        ("permissions".to_string(), array_schema(string_schema(None))),
    ]))
}

fn create_role_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        (
            "permission_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
    ]))
}

fn update_role_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("description".to_string(), string_schema(None)),
        (
            "permission_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
    ]))
}

fn group_response_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("id".to_string(), string_schema(Some("uuid"))),
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        ("created_at".to_string(), string_schema(Some("date-time"))),
        ("member_count".to_string(), integer_schema()),
        (
            "role_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
        ("roles".to_string(), array_schema(string_schema(None))),
    ]))
}

fn create_group_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        (
            "role_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
    ]))
}

fn update_group_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("description".to_string(), string_schema(None)),
        (
            "role_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
    ]))
}

fn policy_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("id".to_string(), string_schema(Some("uuid"))),
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        ("effect".to_string(), string_schema(None)),
        ("resource".to_string(), string_schema(None)),
        ("action".to_string(), string_schema(None)),
        ("conditions".to_string(), any_value_schema()),
        ("row_filter".to_string(), string_schema(None)),
        ("enabled".to_string(), boolean_schema()),
        ("created_by".to_string(), string_schema(Some("uuid"))),
        ("created_at".to_string(), string_schema(Some("date-time"))),
        ("updated_at".to_string(), string_schema(Some("date-time"))),
    ]))
}

fn upsert_policy_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("name".to_string(), string_schema(None)),
        ("description".to_string(), string_schema(None)),
        ("effect".to_string(), string_schema(None)),
        ("resource".to_string(), string_schema(None)),
        ("action".to_string(), string_schema(None)),
        ("conditions".to_string(), any_value_schema()),
        ("row_filter".to_string(), string_schema(None)),
        ("enabled".to_string(), boolean_schema()),
    ]))
}

fn policy_evaluation_result_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("allowed".to_string(), boolean_schema()),
        (
            "matched_policy_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
        (
            "deny_policy_ids".to_string(),
            array_schema(string_schema(Some("uuid"))),
        ),
        ("row_filter".to_string(), string_schema(None)),
    ]))
}

fn evaluate_policy_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("resource".to_string(), string_schema(None)),
        ("action".to_string(), string_schema(None)),
        ("resource_attributes".to_string(), any_value_schema()),
    ]))
}

fn app_branding_settings_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("display_name".to_string(), string_schema(None)),
        ("primary_color".to_string(), string_schema(None)),
        ("accent_color".to_string(), string_schema(None)),
        ("logo_url".to_string(), string_schema(None)),
        ("favicon_url".to_string(), string_schema(None)),
        ("show_powered_by".to_string(), boolean_schema()),
    ]))
}

fn control_panel_settings_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("platform_name".to_string(), string_schema(None)),
        ("support_email".to_string(), string_schema(None)),
        ("docs_url".to_string(), string_schema(None)),
        ("status_page_url".to_string(), string_schema(None)),
        ("announcement_banner".to_string(), string_schema(None)),
        ("maintenance_mode".to_string(), boolean_schema()),
        ("release_channel".to_string(), string_schema(None)),
        ("default_region".to_string(), string_schema(None)),
        ("deployment_mode".to_string(), string_schema(None)),
        ("allow_self_signup".to_string(), boolean_schema()),
        (
            "allowed_email_domains".to_string(),
            array_schema(string_schema(None)),
        ),
        (
            "default_app_branding".to_string(),
            schema_ref("AppBrandingSettings"),
        ),
        (
            "restricted_operations".to_string(),
            array_schema(string_schema(None)),
        ),
        ("updated_by".to_string(), string_schema(Some("uuid"))),
        ("updated_at".to_string(), string_schema(Some("date-time"))),
    ]))
}

fn update_control_panel_request_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("platform_name".to_string(), string_schema(None)),
        ("support_email".to_string(), string_schema(None)),
        ("docs_url".to_string(), string_schema(None)),
        ("status_page_url".to_string(), string_schema(None)),
        ("announcement_banner".to_string(), string_schema(None)),
        ("maintenance_mode".to_string(), boolean_schema()),
        ("release_channel".to_string(), string_schema(None)),
        ("default_region".to_string(), string_schema(None)),
        ("deployment_mode".to_string(), string_schema(None)),
        ("allow_self_signup".to_string(), boolean_schema()),
        (
            "allowed_email_domains".to_string(),
            array_schema(string_schema(None)),
        ),
        (
            "default_app_branding".to_string(),
            schema_ref("AppBrandingSettings"),
        ),
        (
            "restricted_operations".to_string(),
            array_schema(string_schema(None)),
        ),
    ]))
}

fn filesystem_breadcrumb_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("name".to_string(), string_schema(None)),
        ("path".to_string(), string_schema(None)),
    ]))
}

fn filesystem_entry_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("entry_type".to_string(), string_schema(None)),
        ("name".to_string(), string_schema(None)),
        ("path".to_string(), string_schema(None)),
        ("storage_path".to_string(), string_schema(None)),
        ("size_bytes".to_string(), integer_schema()),
        (
            "last_modified".to_string(),
            string_schema(Some("date-time")),
        ),
        ("content_type".to_string(), string_schema(None)),
        ("metadata".to_string(), any_value_schema()),
    ]))
}

fn filesystem_sections_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("versions".to_string(), integer_schema()),
        ("branches".to_string(), integer_schema()),
        ("views".to_string(), integer_schema()),
    ]))
}

fn filesystem_list_response_schema() -> OpenApiSchema {
    object_schema(BTreeMap::from([
        ("dataset_id".to_string(), string_schema(Some("uuid"))),
        ("requested_path".to_string(), string_schema(None)),
        ("root".to_string(), string_schema(None)),
        ("current_version".to_string(), integer_schema()),
        ("active_branch".to_string(), string_schema(None)),
        (
            "entries".to_string(),
            array_schema(schema_ref("FilesystemEntry")),
        ),
        (
            "items".to_string(),
            array_schema(schema_ref("FilesystemEntry")),
        ),
        (
            "breadcrumbs".to_string(),
            array_schema(schema_ref("FilesystemBreadcrumb")),
        ),
        ("sections".to_string(), schema_ref("FilesystemSections")),
    ]))
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
