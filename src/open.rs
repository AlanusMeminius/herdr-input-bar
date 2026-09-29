use crate::herdr;
use crate::state::{state_dir_from_env, TargetBarState};
use std::env;
use std::io;

const TARGET_ENV: &str = "INPUT_BAR_TARGET_PANE";
const BAR_ROWS: i64 = 3;

pub fn run() -> io::Result<()> {
    let target_pane = focused_pane_id()?;
    let state_dir = state_dir_from_env()?;
    let mut state = TargetBarState::load(&state_dir)?;
    state.prune_dead(|id| herdr::pane_exists(id))?;
    state.save(&state_dir)?;

    if state.is_bar_pane(&target_pane) || is_labeled_input_bar(&target_pane)? {
        return Ok(());
    }

    if let Some(bar) = state.bar_for_target(&target_pane).map(str::to_string) {
        if herdr::pane_exists(&bar)? {
            focus_bar_pane(&target_pane, &bar)?;
            return Ok(());
        }
        state.unregister_bar(&bar);
        state.save(&state_dir)?;
    }

    let value = herdr::run_herdr_json(&[
        "plugin",
        "pane",
        "open",
        "--plugin",
        &env::var("HERDR_PLUGIN_ID").unwrap_or_else(|_| "alanus.input-bar".into()),
        "--entrypoint",
        "bar",
        "--placement",
        "split",
        "--direction",
        "down",
        "--target-pane",
        &target_pane,
        "--env",
        &format!("{TARGET_ENV}={target_pane}"),
        "--focus",
    ])?;

    if let Some(msg) = herdr::cli_error(&value) {
        return Err(io::Error::other(msg));
    }

    let bar_pane = value
        .get("result")
        .and_then(|r| r.get("plugin_pane"))
        .and_then(|pp| pp.get("pane"))
        .and_then(|p| p.get("pane_id"))
        .and_then(|v| v.as_str())
        .or_else(|| {
            value
                .get("result")
                .and_then(|r| r.get("pane"))
                .and_then(|p| p.get("pane_id"))
                .and_then(|v| v.as_str())
        })
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing bar pane id"))?;

    state.register(&target_pane, bar_pane);
    state.save(&state_dir)?;

    let _ = fit_bar_height(&target_pane, bar_pane);
    Ok(())
}

/// herdr only resizes by split fraction, so converge on the row count in a few steps.
fn fit_bar_height(target_pane: &str, bar_pane: &str) -> io::Result<()> {
    for _ in 0..3 {
        let (Some(target), Some(bar)) = (
            herdr::viewport_rows(target_pane)?,
            herdr::viewport_rows(bar_pane)?,
        ) else {
            return Ok(());
        };
        let diff = bar - BAR_ROWS;
        if diff == 0 || target + bar == 0 {
            return Ok(());
        }
        let direction = if diff > 0 { "down" } else { "up" };
        let amount = diff.abs() as f64 / (target + bar) as f64;
        herdr::resize_pane(bar_pane, direction, amount)?;
    }
    Ok(())
}

fn is_labeled_input_bar(pane_id: &str) -> io::Result<bool> {
    let value = herdr::run_herdr_json(&["pane", "get", pane_id])?;
    if herdr::cli_error(&value).is_some() {
        return Ok(false);
    }
    Ok(value
        .get("result")
        .and_then(|r| r.get("pane"))
        .and_then(|p| p.get("label"))
        .and_then(|v| v.as_str())
        == Some("Input Bar"))
}

fn focus_bar_pane(target_pane: &str, bar_pane: &str) -> io::Result<()> {
    for dir in ["down", "up", "right", "left"] {
        if herdr::neighbor_pane(target_pane, dir)?.as_deref() == Some(bar_pane) {
            return herdr::focus_neighbor(target_pane, dir);
        }
    }
    herdr::focus_neighbor(target_pane, "down")
}

fn focused_pane_id() -> io::Result<String> {
    if let Ok(id) = env::var("HERDR_PANE_ID") {
        if !id.is_empty() {
            return Ok(id);
        }
    }
    if let Ok(json) = env::var("HERDR_PLUGIN_CONTEXT_JSON") {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&json) {
            if let Some(id) = v
                .get("focused_pane_id")
                .or_else(|| v.get("pane_id"))
                .and_then(|x| x.as_str())
            {
                return Ok(id.to_string());
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "no focused pane in plugin context",
    ))
}
