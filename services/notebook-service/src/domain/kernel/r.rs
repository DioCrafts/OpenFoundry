use tokio::process::Command;

use crate::domain::kernel::KernelExecutionResult;

pub async fn execute(
    source: &str,
    workspace_dir: Option<&str>,
) -> Result<KernelExecutionResult, String> {
    let mut command = Command::new("Rscript");
    command.arg("-e");
    command.arg(build_script(source, workspace_dir));

    if let Some(dir) = workspace_dir {
        command.current_dir(dir);
    }

    let output = command
        .output()
        .await
        .map_err(|error| format!("failed to start Rscript: {error}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(KernelExecutionResult {
            output_type: "text".to_string(),
            content: serde_json::json!(stdout),
        })
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(if stderr.trim().is_empty() {
            "R execution failed".to_string()
        } else {
            stderr
        })
    }
}

fn build_script(source: &str, workspace_dir: Option<&str>) -> String {
    let workspace_dir = workspace_dir
        .unwrap_or("")
        .replace('\\', "/")
        .replace('\'', "\\'");

    format!(
        "workspace_dir <- '{}'\nif (nzchar(workspace_dir)) {{ setwd(workspace_dir) }}\n{}\n",
        workspace_dir, source
    )
}
