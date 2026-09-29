use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const MAP_FILE: &str = "target_bars.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetBarState {
    /// Target Pane id -> Input Bar pane id
    pub target_to_bar: HashMap<String, String>,
    /// Input Bar pane id -> Target Pane id
    pub bar_to_target: HashMap<String, String>,
}

impl TargetBarState {
    pub fn load(state_dir: &Path) -> io::Result<Self> {
        let path = state_dir.join(MAP_FILE);
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = fs::read_to_string(path)?;
        serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, state_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(state_dir)?;
        let path = state_dir.join(MAP_FILE);
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(path, data)
    }

    pub fn register(&mut self, target_pane: &str, bar_pane: &str) {
        if let Some(old_bar) = self.target_to_bar.get(target_pane) {
            self.bar_to_target.remove(old_bar);
        }
        if let Some(old_target) = self.bar_to_target.get(bar_pane) {
            self.target_to_bar.remove(old_target);
        }
        self.target_to_bar
            .insert(target_pane.to_string(), bar_pane.to_string());
        self.bar_to_target
            .insert(bar_pane.to_string(), target_pane.to_string());
    }

    pub fn unregister_bar(&mut self, bar_pane: &str) {
        if let Some(target) = self.bar_to_target.remove(bar_pane) {
            self.target_to_bar.remove(&target);
        }
    }

    pub fn bar_for_target(&self, target: &str) -> Option<&str> {
        self.target_to_bar.get(target).map(String::as_str)
    }

    pub fn is_bar_pane(&self, pane_id: &str) -> bool {
        self.bar_to_target.contains_key(pane_id)
    }

    pub fn prune_dead<F>(&mut self, mut exists: F) -> io::Result<()>
    where
        F: FnMut(&str) -> io::Result<bool>,
    {
        let bars: Vec<String> = self.bar_to_target.keys().cloned().collect();
        for bar in bars {
            if !exists(&bar)? {
                self.unregister_bar(&bar);
            }
        }
        Ok(())
    }
}

pub fn state_dir_from_env() -> io::Result<PathBuf> {
    std::env::var("HERDR_PLUGIN_STATE_DIR")
        .map(PathBuf::from)
        .map_err(|_| io::Error::new(io::ErrorKind::NotFound, "HERDR_PLUGIN_STATE_DIR unset"))
}
