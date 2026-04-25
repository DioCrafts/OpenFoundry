mod benchmark;
mod mock_provider;
mod openapi;
mod smoke;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use plugin_sdk::{PluginKind, scaffold};
use serde::Serialize;

#[derive(Parser)]
#[command(name = "of")]
#[command(about = "OpenFoundry CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    Deploy {
        #[command(subcommand)]
        command: DeployCommand,
    },
    Script {
        #[command(subcommand)]
        command: ScriptCommand,
    },
    Docs {
        #[command(subcommand)]
        command: DocsCommand,
    },
    Bench {
        #[command(subcommand)]
        command: BenchCommand,
    },
    Smoke {
        #[command(subcommand)]
        command: SmokeCommand,
    },
    MockProvider {
        #[command(subcommand)]
        command: MockProviderCommand,
    },
    Terraform {
        #[command(subcommand)]
        command: TerraformCommand,
    },
}

#[derive(Subcommand)]
enum ProjectCommand {
    Init {
        name: String,
        #[arg(long, value_enum, default_value_t = ProjectTemplate::Connector)]
        template: ProjectTemplate,
        #[arg(long, default_value = ".")]
        output: PathBuf,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum ProjectTemplate {
    Connector,
    Transform,
    Widget,
}

#[derive(Subcommand)]
enum DeployCommand {
    Plan {
        service: String,
        #[arg(long, default_value = "dev")]
        environment: String,
    },
}

#[derive(Subcommand)]
enum ScriptCommand {
    Render {
        template: String,
        #[arg(long = "var")]
        vars: Vec<String>,
    },
}

#[derive(Subcommand)]
enum DocsCommand {
    GenerateOpenapi {
        #[arg(long, default_value = "proto")]
        proto_dir: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidateOpenapi {
        #[arg(long, default_value = "proto")]
        proto_dir: PathBuf,
        #[arg(long)]
        input: PathBuf,
    },
    GenerateSdkTypescript {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidateSdkTypescript {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    GenerateSdkPython {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidateSdkPython {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    GenerateSdkJava {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    ValidateSdkJava {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum BenchCommand {
    Run {
        #[arg(long, default_value = "benchmarks/scenarios/critical-paths.json")]
        scenario: PathBuf,
        #[arg(long, default_value = "benchmarks/results/critical-paths.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum SmokeCommand {
    Run {
        #[arg(long, default_value = "smoke/scenarios/p2-runtime-critical-path.json")]
        scenario: PathBuf,
        #[arg(long, default_value = "smoke/results/p2-runtime-critical-path.json")]
        output: PathBuf,
    },
}

#[derive(Subcommand)]
enum MockProviderCommand {
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value_t = 50110)]
        port: u16,
    },
}

#[derive(Subcommand)]
enum TerraformCommand {
    Schema {
        #[arg(long)]
        output: PathBuf,
    },
}

#[derive(Debug, Serialize)]
struct DeployPlan<'a> {
    service: &'a str,
    environment: &'a str,
    steps: Vec<&'a str>,
    artifacts: Vec<String>,
    verification: Vec<&'a str>,
}

#[derive(Debug, Serialize)]
struct TerraformSchema {
    provider: ProviderDefinition,
    resources: Vec<ResourceDefinition>,
    data_sources: Vec<DataSourceDefinition>,
}

#[derive(Debug, Serialize)]
struct ProviderDefinition {
    name: &'static str,
    version: &'static str,
    configuration: BTreeMap<&'static str, &'static str>,
}

#[derive(Debug, Serialize)]
struct ResourceDefinition {
    name: &'static str,
    description: &'static str,
    attributes: BTreeMap<&'static str, &'static str>,
}

#[derive(Debug, Serialize)]
struct DataSourceDefinition {
    name: &'static str,
    description: &'static str,
    attributes: BTreeMap<&'static str, &'static str>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Project { command } => match command {
            ProjectCommand::Init {
                name,
                template,
                output,
            } => init_project(&name, template, &output),
        },
        Command::Deploy { command } => match command {
            DeployCommand::Plan {
                service,
                environment,
            } => print_json(&build_deploy_plan(&service, &environment)),
        },
        Command::Script { command } => match command {
            ScriptCommand::Render { template, vars } => render_script(&template, &vars),
        },
        Command::Docs { command } => match command {
            DocsCommand::GenerateOpenapi { proto_dir, output } => {
                generate_openapi(&proto_dir, &output)
            }
            DocsCommand::ValidateOpenapi { proto_dir, input } => {
                validate_openapi(&proto_dir, &input)
            }
            DocsCommand::GenerateSdkTypescript { input, output } => {
                generate_typescript_sdk(&input, &output)
            }
            DocsCommand::ValidateSdkTypescript { input, output } => {
                validate_typescript_sdk(&input, &output)
            }
            DocsCommand::GenerateSdkPython { input, output } => {
                generate_python_sdk(&input, &output)
            }
            DocsCommand::ValidateSdkPython { input, output } => {
                validate_python_sdk(&input, &output)
            }
            DocsCommand::GenerateSdkJava { input, output } => {
                generate_java_sdk(&input, &output)
            }
            DocsCommand::ValidateSdkJava { input, output } => {
                validate_java_sdk(&input, &output)
            }
        },
        Command::Bench { command } => match command {
            BenchCommand::Run { scenario, output } => {
                benchmark::run_suite(&scenario, &output).await
            }
        },
        Command::Smoke { command } => match command {
            SmokeCommand::Run { scenario, output } => smoke::run_suite(&scenario, &output).await,
        },
        Command::MockProvider { command } => match command {
            MockProviderCommand::Serve { host, port } => mock_provider::serve(&host, port).await,
        },
        Command::Terraform { command } => match command {
            TerraformCommand::Schema { output } => generate_terraform_schema(&output),
        },
    }
}

fn init_project(name: &str, template: ProjectTemplate, output_root: &Path) -> Result<()> {
    let kind = match template {
        ProjectTemplate::Connector => PluginKind::Connector,
        ProjectTemplate::Transform => PluginKind::Transform,
        ProjectTemplate::Widget => PluginKind::Widget,
    };
    let project_dir = output_root.join(name);
    if project_dir.exists() {
        bail!("output directory already exists: {}", project_dir.display());
    }

    fs::create_dir_all(project_dir.join("src"))
        .with_context(|| format!("failed to create {}", project_dir.display()))?;
    fs::write(
        project_dir.join("Cargo.toml"),
        scaffold::cargo_toml(name, kind),
    )?;
    fs::write(
        project_dir.join("plugin.json"),
        scaffold::manifest_json(name, kind),
    )?;
    fs::write(project_dir.join("src/lib.rs"), scaffold::lib_rs(name, kind))?;

    println!(
        "scaffolded {} plugin at {}",
        kind.as_str(),
        project_dir.display()
    );
    Ok(())
}

fn build_deploy_plan<'a>(service: &'a str, environment: &'a str) -> DeployPlan<'a> {
    DeployPlan {
        service,
        environment,
        steps: vec![
            "validate configuration",
            "render infrastructure inputs",
            "apply rollout strategy",
            "verify health and smoke checks",
        ],
        artifacts: vec![
            format!("infra/terraform/environments/{environment}"),
            format!("services/{service}"),
            format!("release/{service}:{environment}"),
        ],
        verification: vec![
            "health endpoint",
            "primary workflow smoke test",
            "post-deploy audit event",
        ],
    }
}

fn render_script(template: &str, vars: &[String]) -> Result<()> {
    let mut rendered = template.to_string();
    for entry in vars {
        let (key, value) = entry
            .split_once('=')
            .context("vars must use key=value format")?;
        rendered = rendered.replace(&format!("{{{{{key}}}}}"), value);
    }
    println!("{rendered}");
    Ok(())
}

fn generate_openapi(proto_dir: &Path, output: &Path) -> Result<()> {
    let spec = openapi::generate_spec(proto_dir)?;
    write_json(output, &spec)?;
    println!("generated OpenAPI spec at {}", output.display());
    Ok(())
}

fn validate_openapi(proto_dir: &Path, input: &Path) -> Result<()> {
    openapi::validate_generated_spec(proto_dir, input)?;
    println!("validated OpenAPI spec at {}", input.display());
    Ok(())
}

fn generate_typescript_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::generate_typescript_sdk(input, output)?;
    println!("generated TypeScript SDK at {}", output.display());
    Ok(())
}

fn validate_typescript_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::validate_typescript_sdk(input, output)?;
    println!("validated TypeScript SDK at {}", output.display());
    Ok(())
}

fn generate_python_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::generate_python_sdk(input, output)?;
    println!("generated Python SDK at {}", output.display());
    Ok(())
}

fn validate_python_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::validate_python_sdk(input, output)?;
    println!("validated Python SDK at {}", output.display());
    Ok(())
}

fn generate_java_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::generate_java_sdk(input, output)?;
    println!("generated Java SDK at {}", output.display());
    Ok(())
}

fn validate_java_sdk(input: &Path, output: &Path) -> Result<()> {
    openapi::validate_java_sdk(input, output)?;
    println!("validated Java SDK at {}", output.display());
    Ok(())
}

fn generate_terraform_schema(output: &Path) -> Result<()> {
    let mut provider_config = BTreeMap::new();
    provider_config.insert("api_url", "Base URL for the OpenFoundry gateway.");
    provider_config.insert(
        "token",
        "Bearer token used for authenticated API operations.",
    );
    provider_config.insert("workspace", "Logical workspace or environment name.");

    let schema = TerraformSchema {
        provider: ProviderDefinition {
            name: "openfoundry",
            version: "0.1.0",
            configuration: provider_config,
        },
        resources: vec![
            ResourceDefinition {
                name: "openfoundry_repository_integration",
                description: "Manage GitHub or GitLab sync configuration for an OpenFoundry repository.",
                attributes: BTreeMap::from([
                    (
                        "repository_id",
                        "UUID of the repository managed by code-repo-service.",
                    ),
                    ("provider", "git provider: github or gitlab."),
                    ("external_project", "Remote project or repository slug."),
                    (
                        "sync_mode",
                        "push_mirror, bidirectional_mirror, or query_only.",
                    ),
                    (
                        "ci_trigger_strategy",
                        "github_actions, gitlab_ci, or webhook.",
                    ),
                ]),
            },
            ResourceDefinition {
                name: "openfoundry_audit_policy",
                description: "Manage retention and purge policies through audit-service.",
                attributes: BTreeMap::from([
                    ("name", "Human-friendly policy name."),
                    ("classification", "public, confidential, or pii."),
                    ("retention_days", "Retention TTL for matching audit events."),
                    ("purge_mode", "redaction or hard delete posture."),
                ]),
            },
            ResourceDefinition {
                name: "openfoundry_nexus_peer",
                description: "Register and authenticate a cross-organization peer in nexus-service.",
                attributes: BTreeMap::from([
                    ("slug", "Stable peer identifier."),
                    ("endpoint_url", "Partner API endpoint."),
                    ("auth_mode", "mtls+jwt, oidc+mtls, or custom auth profile."),
                    (
                        "shared_scopes",
                        "Scopes allowed for federation and sharing.",
                    ),
                ]),
            },
        ],
        data_sources: vec![DataSourceDefinition {
            name: "openfoundry_openapi_spec",
            description: "Expose the proto-derived OpenAPI contract for downstream tooling.",
            attributes: BTreeMap::from([
                ("path_count", "Number of available API operations."),
                ("spec_json", "Serialized OpenAPI document."),
            ]),
        }],
    };

    write_json(output, &schema)?;
    println!("generated Terraform schema at {}", output.display());
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(value)?)?;
    Ok(())
}
