//! The song model: what a user asks for, and what lands in the library.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How much symbolic planning YuE2 does before singing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Melody and chords: the most structure, the default.
    #[default]
    Full,
    /// Melody only.
    Melody,
    /// No symbolic plan.
    Off,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Full => "full",
            Mode::Melody => "melody",
            Mode::Off => "off",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Mode::Full),
            "melody" => Some(Mode::Melody),
            "off" => Some(Mode::Off),
            _ => None,
        }
    }
}

/// Acoustic steps for a quick audition versus a finished song.
pub const DRAFT_STEPS: u32 = 8;
pub const FULL_STEPS: u32 = 32;

/// Everything needed to make one song.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SongRequest {
    pub title: String,
    /// English tags: genre, vocal, instruments, tempo, mood, production.
    pub style: String,
    /// Section-tagged lyrics. Song length follows the lyrics.
    pub lyrics: String,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_steps")]
    pub steps: u32,
}

fn default_steps() -> u32 {
    FULL_STEPS
}

impl SongRequest {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.style.trim().is_empty(), "style cannot be empty");
        anyhow::ensure!(!self.lyrics.trim().is_empty(), "lyrics cannot be empty");
        anyhow::ensure!((1..=128).contains(&self.steps), "steps must be between 1 and 128");
        Ok(())
    }

    pub fn is_draft(&self) -> bool {
        self.steps < FULL_STEPS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SongStatus {
    Complete,
    Failed,
    Cancelled,
    /// A directory with no result yet: still running, or killed mid-run.
    Incomplete,
}

/// Canto's own notes about a song, kept in `canto.json` inside its directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SongMeta {
    pub title: String,
    pub created: u64,
    pub favorite: bool,
    pub steps: Option<u32>,
    /// The idea or instruction the refiner worked from, when one was used.
    pub prompt_source: Option<String>,
    pub status: Option<SongStatus>,
    pub error: Option<String>,
    pub seconds_to_generate: Option<f64>,
    /// What was asked for, so a failed run can still be reused or remixed.
    pub request: Option<SongRequest>,
}

/// A song as the library sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub dir: PathBuf,
    pub title: String,
    pub style: String,
    pub lyrics: String,
    pub mode: Mode,
    pub seed: u64,
    pub steps: u32,
    pub created: u64,
    pub favorite: bool,
    pub status: SongStatus,
    pub error: Option<String>,
    pub duration_secs: Option<f64>,
    pub audio: Option<PathBuf>,
    pub mp3: Option<PathBuf>,
    pub seconds_to_generate: Option<f64>,
    pub prompt_source: Option<String>,
}

impl Song {
    pub fn request(&self) -> SongRequest {
        SongRequest {
            title: self.title.clone(),
            style: self.style.clone(),
            lyrics: self.lyrics.clone(),
            mode: self.mode,
            seed: self.seed,
            steps: self.steps,
        }
    }

    pub fn is_draft(&self) -> bool {
        self.steps < FULL_STEPS
    }
}
