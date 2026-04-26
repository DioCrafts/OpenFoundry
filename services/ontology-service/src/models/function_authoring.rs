use serde::{Deserialize, Serialize};

use super::function_package::FunctionCapabilities;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAuthoringTemplate {
    pub id: String,
    pub runtime: String,
    pub display_name: String,
    pub description: String,
    pub entrypoint: String,
    pub starter_source: String,
    pub default_capabilities: FunctionCapabilities,
    pub recommended_use_cases: Vec<String>,
    pub cli_scaffold_template: Option<String>,
    pub sdk_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSdkPackageReference {
    pub language: String,
    pub path: String,
    pub package_name: String,
    pub generated_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAuthoringSurfaceResponse {
    pub templates: Vec<FunctionAuthoringTemplate>,
    pub sdk_packages: Vec<FunctionSdkPackageReference>,
    pub cli_commands: Vec<String>,
}
