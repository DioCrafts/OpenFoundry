use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

pub const WASM_MANIFEST_EXPORT: &str = "open_foundry_manifest";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    Connector,
    Transform,
    Widget,
}

impl PluginKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Connector => "connector",
            Self::Transform => "transform",
            Self::Widget => "widget",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Runtime {
    RustWasm32Wasi,
    RustNative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub kind: PluginKind,
    pub runtime: Runtime,
    pub entrypoint: String,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub config_schema: Value,
    #[serde(default)]
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub ok: bool,
    pub message: String,
    #[serde(default)]
    pub diagnostics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformResult {
    #[serde(default)]
    pub output: Value,
    #[serde(default)]
    pub metrics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetRenderResult {
    pub component_name: String,
    #[serde(default)]
    pub props: Value,
    #[serde(default)]
    pub assets: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("manifest serialization error: {0}")]
    ManifestJson(#[from] serde_json::Error),
    #[error("plugin kind mismatch: expected {expected}, got {actual}")]
    KindMismatch {
        expected: &'static str,
        actual: &'static str,
    },
}

pub trait ConnectorPlugin {
    fn manifest(&self) -> PluginManifest;
    fn test_connection(&self, config: Value) -> Result<ConnectionTestResult, String>;
}

pub trait TransformPlugin {
    fn manifest(&self) -> PluginManifest;
    fn transform(&self, input: Value, config: Value) -> Result<TransformResult, String>;
}

pub trait WidgetPlugin {
    fn manifest(&self) -> PluginManifest;
    fn render(&self, props: Value) -> Result<WidgetRenderResult, String>;
}

pub fn parse_manifest(manifest_json: &str) -> Result<PluginManifest, PluginError> {
    Ok(serde_json::from_str(manifest_json)?)
}

pub fn validate_kind(manifest: &PluginManifest, expected: PluginKind) -> Result<(), PluginError> {
    if manifest.kind == expected {
        Ok(())
    } else {
        Err(PluginError::KindMismatch {
            expected: expected.as_str(),
            actual: manifest.kind.as_str(),
        })
    }
}

pub fn manifest_json(manifest: &PluginManifest) -> Result<String, PluginError> {
    Ok(serde_json::to_string_pretty(manifest)?)
}

pub mod scaffold {
    use super::{PluginKind, Runtime};

    pub fn cargo_toml(crate_name: &str, kind: PluginKind) -> String {
        format!(
            "[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\ncrate-type = [\"cdylib\", \"rlib\"]\n\n[dependencies]\nplugin-sdk = {{ path = \"../../libs/plugin-sdk\" }}\nserde_json = \"1\"\n\n[package.metadata.openfoundry]\nplugin_kind = \"{}\"\nruntime = \"rust_wasm32_wasi\"\n",
            kind.as_str()
        )
    }

    pub fn manifest_json(plugin_name: &str, kind: PluginKind) -> String {
        format!(
            "{{\n  \"name\": \"{plugin_name}\",\n  \"version\": \"0.1.0\",\n  \"kind\": \"{}\",\n  \"runtime\": \"{}\",\n  \"entrypoint\": \"{}\",\n  \"capabilities\": [\"preview\", \"validate\"],\n  \"config_schema\": {{\n    \"type\": \"object\",\n    \"properties\": {{\n      \"endpoint\": {{ \"type\": \"string\" }}\n    }}\n  }},\n  \"metadata\": {{\n    \"author\": \"OpenFoundry Developer\"\n  }}\n}}\n",
            kind.as_str(),
            match Runtime::RustWasm32Wasi {
                Runtime::RustWasm32Wasi => "rust_wasm32_wasi",
                Runtime::RustNative => "rust_native",
            },
            plugin_name.replace('-', "_")
        )
    }

    pub fn lib_rs(plugin_name: &str, kind: PluginKind) -> String {
        let manifest = format!(
            "PluginManifest {{\n        name: \"{plugin_name}\".to_string(),\n        version: \"0.1.0\".to_string(),\n        kind: PluginKind::{},\n        runtime: Runtime::RustWasm32Wasi,\n        entrypoint: \"{}\".to_string(),\n        capabilities: vec![\"preview\".to_string(), \"validate\".to_string()],\n        config_schema: serde_json::json!({{\"type\": \"object\"}}),\n        metadata: serde_json::json!({{\"generated_by\": \"of project init\"}}),\n    }}",
            match kind {
                PluginKind::Connector => "Connector",
                PluginKind::Transform => "Transform",
                PluginKind::Widget => "Widget",
            },
            plugin_name.replace('-', "_")
        );

        match kind {
            PluginKind::Connector => format!(
                "use plugin_sdk::{{ConnectionTestResult, ConnectorPlugin, PluginKind, PluginManifest, Runtime}};\nuse serde_json::Value;\n\npub struct Plugin;\n\nimpl ConnectorPlugin for Plugin {{\n    fn manifest(&self) -> PluginManifest {{\n        {manifest}\n    }}\n\n    fn test_connection(&self, config: Value) -> Result<ConnectionTestResult, String> {{\n        Ok(ConnectionTestResult {{\n            ok: true,\n            message: format!(\"connector ready: {{}}\", config),\n            diagnostics: serde_json::json!({{\"status\": \"ok\"}}),\n        }})\n    }}\n}}\n"
            ),
            PluginKind::Transform => format!(
                "use plugin_sdk::{{PluginKind, PluginManifest, Runtime, TransformPlugin, TransformResult}};\nuse serde_json::Value;\n\npub struct Plugin;\n\nimpl TransformPlugin for Plugin {{\n    fn manifest(&self) -> PluginManifest {{\n        {manifest}\n    }}\n\n    fn transform(&self, input: Value, _config: Value) -> Result<TransformResult, String> {{\n        Ok(TransformResult {{\n            output: input,\n            metrics: serde_json::json!({{\"transformed\": true}}),\n        }})\n    }}\n}}\n"
            ),
            PluginKind::Widget => format!(
                "use plugin_sdk::{{PluginKind, PluginManifest, Runtime, WidgetPlugin, WidgetRenderResult}};\nuse serde_json::Value;\n\npub struct Plugin;\n\nimpl WidgetPlugin for Plugin {{\n    fn manifest(&self) -> PluginManifest {{\n        {manifest}\n    }}\n\n    fn render(&self, props: Value) -> Result<WidgetRenderResult, String> {{\n        Ok(WidgetRenderResult {{\n            component_name: \"GeneratedWidget\".to_string(),\n            props,\n            assets: vec![\"dist/widget.js\".to_string()],\n        }})\n    }}\n}}\n"
            ),
        }
    }
}
