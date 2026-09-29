use std::process::{Command, Output};
use std::{env, io};

pub fn herdr_bin() -> String {
    env::var("HERDR_BIN_PATH").unwrap_or_else(|_| "herdr".to_string())
}

pub fn run_herdr(args: &[&str]) -> io::Result<Output> {
    Command::new(herdr_bin()).args(args).output()
}

pub fn run_herdr_json(args: &[&str]) -> io::Result<serde_json::Value> {
    let output = run_herdr(args)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let trimmed = stdout.trim();
    if !trimmed.is_empty() {
        return serde_json::from_str(trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSON from herdr {:?}: {e}; stderr={stderr}", args),
            )
        });
    }
    let err_trimmed = stderr.trim();
    if !err_trimmed.is_empty() {
        return serde_json::from_str(err_trimmed).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("invalid JSON from herdr {:?}: {e}", args),
            )
        });
    }
    // pane send-text / send-keys succeed with empty stdout+stderr
    if output.status.success() {
        return Ok(serde_json::json!({"result": {"type": "ok"}}));
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("empty response from herdr {:?}", args),
    ))
}

pub fn cli_error(value: &serde_json::Value) -> Option<String> {
    value
        .get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(str::to_string)
}

pub fn pane_exists(pane_id: &str) -> io::Result<bool> {
    let value = run_herdr_json(&["pane", "get", pane_id])?;
    if cli_error(&value).is_some() {
        return Ok(false);
    }
    Ok(value.get("result").is_some())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneInfo {
    pub pane_id: String,
    pub agent: Option<String>,
    pub agent_status: Option<String>,
}

pub fn pane_get(pane_id: &str) -> io::Result<Option<PaneInfo>> {
    let value = run_herdr_json(&["pane", "get", pane_id])?;
    if cli_error(&value).is_some() {
        return Ok(None);
    }
    let pane = value
        .get("result")
        .and_then(|r| r.get("pane"))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing pane"))?;
    Ok(Some(PaneInfo {
        pane_id: pane
            .get("pane_id")
            .and_then(|v| v.as_str())
            .unwrap_or(pane_id)
            .to_string(),
        agent: pane.get("agent").and_then(|v| v.as_str()).map(str::to_string),
        agent_status: pane
            .get("agent_status")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    }))
}

pub fn is_agent_target(info: &PaneInfo) -> bool {
    info.agent.as_ref().is_some_and(|a| !a.is_empty())
}

pub fn focus_pane_toward(from_pane: &str, target_pane: &str) -> io::Result<()> {
    if from_pane == target_pane {
        return Ok(());
    }
    for dir in ["up", "down", "left", "right"] {
        if neighbor_pane(from_pane, dir)?.as_deref() == Some(target_pane) {
            return focus_neighbor(from_pane, dir);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no neighbor path from {from_pane} to {target_pane}"),
    ))
}

pub fn focus_neighbor(from_pane: &str, direction: &str) -> io::Result<()> {
    let value = run_herdr_json(&["pane", "focus", "--pane", from_pane, "--direction", direction])?;
    if let Some(msg) = cli_error(&value) {
        return Err(io::Error::other(msg));
    }
    Ok(())
}

pub fn send_text(pane_id: &str, text: &str) -> io::Result<()> {
    let value = run_herdr_json(&["pane", "send-text", pane_id, text])?;
    if let Some(msg) = cli_error(&value) {
        return Err(io::Error::other(msg));
    }
    Ok(())
}

pub fn send_keys(pane_id: &str, keys: &[&str]) -> io::Result<()> {
    let mut args = vec!["pane", "send-keys", pane_id];
    args.extend(keys.iter().copied());
    let value = run_herdr_json(&args)?;
    if let Some(msg) = cli_error(&value) {
        return Err(io::Error::other(msg));
    }
    Ok(())
}

pub fn agent_prompt(target: &str, text: &str) -> Result<(), String> {
    let value = run_herdr_json(&["agent", "prompt", target, text])
        .map_err(|e| e.to_string())?;
    if let Some(msg) = cli_error(&value) {
        return Err(msg);
    }
    Ok(())
}

pub fn viewport_rows(pane_id: &str) -> io::Result<Option<i64>> {
    let value = run_herdr_json(&["pane", "get", pane_id])?;
    Ok(value
        .pointer("/result/pane/scroll/viewport_rows")
        .and_then(|v| v.as_i64()))
}

pub fn resize_pane(pane_id: &str, direction: &str, amount: f64) -> io::Result<()> {
    let amount_str = amount.to_string();
    let value = run_herdr_json(&[
        "pane",
        "resize",
        "--pane",
        pane_id,
        "--direction",
        direction,
        "--amount",
        &amount_str,
    ])?;
    if let Some(msg) = cli_error(&value) {
        return Err(io::Error::other(msg));
    }
    Ok(())
}

pub fn neighbor_pane(from: &str, direction: &str) -> io::Result<Option<String>> {
    let value = run_herdr_json(&["pane", "neighbor", "--pane", from, "--direction", direction])?;
    if cli_error(&value).is_some() {
        return Ok(None);
    }
    Ok(value
        .get("result")
        .and_then(|r| r.get("neighbor"))
        .and_then(|n| n.get("neighbor_pane_id"))
        .and_then(|v| v.as_str())
        .map(str::to_string))
}
