pub mod llm;
pub mod python;
pub mod r;
pub mod sql;

use std::{collections::HashMap, sync::Arc};

use auth_middleware::{Claims, JwtConfig};
use reqwest::Client;
use serde_json::Value;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct KernelExecutionResult {
    pub output_type: String,
    pub content: Value,
}

#[derive(Debug, Clone)]
pub struct KernelWorkspaceFileContext {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct KernelExecutionContext {
    pub notebook_id: Uuid,
    pub workspace_dir: Option<String>,
    pub workspace_files: Vec<KernelWorkspaceFileContext>,
}

/// Manages kernel dispatch for cell execution.
#[derive(Clone)]
pub struct KernelManager {
    pub jwt_config: JwtConfig,
    pub query_service_url: String,
    pub ai_service_url: String,
    pub http_client: Client,
    pub python_sessions: python::PythonSessions,
    pub llm_sessions: llm::LlmSessions,
}

impl KernelManager {
    pub fn new(jwt_config: JwtConfig, query_service_url: String, ai_service_url: String) -> Self {
        Self {
            jwt_config,
            query_service_url,
            ai_service_url,
            http_client: Client::new(),
            python_sessions: Arc::new(RwLock::new(HashMap::new())),
            llm_sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn ensure_session(&self, session_id: Uuid, kernel: &str) -> Result<(), String> {
        match kernel {
            "python" => python::ensure_session(&self.python_sessions, session_id).await,
            "llm" => llm::ensure_session(&self.llm_sessions, session_id).await,
            "r" => Ok(()),
            "sql" => Ok(()),
            other => Err(format!("unsupported kernel: {other}")),
        }
    }

    pub async fn drop_session(&self, session_id: Uuid) {
        python::drop_session(&self.python_sessions, session_id).await;
        llm::drop_session(&self.llm_sessions, session_id).await;
    }

    pub async fn execute(
        &self,
        kernel: &str,
        source: &str,
        session_id: Option<Uuid>,
        claims: &Claims,
        context: &KernelExecutionContext,
    ) -> Result<KernelExecutionResult, String> {
        match kernel {
            "python" => {
                python::execute(
                    &self.python_sessions,
                    session_id,
                    source,
                    context.workspace_dir.as_deref(),
                    context.notebook_id,
                )
                .await
            }
            "llm" => {
                llm::execute(
                    &self.llm_sessions,
                    &self.http_client,
                    &self.ai_service_url,
                    &self.jwt_config,
                    claims,
                    source,
                    session_id,
                    context,
                )
                .await
            }
            "r" => r::execute(source, context.workspace_dir.as_deref()).await,
            "sql" => {
                sql::execute(
                    &self.http_client,
                    &self.query_service_url,
                    &self.jwt_config,
                    claims,
                    source,
                )
                .await
            }
            other => Err(format!("unsupported kernel: {other}")),
        }
    }
}
