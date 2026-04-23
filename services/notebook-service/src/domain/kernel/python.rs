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
) -> Result<KernelExecutionResult, String> {
    let locals = if let Some(session_id) = session_id {
        ensure_session(sessions, session_id).await?;
        sessions.read().await.get(&session_id).cloned()
    } else {
        None
    };

    run_python(source, locals)
}

fn run_python(source: &str, locals: Option<Arc<Py<PyDict>>>) -> Result<KernelExecutionResult, String> {
    Python::with_gil(|py| {
        let locals = locals.unwrap_or_else(|| Arc::new(PyDict::new_bound(py).unbind()));
        let locals = locals.as_ref().bind(py);

        py.run_bound("import io, sys", None, Some(&locals))
            .map_err(|e| format!("setup error: {e}"))?;
        py.run_bound(
            "_buf = io.StringIO()\n_real_stdout = sys.stdout\nsys.stdout = _buf",
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
