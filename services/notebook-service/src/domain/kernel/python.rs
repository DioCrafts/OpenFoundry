use std::{collections::HashMap, sync::Arc};

use pyo3::prelude::*;
use pyo3::types::PyDict;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::domain::kernel::KernelExecutionResult;

pub type PythonSessions = Arc<RwLock<HashMap<Uuid, Arc<Py<PyDict>>>>>;

pub async fn ensure_session(sessions: &PythonSessions, session_id: Uuid) -> Result<(), String> {
    let mut sessions = sessions.write().await;
    sessions
        .entry(session_id)
        .or_insert_with(|| Arc::new(Python::with_gil(|py| PyDict::new_bound(py).unbind())));
    Ok(())
}

pub async fn drop_session(sessions: &PythonSessions, session_id: Uuid) {
    sessions.write().await.remove(&session_id);
}

pub async fn execute(
    sessions: &PythonSessions,
    session_id: Option<Uuid>,
    source: &str,
    workspace_dir: Option<&str>,
    notebook_id: Uuid,
) -> Result<KernelExecutionResult, String> {
    let locals = if let Some(session_id) = session_id {
        ensure_session(sessions, session_id).await?;
        sessions.read().await.get(&session_id).cloned()
    } else {
        None
    };

    run_python(source, locals, workspace_dir, notebook_id)
}

fn run_python(
    source: &str,
    locals: Option<Arc<Py<PyDict>>>,
    workspace_dir: Option<&str>,
    notebook_id: Uuid,
) -> Result<KernelExecutionResult, String> {
    Python::with_gil(|py| {
        let locals = locals.unwrap_or_else(|| Arc::new(PyDict::new_bound(py).unbind()));
        let locals = locals.as_ref().bind(py);
        let workspace_literal = serde_json::to_string(workspace_dir.unwrap_or(""))
            .map_err(|error| format!("workspace serialization error: {error}"))?;
        let notebook_id_literal = serde_json::to_string(&notebook_id.to_string())
            .map_err(|error| format!("notebook id serialization error: {error}"))?;

        py.run_bound("import io, sys, os, pathlib", None, Some(&locals))
            .map_err(|e| format!("setup error: {e}"))?;
        py.run_bound(
            &format!(
                "_buf = io.StringIO()\n_real_stdout = sys.stdout\nsys.stdout = _buf\nworkspace_dir = {workspace}\nnotebook_id = {notebook_id}\n\
def workspace_path(*parts):\n    base = pathlib.Path(workspace_dir) if workspace_dir else pathlib.Path.cwd()\n    return str(base.joinpath(*parts))\n\
os.environ['OPENFOUNDRY_NOTEBOOK_ID'] = notebook_id\n\
os.environ['OPENFOUNDRY_NOTEBOOK_WORKSPACE'] = workspace_dir",
                workspace = workspace_literal,
                notebook_id = notebook_id_literal,
            ),
            None,
            Some(&locals),
        )
        .map_err(|e| format!("stdout capture setup error: {e}"))?;

        let execution = py.run_bound(source, None, Some(&locals));
        let output = py
            .eval_bound("_buf.getvalue()", None, Some(&locals))
            .ok()
            .and_then(|value| value.extract::<String>().ok())
            .unwrap_or_default();

        let _ = py.run_bound("sys.stdout = _real_stdout", None, Some(&locals));

        match execution {
            Ok(_) => Ok(KernelExecutionResult {
                output_type: "text".into(),
                content: serde_json::json!(output),
            }),
            Err(error) => Err(format!("{error}")),
        }
    })
}
