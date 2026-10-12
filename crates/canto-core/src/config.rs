//! Canto settings: where YuE2 lives, where songs go, and which refiner to use.
//!
//! Stored as JSON at `~/.config/canto/config.json`. Every field has a default
//! that matches the Aerie install, so a missing file is a working setup.

use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};

use crate::refine::RefinerKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// The mlx-Yue checkout: holds `.venv`, `models/converted`, `models/vae`.
    pub yue_project: PathBuf,
    /// Where finished songs live, one directory per song.
    pub library_dir: PathBuf,
    /// Weight precision passed to `mlx-yue`: bf16, 8bit or 4bit.
    pub precision: String,
    pub memory_budget_gib: f32,
    pub vae_core_frames: u32,
    /// Which CLI rewrites prompts. Both run on the user's subscription.
    pub refiner: RefinerKind,
    /// Optional model override handed to the refiner CLI (`--model`).
    pub refiner_model: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        let root = PathBuf::from("/Volumes/Swift/local-inference/yue2");
        Self {
            yue_project: root.join("mlx-Yue"),
            library_dir: root.join("outputs"),
            precision: "8bit".into(),
            memory_budget_gib: 16.0,
            vae_core_frames: 128,
            refiner: RefinerKind::Claude,
            refiner_model: None,
        }
    }
}

impl Config {
    pub fn path() -> PathBuf {
        if let Some(p) = std::env::var_os("CANTO_CONFIG") {
            return PathBuf::from(p);
        }
        home().join(".config/canto/config.json")
    }

    /// Load from disk, falling back to defaults when the file is absent.
    pub fn load() -> Result<Self> {
        Self::load_from(&Self::path())
    }

    pub fn load_from(path: &Path) -> Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(text) => serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).with_context(|| format!("reading {}", path.display())),
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }

    pub fn mlx_yue(&self) -> PathBuf {
        self.yue_project.join(".venv/bin/mlx-yue")
    }
}

pub(crate) fn home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/"))
}
